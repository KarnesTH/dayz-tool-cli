use inquire::MultiSelect;

use log::{error, info, warn};

use std::path::{Path, PathBuf};

use crate::{
    utils::{
        add_mods_to_profile, analyze_types_folder, copy_dir, copy_keys, find_keys_folder,
        find_types_folder, get_installed_mod_list, get_map_name, parse_startup_parameter,
        save_extracted_data, update_cfgeconomy,
    },
    Mod, ModError, Profile, ThreadPool,
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

    let installed_mods = get_installed_mod_list(profile.clone()).unwrap();
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
                                            types.clone(),
                                            spawnable_types.clone(),
                                            events.clone(),
                                        ) {
                                            error!(
                                                "Error while saving data for {}: {}",
                                                mod_short_name, e
                                            );
                                        }

                                        if let Err(e) = update_cfgeconomy(
                                            &workdir_path,
                                            &mod_short_name,
                                            types,
                                            spawnable_types,
                                            events,
                                        ) {
                                            error!(
                                                "Error updating cfgeconomy.xml for {}: {}",
                                                mod_short_name, e
                                            )
                                        }
                                    }
                                });
                            } else {
                                warn!(
                                    "No types, spawnable_types or events found in mod: {}",
                                    source_path.display()
                                );
                            }
                        }
                        Ok(_) => {
                            error!(
                                "Incomplete data in types directory for mod: {}",
                                source_path.display()
                            );
                        }
                        Err(e) => {
                            error!(
                                "Error parsing types directory for mod {}: {}",
                                source_path.display(),
                                e
                            );
                        }
                    }
                } else {
                    error!(
                        "No types directory found for mod: {}",
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

/// Lists all installed mods for a given DayZ profile.
///
/// This function retrieves a list of all installed mods from the specified profile
/// and displays them in the console. The mods are displayed one per line using
/// the info log level. The function handles the conversion from the internal
/// JSON representation to readable mod names.
///
/// The displayed mod names include their '@' prefix as they appear in the
/// DayZ server directory structure.
pub fn list_installed_mods(profile: Profile) -> Result<(), ModError> {
    let installed_mods = get_installed_mod_list(profile.clone()).unwrap();
    let installed_mods_names: Vec<String> = installed_mods
        .into_iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    if installed_mods_names.is_empty() {
        info!("No mods installed.");
        return Ok(());
    }

    for mod_name in installed_mods_names {
        info!("{}", mod_name);
    }

    Ok(())
}

/// Updates installed mods by replacing their directories and types configurations.
///
/// This function performs the following operations for each installed mod:
/// 1. Removes the existing mod directory from the workdir
/// 2. Copies the latest version from the workshop directory
/// 3. Updates types configurations if changes are detected
///
/// The function uses a thread pool for parallel processing of mods to improve performance.
/// All operations are logged for tracking and debugging purposes.
pub fn update_mods(profile: Profile, pool: &ThreadPool) -> Result<(), ModError> {
    let installed_mods = get_installed_mod_list(profile.clone()).unwrap();
    let workdir_path = profile.workdir_path.clone();
    let workshop_path = profile.workshop_path.clone();

    if installed_mods.is_empty() {
        info!("No mods installed.");
        return Ok(());
    }

    info!("Starting mod updates...");

    for mod_entry in installed_mods {
        let mod_name = mod_entry.as_str().unwrap().to_string();
        let mod_workdir_path = Path::new(&workdir_path).join(&mod_name);
        let mod_workshop_path = Path::new(&workshop_path).join(mod_name.trim_start_matches('@'));

        // Lösche den Mod aus dem Workdir
        if mod_workdir_path.exists() {
            info!("Removing {} from workdir", mod_name);
            if let Err(e) = std::fs::remove_dir_all(&mod_workdir_path) {
                error!("Failed to remove {} from workdir: {}", mod_name, e);
                continue;
            }
        }

        info!("Updating {} from workshop", mod_name);
        pool.execute({
            let mod_name = mod_name.clone();
            let mod_workshop_path = mod_workshop_path.clone();
            let mod_workdir_path = mod_workdir_path.clone();
            let workdir_path = workdir_path.clone();
            move || {
                if let Err(e) = copy_dir(&mod_workshop_path, &mod_workdir_path) {
                    error!("Failed to update {} to workdir: {}", mod_name, e);
                    return;
                }

                if let Some(types_folder_path) = find_types_folder(&mod_workshop_path) {
                    match analyze_types_folder(&types_folder_path) {
                        Ok((Some(types), Some(spawnable_types), Some(events))) => {
                            if !types.is_empty()
                                || !spawnable_types.is_empty()
                                || !events.is_empty()
                            {
                                let mod_short_name = Mod {
                                    name: mod_name.clone(),
                                }
                                .short_name();

                                if let Ok(map_name) = get_map_name(&workdir_path) {
                                    if let Err(e) = save_extracted_data(
                                        &workdir_path,
                                        &mod_short_name,
                                        &map_name,
                                        types.clone(),
                                        spawnable_types.clone(),
                                        events.clone(),
                                    ) {
                                        error!("Error updating types data for {}: {}", mod_name, e);
                                    }

                                    if let Err(e) = update_cfgeconomy(
                                        &workdir_path,
                                        &mod_short_name,
                                        types,
                                        spawnable_types,
                                        events,
                                    ) {
                                        error!(
                                            "Error updating cfgeconomy.xml for {}: {}",
                                            mod_name, e
                                        );
                                    }
                                }
                            }
                        }
                        Ok(_) => {
                            error!("Incomplete types data for mod: {}", mod_name);
                        }
                        Err(e) => {
                            error!("Error analyzing types for mod {}: {}", mod_name, e);
                        }
                    }
                }

                info!("Successfully updated {}", mod_name);
            }
        });
    }

    pool.wait();
    info!("All mod updates completed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_installed_mods() {
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

        let result = list_installed_mods(profile.clone());

        assert!(result.is_ok());
    }
}
