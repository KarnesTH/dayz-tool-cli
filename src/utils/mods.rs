use crate::{
    utils::{get_config_path, get_profile},
    Event, EventsWrapper, ModChecksum, ModError, Profile, ProgressBar, SpawnableType,
    SpawnableTypesWrapper, ThreadPool, Type, TypesWrapper, THEME,
};
use log::{debug, error, info};
use quick_xml::se::to_string;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use serde_xml_rs::from_str;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{copy, create_dir_all, read_dir, read_to_string, remove_file, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use walkdir::WalkDir;

/// Recursively copies the contents of one directory to another with optimized handling of large files.
///
/// This function takes a source directory and a target directory as input and
/// recursively copies all files and subdirectories from the source to the target.
/// For files larger than 100MB, it uses a chunked copying approach to optimize memory usage
/// and provide progress tracking.
pub fn copy_dir(source_dir: &Path, target_dir: &Path) -> Result<(), ModError> {
    match create_dir_all(target_dir) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create directory {}: {}", target_dir.display(), e);
            return Err(ModError::CreateDirError);
        }
    }

    const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;
    const CHUNK_SIZE: usize = 8 * 1024 * 1024;

    for entry in source_dir.read_dir().map_err(|e| {
        error!("Failed to read directory {}: {}", source_dir.display(), e);
        ModError::CopyFileError
    })? {
        let entry = entry.map_err(|e| {
            error!("Failed to read directory entry: {}", e);
            ModError::CopyFileError
        })?;

        let source_path = entry.path();
        let target_path = target_dir.join(source_path.strip_prefix(source_dir).unwrap());

        let file_type = entry.file_type().map_err(|e| {
            error!(
                "Failed to get file type for {}: {}",
                source_path.display(),
                e
            );
            ModError::CopyFileError
        })?;

        if file_type.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            let metadata = entry.metadata().map_err(|e| {
                error!(
                    "Failed to get metadata for {}: {}",
                    source_path.display(),
                    e
                );
                ModError::CopyFileError
            })?;

            let file_size = metadata.len();

            if file_size > LARGE_FILE_THRESHOLD {
                debug!(
                    "Copying large file ({} MB): {}",
                    file_size / (1024 * 1024),
                    source_path.display()
                );
                copy_large_file(&source_path, &target_path, CHUNK_SIZE).map_err(|e| {
                    error!("Failed to copy large file {}: {}", source_path.display(), e);
                    ModError::CopyFileError
                })?;
            } else {
                copy(&source_path, &target_path).map_err(|e| {
                    error!("Failed to copy file {}: {}", source_path.display(), e);
                    ModError::CopyFileError
                })?;
            }
        }
    }

    Ok(())
}

/// Copies a large file in chunks with progress tracking.
///
/// This function implements a memory-efficient copying mechanism for large files
/// by reading and writing the file in chunks rather than loading it entirely into memory.
/// It also provides progress updates through logging.
fn copy_large_file(source: &Path, target: &Path, chunk_size: usize) -> std::io::Result<()> {
    let mut source_file = File::open(source)?;
    let mut target_file = File::create(target)?;
    let file_size = source_file.metadata()?.len();
    let mut buffer = vec![0; chunk_size];

    let progress = ProgressBar::new(
        file_size,
        30,
        &format!(
            "Copying {}",
            source.file_name().unwrap_or_default().to_string_lossy()
        ),
        Arc::new(THEME.clone()),
    );

    while let Ok(bytes_read) = source_file.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }

        target_file.write_all(&buffer[..bytes_read])?;
        progress.inc(bytes_read as u64);
    }

    target_file.flush()?;
    Ok(())
}

