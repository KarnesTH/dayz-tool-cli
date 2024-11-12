# DayZ Tool CLI

A command-line tool for simplifying DayZ server administration.

## Features

* **Automatic Mod Installation:** Easily install mods from your Steam Workshop directory to your server.
* **GUID Generator:**  Generate GUIDs from Steam64 IDs for use in your server's whitelist.
* **Day/Night Cycle Calculator:** Calculate the current time (day or night) on your DayZ server.
* **(More features to be added soon!)**

## Installation

1. **Install Rust and Cargo:** If you don't have them already, follow the instructions at [https://www.rust-lang.org/](https://www.rust-lang.org/).
2. **Clone this repository:**
   ```bash
   git clone https://github.com/KarnesTH/dayz-tool-cli.git
   ```
3. **Navigate to the project directory:**
   ```bash
   cd dayz-tool-cli
   ```
4. **Build and install the CLI:**
   ```bash
   cargo install --path .
   ```

## Usage

After installation, you can use the following commands:

* **`dayz-tool-cli guid <steam64Id>`:** Generates a GUID from the given Steam64 ID.
* **`dayz-tool-cli dnc -d 8h -n 10min`:** Calculates and displays the `serverTimeAcceleration` and `serverNightTimeAcceleration` settings for a DayZ server, based on the desired day and night lengths..
* **`dayz-tool-cli mods install`:** Installs mods from your Steam Workshop directory to your server.
* **(More commands to be added soon!)**

## Configuration

The CLI uses a configuration file named `config.json` to store settings. By default, this file is located in the `.dayz-tool` directory in your home directory.

## License

This project is licensed under the [MIT License](LICENSE).
