use crate::{ConfigError, Profile, Root};
use serde_json::to_string_pretty;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::{stdin, Write};
use std::path::PathBuf;

/// Returns the path to the configuration file.
///
/// The configuration file is located in the `.dayz-tool` directory in the user's home directory.
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
/// use dayz_tool_cli::utils::get_config_path;
///
/// let config_path = get_config_path();
///
/// assert_eq!(config_path, PathBuf::from("/home/karnes/.dayz-tool/config.json"));
/// ```
pub fn get_config_path() -> PathBuf {
    let home_dir = match env::var("HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match env::var("USERPROFILE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                panic!("Failed to get the user's home directory.");
            }
        },
    };

    let mut config_path = PathBuf::from(home_dir);
    config_path.push(".dayz-tool");
    config_path.push("config.json");

    config_path
}

pub fn add_profile(config_path: &PathBuf, profile: &Profile) -> Result<(), ConfigError> {
    let mut config = if config_path.exists() {
        match read_config_file(config_path) {
            Ok(config) => config,
            Err(_) => return Err(ConfigError::OpenFileError),
        }
    } else {
        Root { profiles: vec![] }
    };

    config.profiles.push(profile.clone());

    let json = to_string_pretty(&config).unwrap();

    if let Err(e) = create_dir_all(config_path.parent().unwrap()) {
        eprintln!("Failed to create directory: {}", e);
        return Err(ConfigError::CreateFileError);
    }

    let mut config_file = match File::create(config_path) {
        Ok(file) => file,
        Err(_) => return Err(ConfigError::CreateFileError),
    };

    if let Err(_) = config_file.write_all(json.as_bytes()) {
        return Err(ConfigError::WriteFileError);
    }

    Ok(())
}

pub fn read_config_file(config_path: &PathBuf) -> Result<Root, ConfigError> {
    let config_file = match File::open(config_path) {
        Ok(file) => file,
        Err(_) => return Err(ConfigError::OpenFileError),
    };

    let config: Root = match serde_json::from_reader(config_file) {
        Ok(config) => config,
        Err(_) => return Err(ConfigError::ParseError),
    };

    Ok(config)
}

pub fn create_initial_profile(config_path: &PathBuf) -> Result<(), ConfigError> {
    println!("It's looks like this is your first time using dayz-tool-cli!");
    println!("Let's create your first profile");
    println!("Please enter a name for your profile. (e.g. Your server's name)");
    let mut name = String::new();
    stdin().read_line(&mut name).unwrap();
    name = name.trim().to_string();

    println!("Please enter the path to your DayZ server's working directory. (e.g. /home/user/DayZServer)");
    let mut workdir_path = String::new();
    stdin().read_line(&mut workdir_path).unwrap();
    workdir_path = workdir_path.trim().to_string();

    println!("Please enter the path to your DayZ server's workshop directory. (e.g. /home/user/DayZServer/steamapps/workshop/content/221100)");
    let mut workshop_path = String::new();
    stdin().read_line(&mut workshop_path).unwrap();
    workshop_path = workshop_path.trim().to_string();

    let profile = Profile {
        name,
        workdir_path,
        workshop_path,
        installed_mods: vec![],
    };

    add_profile(config_path, &profile)?;

    Ok(())
}