/// Calculates checksums for all files in a mod directory using parallel processing.
///
/// This function walks through the mod directory and calculates checksums for all files,
/// using a thread pool for parallel processing. It handles files differently based on their size:
/// - Files > 1MB: Full SHA256 hash calculation
/// - Files ≤ 1MB: Only size comparison ("small_file" marker)
fn calculate_mod_checksums(
    mod_path: &Path,
    pool: &ThreadPool,
) -> Result<Vec<ModChecksum>, std::io::Error> {
    let checksums_mutex = Arc::new(Mutex::new(Vec::new()));
    let error_mutex = Arc::new(Mutex::new(None));

    let files: Vec<_> = WalkDir::new(mod_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_ignored_file(e))
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    debug!("Found {} files to check", files.len());

    for entry in files {
        let checksums = Arc::clone(&checksums_mutex);
        let errors = Arc::clone(&error_mutex);
        let path = entry.path().to_path_buf();
        let mod_path = mod_path.to_path_buf();

        pool.execute(move || {
            let result: Result<(), std::io::Error> = (|| {
                let metadata = entry.metadata()?;
                let size = metadata.len();
                let hash = if size > 1024 * 1024 {
                    calculate_file_hash(&path)?
                } else {
                    "small_file".to_string()
                };

                let rel_path = path
                    .strip_prefix(&mod_path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                    .to_path_buf();

                let mut checksums_guard = checksums.lock().unwrap();
                checksums_guard.push(ModChecksum {
                    path: rel_path,
                    size,
                    hash,
                });
                Ok(())
            })();

            if let Err(e) = result {
                let mut error_guard = errors.lock().unwrap();
                *error_guard = Some(e);
            }
        });
    }

    pool.wait();

    let error_guard = error_mutex.lock().unwrap();
    if let Some(e) = &*error_guard {
        return Err(std::io::Error::new(e.kind(), e.to_string()));
    }
    drop(error_guard);

    let checksums_guard = checksums_mutex.lock().unwrap();
    let result = checksums_guard.clone();
    Ok(result)
}

/// Calculates the SHA256 hash of a file.
///
/// Reads the file in 1MB chunks and calculates a SHA256 hash of its contents.
/// This method is memory-efficient as it processes the file in chunks rather
/// than loading it entirely into memory.
fn calculate_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Determines if a file should be ignored during mod comparison.
///
/// Filters out system files and hidden files that should not be included
/// in mod comparison calculations. Currently ignores:
/// - Hidden files (starting with '.')
/// - Windows system files ('desktop.ini', 'thumbs.db')
fn is_ignored_file(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') || s == "desktop.ini" || s == "thumbs.db")
        .unwrap_or(false)
}

/// Compares mod versions between workshop and workdir by checking file checksums.
///
/// This function performs a detailed comparison of mod files between the workshop and workdir
/// directories using parallel checksum calculation. It checks for:
/// - Different number of files
/// - Missing files
/// - File size differences
/// - Content differences (via hash comparison)
pub fn compare_mod_versions(
    workshop_path: &Path,
    workdir_path: &Path,
    pool: &ThreadPool,
) -> Result<bool, std::io::Error> {
    debug!("Calculating checksums for workshop version...");
    let workshop_checksums = calculate_mod_checksums(workshop_path, pool)?;

    debug!("Calculating checksums for installed version...");
    let workdir_checksums = calculate_mod_checksums(workdir_path, pool)?;

    if workshop_checksums.len() != workdir_checksums.len() {
        info!("Different number of files detected");
        return Ok(false);
    }

    let workdir_map: HashMap<_, _> = workdir_checksums
        .into_iter()
        .map(|c| (c.path, (c.size, c.hash)))
        .collect();

    for workshop_check in workshop_checksums {
        if let Some((size, hash)) = workdir_map.get(&workshop_check.path) {
            if *size != workshop_check.size || *hash != workshop_check.hash {
                info!(
                    "File {} has different size or hash",
                    workshop_check.path.display()
                );
                return Ok(false);
            }
        } else {
            info!("Missing file in workdir: {}", workshop_check.path.display());
            return Ok(false);
        }
    }

    Ok(true)
}

/// Searches for a subdirectory named "keys" in the specified mod directory.
///
/// This function searches the given directory for a subdirectory named "keys"
/// (case-insensitive). If such a directory is found, the path to this directory
/// is returned. Otherwise, `None` is returned.
pub fn find_keys_folder(mod_path: &Path) -> Option<PathBuf> {
    for entry in mod_path.read_dir().unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            let folder_name = entry.file_name().to_string_lossy().to_lowercase();
            if folder_name == "keys" {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Copies all ".bikey" files from the source directory to the target directory.
///
/// This function iterates through the entries in the specified source directory,
/// and copies all files with the ".bikey" extension to the target directory. If
/// any file copy operation fails, it returns a `ModError::CopyFileError`.
pub fn copy_keys(source_dir: &Path, target_dir: &Path) -> Result<(), ModError> {
    for entry in source_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        if source_path.extension().and_then(|s| s.to_str()) == Some("bikey") {
            let target_path = target_dir.join(source_path.file_name().unwrap());
            if !target_path.exists() {
                match copy(&source_path, &target_path) {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(ModError::CopyFileError);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Generates a startup parameter string for the installed mods.
///
/// This function retrieves the configuration path and profile, then generates a list
/// of installed mods. It formats these mods into a startup parameter string suitable
pub fn parse_startup_parameter() -> Result<String, ModError> {
    let config = get_config_path();
    let updatet_profile = get_profile(&config).unwrap();

    let installed_mods = get_installed_mod_list(updatet_profile).unwrap();
    let installed_mods_strings: Vec<String> = installed_mods
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let startup_parameter = format!("\"-mod={};\"", installed_mods_strings.join(";"));
    Ok(startup_parameter)
}

/// Recursively searches for a folder containing a file with "types" in its name.
///
/// This function starts at the given path and traverses directories recursively
/// to find a folder that contains a file with "types" in its name. If such a folder
/// is found, the path to the folder is returned. If no such folder is found, `None`
/// is returned.
pub fn find_types_folder(path: &Path) -> Option<PathBuf> {
    fn visit_dirs(dir: &Path) -> Option<PathBuf> {
        if dir.is_dir() {
            for entry in read_dir(dir).ok()? {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(result) = visit_dirs(&path) {
                        return Some(result);
                    }
                } else if path.is_file()
                    && path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .contains("types")
                {
                    return Some(path.parent().unwrap().to_path_buf());
                }
            }
        }
        None
    }

    visit_dirs(path)
}

/// Extracts XML data elements from a given file.
///
/// This function reads the content of the specified XML file and extracts elements
/// of type `<type>` or `<event>`. It handles cases where the root tag might be missing
/// and adds it if necessary. The function returns a vector of strings, each containing
/// a complete XML element.
fn extract_xml_data(file_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let content = if !content.contains("<types>")
        && !content.contains("<spawnabletypes>")
        && !content.contains("<events>")
    {
        let root_tag = if content.contains("<type") {
            "types"
        } else if content.contains("<event") {
            "events"
        } else {
            "spawnabletypes"
        };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<{}>\n{}\n</{}>",
            root_tag, content, root_tag
        )
    } else {
        content
    };

    let mut data: Vec<String> = Vec::new();
    let mut current_element = String::new();
    let mut in_element_tag = false;

    for line in content.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with("<?xml")
            || trimmed_line.starts_with("<types")
            || trimmed_line.starts_with("<spawnabletypes")
            || trimmed_line.starts_with("<events")
            || trimmed_line.starts_with("</types")
            || trimmed_line.starts_with("</spawnabletypes")
            || trimmed_line.starts_with("</events")
        {
            continue;
        }

        if trimmed_line.starts_with("<type") || trimmed_line.starts_with("<event") {
            in_element_tag = true;
            current_element.clear();
            current_element.push_str(trimmed_line);
            current_element.push('\n');
        } else if (trimmed_line.starts_with("</type") || trimmed_line.starts_with("</event"))
            && in_element_tag
        {
            in_element_tag = false;
            current_element.push_str(trimmed_line);
            current_element.push('\n');
            data.push(current_element.clone());
        } else if in_element_tag && !trimmed_line.starts_with("<!--") {
            current_element.push_str(trimmed_line);
            current_element.push('\n');
        }
    }

    Ok(data)
}

/// Extracts `Type` elements from a given XML file.
///
/// This function reads the content of the specified XML file and extracts elements
/// of type `<type>`. It uses the `extract_xml_data` function to get the raw XML strings
/// and then parses each string into a `Type` struct. The function returns a vector of
/// `Type` structs.
fn extract_types(file_path: &Path) -> Result<Vec<Type>, Box<dyn std::error::Error>> {
    let xml_strings = extract_xml_data(file_path)?;

    let mut types: Vec<Type> = Vec::new();
    for xml_string in xml_strings {
        if xml_string.starts_with("<type") {
            match from_str::<Type>(&xml_string) {
                Ok(type_data) => types.push(type_data),
                Err(e) => return Err(format!("Fehler beim Parsen von Type: {}", e).into()),
            }
        }
    }

    Ok(types)
}

/// Extracts `SpawnableType` elements from a given XML file.
///
/// This function reads the content of the specified XML file and extracts elements
/// of type `<type>`. It uses the `extract_xml_data` function to get the raw XML strings
/// and then parses each string into a `SpawnableType` struct. The function returns a vector
/// of `SpawnableType` structs.
fn extract_cfgspawnabletypes(
    file_path: &Path,
) -> Result<Vec<SpawnableType>, Box<dyn std::error::Error>> {
    let xml_strings = extract_xml_data(file_path)?;

    let mut spawnable_types: Vec<SpawnableType> = Vec::new();
    for xml_string in xml_strings {
        if xml_string.starts_with("<type") {
            match from_str::<SpawnableType>(&xml_string) {
                Ok(type_data) => spawnable_types.push(type_data),
                Err(e) => return Err(format!("Fehler beim Parsen von SpawnableType: {}", e).into()),
            }
        }
    }

    Ok(spawnable_types)
}

/// Extracts `Event` elements from a given XML file.
///
/// This function reads the content of the specified XML file and extracts elements
/// of type `<event>`. It uses the `extract_xml_data` function to get the raw XML strings
/// and then parses each string into an `Event` struct. The function returns a vector
/// of `Event` structs.
fn extract_events(file_path: &Path) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let xml_strings = extract_xml_data(file_path)?;

    let mut events: Vec<Event> = Vec::new();
    for xml_string in xml_strings {
        if xml_string.starts_with("<event") {
            match from_str::<Event>(&xml_string) {
                Ok(event_data) => events.push(event_data),
                Err(e) => return Err(format!("Fehler beim Parsen von Event: {}", e).into()),
            }
        }
    }

    Ok(events)
}

/// A type alias for the result of an analysis operation.
///
/// This type alias represents the result of an analysis operation that may return
/// vectors of `Type`, `SpawnableType`, and `Event` structs. Each of these vectors
/// is optional, meaning that the analysis may return any combination of these types
/// or none at all. If an error occurs during the analysis, it will return an error
/// boxed as `Box<dyn std::error::Error>`.
type AnalyzeResult = Result<
    (
        Option<Vec<Type>>,
        Option<Vec<SpawnableType>>,
        Option<Vec<Event>>,
    ),
    Box<dyn std::error::Error>,
>;

/// Analyzes a folder for XML files containing `Type`, `SpawnableType`, and `Event` elements.
///
/// This function searches the specified folder for XML files that contain `Type`, `SpawnableType`,
/// and `Event` elements. It processes each file accordingly and extracts the relevant data into
/// vectors. The function returns a tuple containing optional vectors of `Type`, `SpawnableType`,
/// and `Event` structs.
pub fn analyze_types_folder(folder_path: &Path) -> AnalyzeResult {
    let mut types = Vec::new();
    let mut spawnable_types = Vec::new();
    let mut events = Vec::new();

    debug!("Scanning directory: {}", folder_path.display());

    for entry in read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            debug!("File found: {}", file_name);

            if file_name.contains("types") && !file_name.contains("spawnable") {
                debug!("Processing types file");
                types = extract_types(&path)?;
                debug!("Found Types: {}", types.len());
            } else if file_name.contains("spawnabletypes") {
                debug!("Processing spawnabletypes file");
                spawnable_types = extract_cfgspawnabletypes(&path)?;
                debug!("Found SpawnableTypes: {}", spawnable_types.len());
            } else if file_name.contains("events") {
                debug!("Processing events file");
                events = extract_events(&path)?;
                debug!("Found Events: {}", events.len());
            }
        }
    }

    Ok((Some(types), Some(spawnable_types), Some(events)))
}

/// Retrieves the map name from the `serverDZ.cfg` file in the specified working directory.
///
/// This function searches for the `serverDZ.cfg` file in the given working directory and
/// extracts the map name using a regular expression. The map name is expected to be in the
/// format `word.word` (e.g., `chernarusplus.chernarus`). If the file is not found or the
/// map name cannot be extracted, an error is returned.
pub fn get_map_name(workdir: &str) -> Result<String, ModError> {
    let cfg_path = Path::new(workdir).join("serverDZ.cfg");

    if !cfg_path.is_file() {
        return Err(ModError::NotFound);
    }

    let mut file = File::open(cfg_path).map_err(|_| ModError::NotFound)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|_| ModError::NotFound)?;

    let re = Regex::new(r"(\w+\.\w+)").unwrap();

    re.captures(&contents)
        .map(|cap| cap[1].to_string())
        .ok_or(ModError::NotFound)
}

