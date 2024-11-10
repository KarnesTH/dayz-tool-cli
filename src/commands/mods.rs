use inquire::MultiSelect;
use quick_xml::se::to_string;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use serde_xml_rs::from_str;
use std::{
    fs::{copy, create_dir_all, read_dir, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    utils::{add_mods_to_profile, get_config_path, get_profile},
    Event, EventsWrapper, Mod, ModError, Profile, SpawnableType, SpawnableTypesWrapper, ThreadPool,
    Type, TypesWrapper,
};

/// Installs selected mods from the workshop directory to the workdir directory.
///
/// This function prompts the user to select filtered, not installed mods from the workshop directory and then
/// copies the selected mods to the workdir directory. It also updates the profile
/// with the installed mods and returns a startup parameter string for launching the game
/// with the installed mods.
pub fn install_mods(pool: &ThreadPool, profile: Profile) -> Result<String, ModError> {
    let workshop_path = profile.workshop_path.clone();
    let path = Path::new(&workshop_path);

    let mut mods: Vec<String> = vec![];
    let mut mods_paths: Vec<String> = vec![];
    let mut mods_to_install: Vec<String> = vec![];

    let installed_mods = installed_mod_list(profile.clone()).unwrap();
    let installed_mods_names: Vec<String> = installed_mods
        .into_iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for entry in path.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let path_str = path.to_str().unwrap();
        let folder_name = path.file_name().unwrap().to_str().unwrap();

        if !installed_mods_names.contains(&folder_name.to_string()) {
            mods.push(folder_name.to_string());
            mods_paths.push(path_str.to_string());
        }
    }

    let ans = MultiSelect::new("Select the mods to intsall:", mods.clone()).prompt();

    match ans {
        Ok(selected_mods) => {
            mods_to_install.clone_from(&selected_mods);
            let selected_mods_paths: Vec<String> = mods_paths
                .into_iter()
                .enumerate()
                .filter_map(|(index, path)| {
                    if selected_mods.contains(&mods[index]) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();

            for selected_mod_path in selected_mods_paths {
                let source_path = PathBuf::from(selected_mod_path);
                let workdir_path = profile.workdir_path.clone();
                let target_path = Path::new(&workdir_path).join(source_path.file_name().unwrap());
                pool.execute({
                    let source_path = source_path.clone();
                    let target_path = target_path.clone();
                    move || {
                        copy_dir(&source_path, &target_path).unwrap();
                    }
                });

                // Copy bikey files in the keys folder
                if let Some(key_source_path) = find_keys_folder(&source_path) {
                    let key_target_path = Path::new(&workdir_path).join("keys");
                    pool.execute({
                        let key_source_path = key_source_path.clone();
                        let key_target_path = key_target_path.clone();
                        move || {
                            copy_keys(&key_source_path, &key_target_path).unwrap();
                        }
                    });
                }

                // Copy types, spawnable_types and events to the mpmissions/<map_name> folder
                if let Some(types_folder_path) = find_types_folder(&source_path) {
                    let map_name = get_map_name(&workdir_path).unwrap();

                    match analyze_types_folder(&types_folder_path) {
                        Ok((Some(types), Some(spawnable_types), Some(events))) => {
                            if !types.is_empty()
                                || !spawnable_types.is_empty()
                                || !events.is_empty()
                            {
                                let mod_short_name = Mod {
                                    name: source_path
                                        .file_name()
                                        .ok_or(ModError::PathError)?
                                        .to_str()
                                        .ok_or(ModError::PathError)?
                                        .to_string(),
                                }
                                .short_name();
                                pool.execute({
                                    let mod_short_name = mod_short_name.clone();
                                    let map_name = map_name.clone();
                                    let types = types.clone();
                                    let spawnable_types = spawnable_types.clone();
                                    let events = events.clone();
                                    move || {
                                        if let Err(e) = save_extracted_data(
                                            &workdir_path,
                                            &mod_short_name,
                                            &map_name,
                                            types,
                                            spawnable_types,
                                            events,
                                        ) {
                                            eprintln!(
                                                "Fehler beim Speichern der Daten für {}: {}",
                                                mod_short_name, e
                                            );
                                        }
                                    }
                                });
                            } else {
                                println!(
                                    "Keine types, spawnable_types oder events gefunden für Mod: {}",
                                    source_path.display()
                                );
                            }
                        }
                        Ok(_) => {
                            println!(
                                "Unvollständige Daten im types-Ordner für Mod: {}",
                                source_path.display()
                            );
                        }
                        Err(e) => {
                            println!(
                                "Fehler beim Analysieren des types-Ordners für Mod {}: {}",
                                source_path.display(),
                                e
                            );
                        }
                    }
                } else {
                    println!(
                        "Kein types-Ordner gefunden für Mod: {}",
                        source_path.display()
                    );
                }
            }

            add_mods_to_profile(mods_to_install.clone()).unwrap();
            pool.wait();
        }
        Err(_) => {
            return Err(ModError::SelectError);
        }
    }

    match parse_startup_parameter() {
        Ok(startup_parameter) => Ok(startup_parameter),
        Err(_) => Err(ModError::ParseError),
    }
}

/// Retrieves the list of installed mods from the given profile.
///
/// This function takes a `Profile` as input and returns a list of installed mods
/// associated with that profile. If the operation is successful, it returns a `Vec<Value>`
/// containing the installed mods. If an error occurs, a `ModError` is returned.
pub fn installed_mod_list(profile: Profile) -> Result<Vec<Value>, ModError> {
    let installed_mods = profile.installed_mods;

    Ok(installed_mods)
}

/// Recursively copies the contents of one directory to another.
///
/// This function takes a source directory and a target directory as input and
/// recursively copies all files and subdirectories from the source to the target.
/// If the target directory does not exist, it will be created. If any error occurs
/// during the creation of the target directory or the copying of files, an appropriate
/// `ModError` will be returned.
fn copy_dir(source_dir: &Path, target_dir: &Path) -> Result<(), ModError> {
    match create_dir_all(target_dir) {
        Ok(dir) => dir,
        Err(_) => {
            return Err(ModError::CreateDirError);
        }
    }

    for entry in source_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let target_path = target_dir.join(source_path.strip_prefix(source_dir).unwrap());

        if entry.file_type().unwrap().is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            match copy(&source_path, &target_path) {
                Ok(_) => {}
                Err(_) => {
                    return Err(ModError::CopyFileError);
                }
            }
        }
    }

    Ok(())
}

