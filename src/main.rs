use clap::{Parser, Subcommand};
use dayz_tool_cli::commands::generate_guid;
use std::env;
use std::fs::File;
use std::path::PathBuf;

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
}

/// Returns the path to the configuration file.
///
/// The configuration file is located in the `.dayz-tool` directory in the user's home directory.
///
/// # Example
///
/// ```rust
/// use dayz_tool_cli::get_config_path;
///
/// let config_path = get_config_path();
///
/// assert_eq!(config_path, PathBuf::from("/home/user/.dayz-tool/config.json"));
/// ```
fn get_config_path() -> PathBuf {
    let home_dir = match env::var("HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match env::var("USERPROFILE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                panic!("Home-Verzeichnis nicht gefunden!");
            }
        },
    };

    let mut config_path = PathBuf::from(home_dir);
    config_path.push(".dayz-tool");
    config_path.push("config.json");

    config_path
}

fn main() {
    let config_path = get_config_path();

    if !config_path.exists() {
        if let Err(e) = File::create(config_path) {
            eprintln!("Fehler beim Erstellen der Konfigurationsdatei: {}", e);
        }
    }

    let args = Cli::parse();
    match &args.commands {
        Commands::Guid { id } => match id {
            Some(id) => {
                let guid = generate_guid(id);
                println!("The GUID form {} is: {}", id, guid);
            }
            None => println!("No ID provided"),
        },
    }
}