/// Writes serialized data to an XML file with proper formatting.
///
/// This function takes a reference to serializable data and a file path, serializes the data
/// to an XML string, and writes it to the specified file. The XML content is formatted based
/// on the root element (`<types>`, `<spawnabletypes>`, or `<events>`). The function also writes
/// the XML declaration at the beginning of the file.
fn write_to_file<T>(data: &T, file_path: &Path) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize + std::fmt::Debug,
{
    let mut file = File::create(file_path)?;
    file.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n")?;

    let xml = to_string(&data)?;

    let formatted = if xml.contains("<types>") {
        format_types(&xml)
    } else if xml.contains("<spawnabletypes>") {
        format_spawnabletypes(&xml)
    } else {
        format_events(&xml)
    };

    file.write_all(formatted.as_bytes())?;
    Ok(())
}

/// Formats the XML string for `Type` elements with proper indentation and line breaks.
///
/// This function takes an XML string containing `<types>` and `<type>` elements and formats it
/// with appropriate indentation and line breaks to improve readability. It ensures that each
/// element and its sub-elements are properly indented and separated by new lines.
fn format_types(xml: &str) -> String {
    xml.replace("<types>", "<types>\n")
        .replace("<type ", "\t<type ")
        .replace("><nominal>", ">\n\t\t<nominal>")
        .replace("</nominal><", "</nominal>\n\t\t<")
        .replace("</lifetime><", "</lifetime>\n\t\t<")
        .replace("</restock><", "</restock>\n\t\t<")
        .replace("</min><", "</min>\n\t\t<")
        .replace("</quantmin><", "</quantmin>\n\t\t<")
        .replace("</quantmax><", "</quantmax>\n\t\t<")
        .replace("</cost><", "</cost>\n\t\t<")
        .replace("/><flags", "/>\n\t\t<flags")
        .replace("/><category", "/>\n\t\t<category")
        .replace("/><usage", "/>\n\t\t<usage")
        .replace("/><tag", "/>\n\t\t<tag")
        .replace("/><value", "/>\n\t\t<value")
        .replace("</type>", "\n\t</type>\n")
        .replace("</types>", "</types>\n")
}

