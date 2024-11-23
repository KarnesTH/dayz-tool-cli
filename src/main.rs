use clap::{Parser, Subcommand};
use dayz_tool_cli::commands::{
    calculate_dnc, create_profile, delete_profile, generate_guid, generate_startup_script,
    install_mods, list_installed_mods, list_profiles, show_profile, switch_profile, uninstall_mods,
    update_mods, update_profile,
};
use dayz_tool_cli::utils::{
    create_initial_profile, get_config_path, get_profile, get_render_config, init_logger,
};
use dayz_tool_cli::{THEME, THREAD_POOL};
use log::{debug, error, info};

/// A command-line tool for simplifying DayZ server administration.
///
/// This tool provides commands for managing your DayZ server,
/// including mod installation, GUID generation, and Day/Night cycle calculation.
///
/// To view available commands and their usage, use the `--help` flag.
///
/// Example:
/// ```bash
/// dayz-tool-cli --help
/// ```
#[derive(Parser)]
#[command(author = "KarnesTH", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Provides various generation utilities for DayZ server administration.
    ///
    /// This command offers subcommands for generating different types of data and configurations
    /// commonly needed in DayZ server administration, such as GUIDs from Steam64 IDs and
    /// day/night cycle calculations.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli generate <subcommand> [options]
    /// ```
    Generate {
        #[command(subcommand)]
        subcommands: GenerateCommands,
    },

    /// Manages mods for the DayZ server.
    ///
    /// This command provides subcommands for installing, uninstalling, listing, and updating mods.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli mod <subcommand>
    /// ```
    Mods {
        #[command(subcommand)]
        subcommands: ModCommands,
    },

    /// Manages profiles for the CLI.
    ///
    /// This command provides subcommands for creating, updating, deleting, and switching between profiles.
    /// Profiles allow you to store and manage different configurations for the CLI.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile <subcommand>
    /// ```
    Profile {
        #[command(subcommand)]
        subcommands: ProfileCommands,
    },
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generates a GUID from a Steam64 ID.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli guid <steam64Id>
    /// ```
    ///
    /// # Example
    ///
    /// ```bash
    /// dayz-tool-cli generate guid 76561198039479170
    /// ```
    Guid {
        /// The Steam64 ID to generate the GUID from.
        id: Option<String>,
    },

    /// Converts hours and minutes into DayZ server settings for Day Night Cycle.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli generate dnc -d "8h" -n "10min"
    /// ```
    Dnc {
        /// The amount of time the server should be in day time. (e.g. 8h, 10min)
        #[arg(short = 'd', long)]
        day: Option<String>,
        /// The amount of time the server should be in night time. (e.g. 8h, 10min)
        #[arg(short = 'n', long)]
        night: Option<String>,
    },

    /// Generates a server_start script for the DayZ server.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli generate start-up
    /// ```
    StartUp,
}

#[derive(Subcommand)]
enum ModCommands {
    /// Installs selected mods from the Workshop directory.
    ///
    /// Please ensure that your Workshop directory is correctly configured in the profile settings.
    /// Mods must be subscribed to on the Steam Workshop.
    /// (e.g. when using the standalone dayz launcher you can find the !Workshop folder under: path/to/steam/steamapps/common/DayZ/!Workshop)
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli mod install
    /// ```
    Install,

    /// Uninstalls a mod from the server.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli mod uninstall <modName>
    /// ```
    Uninstall,

    /// Lists all installed mods.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli mod list
    /// ```
    List,

    /// Updates all installed mods.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli mod update
    /// ```
    Update,
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Displays the current profile settings.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile show
    /// ```
    Show,

    /// Updates the profile settings.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile update
    /// ```
    Update,

    /// Deletes the current profile settings.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile delete
    /// ```
    Delete,

    /// Creates a new profile.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile add
    /// ```
    Add,

    /// Lists all available profiles.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile list
    /// ```
    List,

    /// Uses the specified profile.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli profile use <profileName>
    /// ```
    Use,
}

