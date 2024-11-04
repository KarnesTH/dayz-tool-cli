use inquire::MultiSelect;
use serde_json::Value;
use std::{
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
};

use crate::{utils::add_mods_to_profile, ModError, Profile};

pub fn install_mods(profile: Profile) -> Result<String, ModError> {
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
                copy_dir(&source_path, &target_path)?;
            }

            add_mods_to_profile(mods_to_install.clone()).unwrap();
        }
        Err(_) => {
            return Err(ModError::SelectError);
        }
    }

    let startup_parameter = format!("\"-mod={};\"", mods_to_install.join(";"));

    Ok(startup_parameter)
}

pub fn installed_mod_list(profile: Profile) -> Result<Vec<Value>, ModError> {
    let installed_mods = profile.installed_mods;

    Ok(installed_mods)
}

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