/// Formats the XML string for `SpawnableType` elements with proper indentation and line breaks.
///
/// This function takes an XML string containing `<spawnabletypes>` and `<type>` elements and formats it
/// with appropriate indentation and line breaks to improve readability. It ensures that each
/// element and its sub-elements are properly indented and separated by new lines.
fn format_spawnabletypes(xml: &str) -> String {
    xml.replace("<spawnabletypes>", "<spawnabletypes>\n")
        .replace("<type ", "\t<type ")
        .replace("><attachments", ">\n\t\t<attachments")
        .replace("/></attachments>", "/>\n\t\t</attachments>")
        .replace("<item", "\n\t\t\t<item")
        .replace("</type>", "\n\t</type>\n")
        .replace("</spawnabletypes>", "</spawnabletypes>\n")
}

/// Formats the XML string for `Event` elements with proper indentation and line breaks.
///
/// This function takes an XML string containing `<events>` and `<event>` elements and formats it
/// with appropriate indentation and line breaks to improve readability. It ensures that each
/// element and its sub-elements are properly indented and separated by new lines.
fn format_events(xml: &str) -> String {
    xml.replace("<events>", "<events>\n")
        .replace("<event ", "\t<event ")
        .replace("><nominal>", ">\n\t\t<nominal>")
        .replace("</nominal><", "</nominal>\n\t\t<")
        .replace("</lifetime><", "</lifetime>\n\t\t<")
        .replace("</restock><", "</restock>\n\t\t<")
        .replace("</min><", "</min>\n\t\t<")
        .replace("</max><", "</max>\n\t\t<")
        .replace("</saferadius><", "</saferadius>\n\t\t<")
        .replace("</distanceraduis><", "</distanceraduis>\n\t\t<")
        .replace("</cleanupradius><", "</cleanupradius>\n\t\t<")
        .replace("/><flags", "/>\n\t\t<flags")
        .replace("/><position", "/>\n\t\t<position")
        .replace("</position><", "</position>\n\t\t<")
        .replace("</limit><", "</limit>\n\t\t<")
        .replace("</active><", "</active>\n\t\t<")
        .replace("</children>", "\n\t\t</children>")
        .replace("><child", ">\n\t\t\t<child")
        .replace("/><child", "/>\n\t\t\t<child")
        .replace("</event>", "\n\t</event>\n")
        .replace("</events>", "</events>\n")
}

