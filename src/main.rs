use clap::{Parser, Subcommand};
use dayz_tool_cli::commands::{calculate_dnc, generate_guid};
use dayz_tool_cli::utils::{create_initial_profile, get_config_path, get_render_config};

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
}

fn main() {
    inquire::set_global_render_config(get_render_config());
    let config_path = get_config_path();

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
        }
    }
}
