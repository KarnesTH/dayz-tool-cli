use clap::{Parser, Subcommand};
use dayz_tool_cli::commands::{calculate_dnc, generate_guid, install_mods, installed_mod_list};
use dayz_tool_cli::utils::{
    create_initial_profile, get_config_path, get_profile, get_render_config,
};

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
    /// dayz-tool-cli guid 76561198039479170
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
    /// dayz-tool-cli dnc -d "8h" -n "10min"
    /// ```
    Dnc {
        /// The amount of time the server should be in day time. (e.g. 8h, 10min)
        #[arg(short = 'd', long)]
        day: Option<String>,
        /// The amount of time the server should be in night time. (e.g. 8h, 10min)
        #[arg(short = 'n', long)]
        night: Option<String>,
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
    Uninstall {
        /// The name of the mod to uninstall.
        mod_name: String,
    },

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

fn main() {
    inquire::set_global_render_config(get_render_config());
    let config_path = get_config_path();
    let profile = get_profile(&config_path);

    if !config_path.exists() {
        match create_initial_profile(&config_path) {
            Ok(_) => println!("Initial profile created successfully! You can now use the CLI. Run `dayz-tool-cli --help` for more information."),
            Err(_) => println!("Failed creating initial profile"),
        }
    } else {
        let args = Cli::parse();
        match &args.commands {
            Commands::Guid { id } => match id {
                Some(id) => {
                    let guid = generate_guid(id);
                    println!("The GUID form {} is: {}", id, guid);
                }
                None => println!("No ID provided"),
            },
            Commands::Dnc { day, night } => {
                if let (Some(day), Some(night)) = (day, night) {
                    match calculate_dnc(day, night) {
                        Ok((day_duration, night_duration)) => {
                            println!("serverTimeAcceleration = {}", day_duration);
                            println!("serverNightTimeAcceleration = {}", night_duration);
                        }
                        Err(e) => println!("{}", e),
                    }
                } else {
                    println!("Bitte geben Sie sowohl die Tag- als auch die Nachtlänge an.");
                }
            }
            Commands::Mods { subcommands } => match subcommands {
                ModCommands::Install => match profile {
                    Ok(profile) => match install_mods(profile) {
                        Ok(mods) => {
                            println!("Please add this: {} to your startup parameters", mods)
                        }
                        Err(_) => println!("Failed to install mods"),
                    },
                    Err(_) => println!("No profile found"),
                },
                ModCommands::Uninstall { mod_name } => {
                    println!("Uninstalling mod: {}", mod_name);
                }
                ModCommands::List => match profile {
                    Ok(profile) => match installed_mod_list(profile) {
                        Ok(mods) => {
                            println!("Installed mods:");
                            for installed_mod in mods {
                                println!("{}", installed_mod);
                            }
                        }
                        Err(_) => println!("No mods found"),
                    },
                    Err(_) => println!("No profile found"),
                },
                ModCommands::Update => {
                    println!("Updating mods...");
                }
            },
        }
    }
}