/// Saves extracted data (`Type`, `SpawnableType`, and `Event` elements) to XML files.
///
/// This function takes the extracted data and saves it to XML files in a specified directory
/// structure. The files are named based on the provided `mod_short_name` and are saved in a
/// subdirectory under the specified `workdir` and `map_name`. The function creates the necessary
/// directories if they do not exist.
pub fn save_extracted_data(
    workdir: &str,
    mod_short_name: &str,
    map_name: &str,
    types: Vec<Type>,
    spawnable_types: Vec<SpawnableType>,
    events: Vec<Event>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = Path::new(workdir)
        .join("mpmissions")
        .join(map_name)
        .join(format!("{}_ce", mod_short_name));
    create_dir_all(&base_path)?;

    if !types.is_empty() {
        let types_wrapper = TypesWrapper { types };
        let types_file_path = base_path.join(format!("{}_types.xml", mod_short_name));
        write_to_file(&types_wrapper, &types_file_path)?;
    }

    if !spawnable_types.is_empty() {
        let spawnable_types_wrapper = SpawnableTypesWrapper { spawnable_types };
        let spawnable_types_file_path =
            base_path.join(format!("{}_cfgspawnabletypes.xml", mod_short_name));
        write_to_file(&spawnable_types_wrapper, &spawnable_types_file_path)?;
    }

    if !events.is_empty() {
        let events_wrapper = EventsWrapper { events };
        let events_file_path = base_path.join(format!("{}_events.xml", mod_short_name));
        write_to_file(&events_wrapper, &events_file_path)?;
    }

    Ok(())
}

