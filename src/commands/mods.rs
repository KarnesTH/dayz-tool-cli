use inquire::MultiSelect;

use log::info;

use std::path::{Path, PathBuf};

use crate::{
    utils::{
        add_mods_to_profile, analyze_types_folder, copy_dir, copy_keys, find_keys_folder,
        find_types_folder, get_installed_mod_list, get_map_name, parse_startup_parameter,
        save_extracted_data,
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

pub fn list_installed_mods(profile: Profile) -> Result<(), ModError> {
    let installed_mods = get_installed_mod_list(profile.clone()).unwrap();
    let installed_mods_names: Vec<String> = installed_mods
        .into_iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for mod_name in installed_mods_names {
        info!("{}", mod_name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

        match get_installed_mod_list(profile) {
            Ok(mods) => {
                assert_eq!(mods, installed_mods);
            }
            Err(e) => panic!("Error retrieving installed mods: {:?}", e),
        }
    }
}