fn main() {
    inquire::set_global_render_config(get_render_config());

    if let Err(e) = init_logger() {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }

    let config_path = get_config_path();
    let profile = get_profile(&config_path);

    if !config_path.exists() {
        match create_initial_profile(&config_path) {
            Ok(_) => info!("Initial profile created successfully! You can now use the CLI. Run `dayz-tool-cli --help` for more information."),
            Err(_) => error!("Failed creating initial profile"),
        }
    } else {
        let args = Cli::parse();
        match &args.commands {
            Commands::Generate { subcommands } => match subcommands {
                GenerateCommands::Guid { id } => match id {
                    Some(id) => {
                        let guid = generate_guid(id);
                        debug!("The GUID form {} is: {}", id, guid);
                        println!(
                            "The GUID from {} is: {}",
                            THEME.value_italic(id),
                            THEME.value_bold(guid)
                        )
                    }
                    None => error!("No ID provided"),
                },
                GenerateCommands::Dnc { day, night } => {
                    if let (Some(day), Some(night)) = (day, night) {
                        match calculate_dnc(day, night) {
                            Ok((day_duration, night_duration)) => {
                                info!("serverTimeAcceleration = {}", day_duration);
                                info!("serverNightTimeAcceleration = {}", night_duration);
                            }
                            Err(e) => error!("{}", e),
                        }
                    } else {
                        error!("Please enter both the day and night length.");
                    }
                }
                GenerateCommands::StartUp => match profile {
                    Ok(profile) => match generate_startup_script(profile) {
                        Ok(_) => info!("Startup script generated successfully!"),
                        Err(_) => error!("Failed to generate startup script"),
                    },
                    Err(_) => error!("No profile found"),
                },
            },
            Commands::Mods { subcommands } => match subcommands {
                ModCommands::Install => match profile {
                    Ok(profile) => {
                        match install_mods(&THREAD_POOL, profile) {
                            Ok(mods) => {
                                println!(
                                    "Please add this: {} to your startup parameters",
                                    THEME.value_bold(mods)
                                )
                            }
                            Err(_) => error!("Failed to install mods"),
                        };
                    }
                    Err(_) => error!("No profile found"),
                },
                ModCommands::Uninstall => match profile {
                    Ok(profile) => match uninstall_mods(profile, &THREAD_POOL) {
                        Ok(mods) => mods,
                        Err(_) => error!("Failed to uninstall mods"),
                    },
                    Err(_) => error!("No profile found"),
                },
                ModCommands::List => match profile {
                    Ok(profile) => match list_installed_mods(profile) {
                        Ok(mods) => mods,
                        Err(_) => error!("No mods found"),
                    },
                    Err(_) => error!("No profile found"),
                },
                ModCommands::Update => match profile {
                    Ok(profile) => match update_mods(profile, &THREAD_POOL) {
                        Ok(mods) => mods,
                        Err(_) => error!("Failed to update mods"),
                    },
                    Err(_) => error!("No profile found"),
                },
            },
            Commands::Profile { subcommands } => match subcommands {
                ProfileCommands::Show => match profile {
                    Ok(profile) => match show_profile(profile) {
                        Ok(_) => (),
                        Err(_) => error!("Failed to show profile"),
                    },
                    Err(_) => error!("No profile found"),
                },
                ProfileCommands::Update => match profile {
                    Ok(profile) => match update_profile(profile) {
                        Ok(_) => (),
                        Err(_) => error!("Failed to update profile"),
                    },
                    Err(_) => error!("No profile found"),
                },
                ProfileCommands::Delete => match delete_profile(&config_path) {
                    Ok(_) => info!("Profile deleted successfully"),
                    Err(_) => error!("Failed to delete profile"),
                },
                ProfileCommands::Add => match create_profile(&config_path) {
                    Ok(_) => info!("Profile created successfully"),
                    Err(_) => error!("Failed to create profile"),
                },
                ProfileCommands::List => match list_profiles(&config_path) {
                    Ok(_) => (),
                    Err(_) => error!("Failed to list profiles"),
                },
                ProfileCommands::Use => match switch_profile(&config_path) {
                    Ok(_) => info!("Profile switched successfully"),
                    Err(_) => error!("Failed to switch profile"),
                },
            },
        }
    }
}
