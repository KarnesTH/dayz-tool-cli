use inquire::MultiSelect;
use serde_json::Value;
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
};

use crate::{
    utils::{add_mods_to_profile, get_config_path, get_profile},
    ModError, Profile, ThreadPool,
};

/// Installs selected mods from the workshop directory to the profile's work directory.
///
/// This function prompts the user to select mods from the workshop directory and then
/// copies the selected mods to the profile's work directory. It also updates the profile
/// with the installed mods and returns a startup parameter string for launching the game
/// with the installed mods.
pub fn install_mods(pool: &ThreadPool, profile: Profile) -> Result<String, ModError> {
    let workshop_path = profile.workshop_path.clone();
    let path = Path::new(&workshop_path);

    let mut mods: Vec<String> = vec![];
    let mut mods_paths: Vec<String> = vec![];
    let mut mods_to_install: Vec<String> = vec![];

    for entry in path.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let path_str = path.to_str().unwrap();
        let folder_name = path.file_name().unwrap().to_str().unwrap();

        mods.push(folder_name.to_string());
        mods_paths.push(path_str.to_string());
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
                // TODO: Add copy types.xml to mpmissions folder
            }

            add_mods_to_profile(mods_to_install.clone()).unwrap();
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
