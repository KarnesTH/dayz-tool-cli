use log::info;

use crate::{ConfigError, Profile, THEME};

/// Displays the configuration details of a DayZ profile in a formatted output.
///
/// This function prints various profile settings including the profile name,
/// working directory, workshop path, and a list of installed mods.
pub fn show_profile(profile: Profile) -> Result<(), ConfigError> {
    info!("{}", THEME.header("Profile Settings"));
    info!("{}:\t\t{}", THEME.label("Name"), THEME.value(&profile.name));
    info!(
        "{}:\t{}",
        THEME.label("Workdir"),
        THEME.value(&profile.workdir_path)
    );
    info!(
        "{}:\t{}",
        THEME.label("!Workshop"),
        THEME.value(&profile.workshop_path)
    );
    info!("{}:", THEME.label("Installed Mods"));

    if profile.installed_mods.is_empty() {
        info!("\t{}", THEME.value_italic("No mods installed."));
    } else {
        let mod_names: Vec<String> = profile
            .installed_mods
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if mod_names.is_empty() {
            info!("\t{}", THEME.value_italic("No valid mods found."));
        } else {
            for mod_name in mod_names {
                info!("\t{}", THEME.value(&mod_name));
            }
        }
    }

    Ok(())
}
