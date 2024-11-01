use clap::{Parser, Subcommand};
use dayz_tool_cli::commands::generate_guid;
use dayz_tool_cli::utils::{create_initial_profile, get_config_path};

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
    Guid { id: Option<String> },

    /// Converts hours and minutes into DayZ server settings for Day Night Cycle.
    ///
    /// # Usage
    ///
    /// ```bash
    /// dayz-tool-cli dnc -d "8h" -n "10min"
    /// ```
    Dnc {
        #[arg(short = 'd', long)]
        day: Option<String>,
        #[arg(short = 'n', long)]
        night: Option<String>,
    },
}

fn main() {
    let config_path = get_config_path();

    if !config_path.exists() {
        match create_initial_profile(&config_path) {
            Ok(_) => println!("Initial profile created"),
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
                println!("Day: {:?}", day);
                println!("Night: {:?}", night);
            }
        }
    }
}
