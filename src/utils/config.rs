use crate::{ConfigError, Profile, Root};
use inquire::ui::{Attributes, Color, RenderConfig, StyleSheet, Styled};
use inquire::Text;
use serde_json::{to_string_pretty, Value};
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
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

    let mut config_path = home_dir;
    config_path.push(".dayz-tool");
    config_path.push("config.json");

    config_path
}

/// Retrieves the active profile from the configuration file.
///
/// This function reads the configuration file from the given path and returns the active profile.
/// If the configuration file cannot be read or parsed, or if no active profile is found, an appropriate
/// `ConfigError` is returned.
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
/// use dayz_tool_cli::utils::{get_profile, get_config_path};
///
/// let profile = get_profile(&get_config_path());
/// ```
pub fn get_profile(config_path: &PathBuf) -> Result<Profile, ConfigError> {
    let config = read_config_file(config_path)?;

    let profiles = config.profiles;
    let active_profile = profiles.iter().find(|profile| profile.is_active);

    Ok(active_profile.unwrap().clone())
}

pub fn get_profiles(config_path: &PathBuf) -> Result<Vec<Profile>, ConfigError> {
    let config = read_config_file(config_path)?;

    Ok(config.profiles)
}

pub fn remove_profile(config_path: &PathBuf, profile: &Profile) -> Result<(), ConfigError> {
    let profiles = get_profiles(config_path)?;

    for (i, p) in profiles.iter().enumerate() {
        if p.name == profile.name {
            let mut config = read_config_file(config_path)?;
            config.profiles.remove(i);
            let json = to_string_pretty(&config).unwrap();
            let mut file = File::create(config_path).unwrap();
            file.write_all(json.as_bytes()).unwrap();
            return Ok(());
        }
    }

    Ok(())
}

/// Adds a new profile to the configuration file.
///
/// This function takes a path to the configuration file and a `Profile` object, and adds the profile
/// to the configuration file. If the configuration file does not exist, it will be created. If any error
/// occurs during the process, an appropriate `ConfigError` is returned.
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

    if config_file.write_all(json.as_bytes()).is_err() {
        return Err(ConfigError::WriteFileError);
    }

    Ok(())
}

/// Reads the configuration file and returns the parsed configuration.
///
/// This function takes a path to the configuration file, reads its contents, and parses it into a `Root` object.
/// If the configuration file cannot be opened or parsed, an appropriate `ConfigError` is returned.
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

/// Creates an initial profile by prompting the user for profile details.
///
/// This function guides the user through the process of creating their first profile by prompting
/// for the profile name, work directory path, and workshop path. The created profile is then added
/// to the configuration file. If any error occurs during the process, an appropriate `ConfigError`
/// is returned.
pub fn create_initial_profile(config_path: &PathBuf) -> Result<(), ConfigError> {
    println!("It's looks like this is your first time using dayz-tool-cli!");
    println!("Let's create your first profile");
    let name = Text::new("Please enter a name.")
        .with_help_message("Please enter a name for your profile. (e.g. Your server's name)")
        .prompt()
        .expect("Failed to get name");

    let workdir_path = Text::new("What's your workdir path?").with_help_message("Please enter the path to your DayZ server's working directory. (e.g. /home/user/DayZServer)").prompt().expect("Failed to get workdir path");

    let workshop_path = Text::new("What's your !Workshop path?").with_help_message("Please enter the path to your DayZ server's workshop directory. (e.g. for the DayZ Standalone Launcher /path/to/steam/steamapps/common/DayZ/!Workshop)").prompt().expect("Failed to get workshop path");

    let profile = Profile {
        name,
        workdir_path,
        workshop_path,
        installed_mods: vec![],
        is_active: true,
    };

    add_profile(config_path, &profile)?;

    Ok(())
}

