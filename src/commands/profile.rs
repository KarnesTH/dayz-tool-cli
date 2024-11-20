use std::path::PathBuf;

use inquire::{Confirm, Text};
use log::debug;

use crate::{
    utils::{add_profile, get_render_config, save_profile},
    ConfigError, Profile, THEME,
};

/// Displays the configuration details of a DayZ profile in a formatted output.
///
/// This function prints various profile settings including the profile name,
/// working directory, workshop path, and a list of installed mods.
pub fn show_profile(profile: Profile) -> Result<(), ConfigError> {
    debug!("Displaying profile information for '{}'", profile.name);
    println!("{}", THEME.header("Profile Settings"));
    println!("{}:\t\t{}", THEME.label("Name"), THEME.value(&profile.name));
    println!(
        "{}:\t{}",
        THEME.label("Workdir"),
        THEME.value(&profile.workdir_path)
    );
    println!(
        "{}:\t{}",
        THEME.label("!Workshop"),
        THEME.value(&profile.workshop_path)
    );
    println!("{}:", THEME.label("Installed Mods"));

    if profile.installed_mods.is_empty() {
        println!("\t{}", THEME.value_italic("No mods installed."));
    } else {
        let mod_names: Vec<String> = profile
            .installed_mods
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if mod_names.is_empty() {
            println!("\t{}", THEME.value_italic("No valid mods found."));
        } else {
            for mod_name in mod_names {
                println!("\t{}", THEME.value(&mod_name));
            }
        }
    }

    Ok(())
}

/// Updates an existing profile through an interactive command-line interface.
///
/// This function guides the user through a series of prompts to update various profile settings:
/// - Profile name
/// - Working directory path
/// - Workshop directory path
///
/// After each potential modification, the user is prompted to confirm whether they want to save
/// the changes. The function uses the inquire crate for user interaction and provides
/// a user-friendly interface with default values and help messages.
pub fn update_profile(mut profile: Profile) -> Result<(), ConfigError> {
    debug!("Starting profile update for '{}'", profile.name);

    println!("{}", THEME.header("Update Profile"));
    println!("{}", THEME.label("Current Settings:"));
    show_profile(profile.clone())?;

    if let Ok(true) = Confirm::new("Update profile name?")
        .with_default(false)
        .with_help_message("Change the profile name")
        .prompt()
    {
        let new_name = Text::new("New profile name:")
            .with_default(profile.name.as_str())
            .with_render_config(get_render_config())
            .prompt()
            .expect("Failed to get new profile name");
        profile.name = new_name;
    }

    if let Ok(true) = Confirm::new("Update working directory?")
        .with_default(false)
        .with_help_message("Change the DayZ server working directory path")
        .prompt()
    {
        let new_workdir = Text::new("New working directory path:")
            .with_default(profile.workdir_path.as_str())
            .with_help_message("Path to your DayZ server's working directory")
            .with_render_config(get_render_config())
            .prompt()
            .expect("Failed to get new working directory path");
        profile.workdir_path = new_workdir;
    }

    if let Ok(true) = Confirm::new("Update workshop path?")
        .with_default(false)
        .with_help_message("Change the DayZ workshop directory path")
        .prompt()
    {
        let new_workshop = Text::new("New workshop path:")
            .with_default(profile.workshop_path.as_str())
            .with_help_message("Path to your DayZ workshop directory")
            .with_render_config(get_render_config())
            .prompt()
            .expect("Failed to get new workshop path");
        profile.workshop_path = new_workshop;
    }

    if let Ok(true) = Confirm::new("Save changes?")
        .with_default(true)
        .with_help_message("Save all changes to this profile")
        .prompt()
    {
        save_profile(&profile)?;
        println!("{}", THEME.value_bold("Profile updated successfully!"));
    } else {
        println!("{}", THEME.value_italic("Changes discarded."));
    }

    Ok(())
}

/// Creates a new DayZ server profile by prompting the user for necessary information.
///
/// This function interactively collects the following information:
/// - Profile name (e.g., server name)
/// - Working directory path (path to DayZ server directory)
/// - Workshop path (path to DayZ workshop mods directory)
///
/// The created profile is then added to the configuration file.
pub fn create_profile(config_path: &PathBuf) -> Result<(), ConfigError> {
    debug!("Creating a new profile");

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
        is_active: false,
    };

    add_profile(config_path, &profile)?;

    Ok(())
}