/// Retrieves the list of installed mods from the given profile.
///
/// This function takes a `Profile` as input and returns a list of installed mods
/// associated with that profile. If the operation is successful, it returns a `Vec<Value>`
/// containing the installed mods. If an error occurs, a `ModError` is returned.
pub fn get_installed_mod_list(profile: Profile) -> Result<Vec<Value>, ModError> {
    let installed_mods = profile.installed_mods;

    Ok(installed_mods)
}

/// Updates the cfgeconomycore.xml file by adding CE (Central Economy) entries for a mod.
///
/// This function adds XML entries for types, spawnable types, and events files that exist
/// for the given mod. The entries are added just before the closing </economycore> tag.
pub fn update_cfgeconomy(
    workdir: &str,
    mod_short_name: &str,
    types: Vec<Type>,
    spawnable_types: Vec<SpawnableType>,
    events: Vec<Event>,
) -> Result<(), Box<dyn std::error::Error>> {
    if types.is_empty() && spawnable_types.is_empty() && events.is_empty() {
        return Ok(());
    }

    let file_path = Path::new(workdir)
        .join("mpmissions")
        .join(get_map_name(workdir)?)
        .join("cfgeconomycore.xml");

    let content = read_to_string(&file_path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    let end_idx = lines
        .iter()
        .position(|line| line.trim() == "</economycore>")
        .ok_or("Could not find closing economycore tag")?;

    let mut new_content = vec![
        format!("\t<!-- {} -->", mod_short_name),
        format!("\t<ce folder=\"{}_ce\">", mod_short_name),
    ];

    if !types.is_empty() {
        new_content.push(format!(
            "\t\t<file name=\"{}_types.xml\" type=\"types\" />",
            mod_short_name
        ));
    }

    if !spawnable_types.is_empty() {
        new_content.push(format!(
            "\t\t<file name=\"{}_cfgspawnabletypes.xml\" type=\"spawnabletypes\" />",
            mod_short_name
        ));
    }
    if !events.is_empty() {
        new_content.push(format!(
            "\t\t<file name=\"{}_events.xml\" type=\"events\" />",
            mod_short_name
        ));
    }

    new_content.push("\t</ce>".to_string());

    lines.splice(end_idx..end_idx, new_content);

    std::fs::write(&file_path, lines.join("\n"))?;

    Ok(())
}

/// Removes bikey files associated with a mod from the server's keys directory.
///
/// This function searches for bikey files in the mod's keys folder and removes their
/// corresponding files from the server's workdir/keys directory. It performs the following steps:
/// 1. Verifies the existence of the workdir keys directory
/// 2. Locates the mod's keys folder
/// 3. Identifies and removes matching bikey files
pub fn remove_keys_for_mod(workdir: &str, mod_path: &Path) -> Result<(), ModError> {
    let workdir_keys = Path::new(workdir).join("keys");
    if !workdir_keys.exists() {
        return Err(ModError::PathError);
    }

    if let Some(mod_keys_folder) = find_keys_folder(mod_path) {
        for entry in read_dir(mod_keys_folder).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();

            if source_path.is_file() && source_path.extension().map_or(false, |ext| ext == "bikey")
            {
                if let Some(key_name) = source_path.file_name() {
                    let target_path = workdir_keys.join(key_name);
                    if target_path.exists() {
                        info!("Removing bikey: {}", key_name.to_string_lossy());
                        if let Err(e) = remove_file(&target_path) {
                            error!(
                                "Failed to remove bikey {}: {}",
                                key_name.to_string_lossy(),
                                e
                            );
                            return Err(ModError::RemoveFileError);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Removes Central Economy (CE) entries for a specific mod from cfgeconomycore.xml.
///
/// This function modifies the cfgeconomycore.xml file by removing mod-specific CE entries.
/// It looks for and removes entire CE blocks that match the following pattern:
/// ```xml
/// <!-- mod_name -->
/// <ce folder="mod_name_ce">
///     ... (various CE entries)
/// </ce>
/// ```
pub fn remove_ce_entries(workdir: &str, map_name: &str, mod_short: &str) -> Result<(), ModError> {
    let config_path = Path::new(workdir)
        .join("mpmissions")
        .join(map_name)
        .join("cfgeconomycore.xml");

    if !config_path.exists() {
        return Err(ModError::NotFound);
    }

    let content = std::fs::read_to_string(&config_path).map_err(|_| ModError::ReadError)?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut skip_lines = false;

    for line in lines {
        if line.contains(&format!("<!-- {} -->", mod_short))
            || line.contains(&format!(r#"<ce folder="{}_ce">"#, mod_short))
        {
            skip_lines = true;
            continue;
        }

        if skip_lines && line.trim() == "</ce>" {
            skip_lines = false;
            continue;
        }

        if !skip_lines {
            new_lines.push(line.to_string());
        }
    }

    std::fs::write(&config_path, new_lines.join("\n")).map_err(|_| ModError::WriteError)?;

    debug!("Successfully removed CE entries for {}", mod_short);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_copy_dir() {
        let temp_dir = std::env::temp_dir();
        let source_dir = temp_dir.join("source_dir");
        let target_dir = temp_dir.join("target_dir");

        fs::create_dir_all(&source_dir).unwrap();
        let mut file1 = File::create(source_dir.join("file1.txt")).unwrap();
        writeln!(file1, "This is a test file.").unwrap();

        let sub_dir = source_dir.join("sub_dir");
        fs::create_dir_all(&sub_dir).unwrap();
        let mut file2 = File::create(sub_dir.join("file2.txt")).unwrap();
        writeln!(file2, "This is another test file.").unwrap();

        match copy_dir(&source_dir, &target_dir) {
            Ok(_) => {
                assert!(target_dir.exists());
                assert!(target_dir.join("file1.txt").exists());
                assert!(target_dir.join("sub_dir").exists());
                assert!(target_dir.join("sub_dir/file2.txt").exists());
            }
            Err(e) => panic!("Error copying directory: {:?}", e),
        }

        fs::remove_dir_all(&source_dir).unwrap();
        fs::remove_dir_all(&target_dir).unwrap();
    }
}