/// Saves a given profile to the configuration file.
///
/// This function updates the active profile in the configuration file with the
/// data from the provided profile. The profile to be updated is identified by
/// the `is_active` flag.
pub fn save_profile(profile: &Profile) -> Result<(), ConfigError> {
    let config_path = get_config_path();
    let mut config = read_config_file(&config_path)?;

    if let Some(existing_profile) = config.profiles.iter_mut().find(|p| p.is_active) {
        *existing_profile = profile.clone();
        let json = to_string_pretty(&config).map_err(|_| ConfigError::SerializeError)?;
        let mut file = File::create(&config_path).map_err(|_| ConfigError::CreateFileError)?;
        file.write_all(json.as_bytes())
            .map_err(|_| ConfigError::WriteFileError)?;
        Ok(())
    } else {
        Err(ConfigError::NoActiveProfile)
    }
}

/// Adds a list of mods to the active profile in the configuration file.
///
/// This function takes a list of mod names, reads the configuration file, and adds the mods
/// to the active profile's list of installed mods. If any error occurs during the process,
/// an appropriate `ConfigError` is returned.
pub fn add_mods_to_profile(mods: Vec<String>) -> Result<(), ConfigError> {
    let config_path = get_config_path();

    let mut config = read_config_file(&config_path)?;

    let active_profile = config
        .profiles
        .iter_mut()
        .find(|p| p.is_active)
        .ok_or(ConfigError::NoActiveProfile)?;

    let mods_as_values: Vec<Value> = mods.into_iter().map(Value::String).collect();

    active_profile.installed_mods.extend(mods_as_values);

    let json = to_string_pretty(&config).map_err(|_| ConfigError::SerializeError)?;

    let mut config_file = File::create(&config_path).map_err(|_| ConfigError::CreateFileError)?;

    config_file
        .write_all(json.as_bytes())
        .map_err(|_| ConfigError::WriteFileError)?;

    Ok(())
}

/// Removes specified mods from the active profile's installed mods list in the configuration file.
///
/// This function updates the config.json by removing the specified mods from the installed_mods
/// array of the active profile. The function handles the entire process of reading the current
/// configuration, modifying it, and writing it back to disk.
pub fn remove_mods_from_profile(mods_to_remove: &[String]) -> Result<(), ConfigError> {
    let config_path = get_config_path();
    let mut config = read_config_file(&config_path)?;

    let active_profile = config
        .profiles
        .iter_mut()
        .find(|p| p.is_active)
        .ok_or(ConfigError::NoActiveProfile)?;

    active_profile.installed_mods.retain(|mod_entry| {
        !mods_to_remove.contains(&mod_entry.as_str().unwrap_or("").to_string())
    });

    let json = to_string_pretty(&config).map_err(|_| ConfigError::SerializeError)?;
    let mut config_file = File::create(&config_path).map_err(|_| ConfigError::CreateFileError)?;
    config_file
        .write_all(json.as_bytes())
        .map_err(|_| ConfigError::WriteFileError)?;

    Ok(())
}

/// Returns a customized render configuration for prompts.
///
/// This function creates and returns a `RenderConfig` object with customized styles for
/// various elements of the prompt, such as the prompt prefix, highlighted option prefix,
/// selected and unselected checkboxes, scroll prefixes, error messages, answers, and help messages.
pub fn get_render_config() -> RenderConfig<'static> {
    let mut render_config = RenderConfig::default();
    render_config.prompt_prefix = Styled::new(">").with_fg(Color::DarkCyan);
    render_config.highlighted_option_prefix = Styled::new("->").with_fg(Color::LightBlue);
    render_config.selected_checkbox = Styled::new("[X]").with_fg(Color::LightGreen);
    render_config.scroll_up_prefix = Styled::new("⇞");
    render_config.scroll_down_prefix = Styled::new("⇟");
    render_config.unselected_checkbox = Styled::new("[ ]");

    render_config.error_message = render_config
        .error_message
        .with_prefix(Styled::new("❌").with_fg(Color::LightRed));

    render_config.answer = StyleSheet::new()
        .with_attr(Attributes::ITALIC)
        .with_fg(Color::LightBlue);

    render_config.help_message = StyleSheet::new().with_fg(Color::DarkCyan);

    render_config
}
