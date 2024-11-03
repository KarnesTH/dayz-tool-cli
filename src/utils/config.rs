use crate::{ConfigError, Profile, Root};
use inquire::ui::{Attributes, Color, RenderConfig, StyleSheet, Styled};
use inquire::Text;
use serde_json::to_string_pretty;
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
    let name = Text::new("Please enter a name.")
        .with_help_message("Please enter a name for your profile. (e.g. Your server's name)")
        .prompt()
        .expect("Failed to get name");

    let workdir_path = Text::new("What's your workdir path?").with_help_message("Please enter the path to your DayZ server's working directory. (e.g. /home/user/DayZServer)").prompt().expect("Failed to get workdir path");

    let workshop_path = Text::new("What's your !Workshop path?").with_help_message("Please enter the path to your DayZ server's workshop directory. (e.g. /home/user/DayZServer/steamapps/workshop/content/221100)").prompt().expect("Failed to get workshop path");

    let profile = Profile {
        name,
        workdir_path,
        workshop_path,
        installed_mods: vec![],
    };

    add_profile(config_path, &profile)?;

    Ok(())
}

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
