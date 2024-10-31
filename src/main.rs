use clap::{Parser, Subcommand};
use dayz_tool_cli::generators::generate_guid;

#[derive(Parser)]
#[command(author = "KarnesTH", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Guid { id: Option<String> },
}

fn main() {
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