/// Searches for a subdirectory named "keys" in the specified mod directory.
///
/// This function searches the given directory for a subdirectory named "keys"
/// (case-insensitive). If such a directory is found, the path to this directory
/// is returned. Otherwise, `None` is returned.
fn find_keys_folder(mod_path: &Path) -> Option<PathBuf> {
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
fn copy_keys(source_dir: &Path, target_dir: &Path) -> Result<(), ModError> {
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
fn parse_startup_parameter() -> Result<String, ModError> {
    let config = get_config_path();
    let updatet_profile = get_profile(&config).unwrap();

    let installed_mods = installed_mod_list(updatet_profile).unwrap();
    let installed_mods_strings: Vec<String> = installed_mods
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let startup_parameter = format!("\"-mod={};\"", installed_mods_strings.join(";"));
    Ok(startup_parameter)
}

fn find_types_folder(path: &Path) -> Option<PathBuf> {
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

type AnalyzeResult = Result<
    (
        Option<Vec<Type>>,
        Option<Vec<SpawnableType>>,
        Option<Vec<Event>>,
    ),
    Box<dyn std::error::Error>,
>;

fn analyze_types_folder(folder_path: &Path) -> AnalyzeResult {
    let mut types = Vec::new();
    let mut spawnable_types = Vec::new();
    let mut events = Vec::new();

    eprintln!("Durchsuche Ordner: {}", folder_path.display());

    for entry in read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            eprintln!("Gefundene Datei: {}", file_name);

            if file_name.contains("types") && !file_name.contains("spawnable") {
                eprintln!("Verarbeite types datei");
                types = extract_types(&path)?;
                eprintln!("Gefundene Types: {}", types.len());
            } else if file_name.contains("spawnabletypes") {
                eprintln!("Verarbeite spawnabletypes datei");
                spawnable_types = extract_cfgspawnabletypes(&path)?;
                eprintln!("Gefundene SpawnableTypes: {}", spawnable_types.len());
            } else if file_name.contains("events") {
                eprintln!("Verarbeite events datei");
                events = extract_events(&path)?;
                eprintln!("Gefundene Events: {}", events.len());
            }
        }
    }

    Ok((Some(types), Some(spawnable_types), Some(events)))
}

fn get_map_name(workdir: &str) -> Result<String, ModError> {
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

fn format_spawnabletypes(xml: &str) -> String {
    xml.replace("<spanwabletypes>", "<spawnabletypes>\n")
        .replace("<type ", "\t<type ")
        .replace("><attachments", ">\n\t\t<attachments")
        .replace("</attachments>", "</attachments>\n\t\t")
        .replace("<item", "\n\t\t\t<item")
        .replace("</type>", "\n\t</type>\n")
        .replace("</spawnabletypes>", "</spawnabletypes>\n")
}

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
        .replace("</position><", "</position>\n\t\t<")
        .replace("</limit><", "</limit>\n\t\t<")
        .replace("</active><", "</active>\n\t\t<")
        .replace("</children><", "</children>\n\t\t<")
        .replace("/><child", "/>\n\t\t\t<child")
        .replace("</event>", "\n\t</event>\n")
        .replace("</events>", "</events>\n")
}

fn save_extracted_data(
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
            base_path.join(format!("{}_spawnabletypes.xml", mod_short_name));
        write_to_file(&spawnable_types_wrapper, &spawnable_types_file_path)?;
    }

    if !events.is_empty() {
        let events_wrapper = EventsWrapper { events };
        let events_file_path = base_path.join(format!("{}_events.xml", mod_short_name));
        write_to_file(&events_wrapper, &events_file_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_installed_mod_list() {
        let mod1 = json!("@mod1");
        let mod2 = json!("@mod2");
        let mod3 = json!("@mod3");
        let installed_mods = vec![mod1.clone(), mod2.clone(), mod3.clone()];
        let profile = Profile {
            name: String::from("DayZTestServer"),
            workdir_path: String::from("/home/karnes/Servers/DayZTestServer"),
            workshop_path: String::from("/home/karnes/Servers/!Workshop"),
            installed_mods: installed_mods.clone(),
            is_active: true,
        };

        match installed_mod_list(profile) {
            Ok(mods) => {
                assert_eq!(mods, installed_mods);
            }
            Err(e) => panic!("Error retrieving installed mods: {:?}", e),
        }
    }

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
