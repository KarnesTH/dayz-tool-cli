# 🎮 DayZ Tool CLI

A command-line tool for simplifying DayZ server administration.

## ✨ Features

* 🔄 **Automatic Mod Installation:** Easily install mods from your Steam Workshop directory to your server.
* 🔑 **GUID Generator:** Generate GUIDs from Steam64 IDs for use in your server's whitelist.
* ⏰ **Day/Night Cycle Calculator:** Calculate the current time (day or night) on your DayZ server.
* 🚀 **(More features to be added soon!)**

## 📥 Installation

1. 🦀 **Install Rust and Cargo:** If you don't have them already, follow the instructions at [https://www.rust-lang.org/](https://www.rust-lang.org/).
2. 📂 **Clone this repository:**
   ```bash
   git clone https://github.com/KarnesTH/dayz-tool-cli.git
   ```
3. 📁 **Navigate to the project directory:**
   ```bash
   cd dayz-tool-cli
   ```
4. 🔨 **Build and install the CLI:**
   ```bash
   cargo install --path .
   ```

## 🛠️ Commands

```plaintext
dayz-tool-cli
├── mods                   # Mod management
│   ├── install            # Install mods from workshop
│   ├── list               # List installed mods
│   ├── update             # Update installed mods
│   └── uninstall          # Remove installed mods
│
├── generate               # Generation utilities
│   ├── guid               # GUID generator
│   │   └── <steam64Id>    # Generate GUID from Steam64 ID
│   ├── dnc                # Day/Night cycle calculator
│   │   ├── -d <time>      # Day length [h|min]
│   │   └── -n <time>      # Night length [h|min]
│   └── start-up           # Generate server start-up file
│
├── profile                # Profile management
│   ├── add                # Add a new profile
│   ├── show               # Show the current profile
│   ├── delete             # Delete a profile
│   ├── list               # List all profiles
│   ├── update             # Update a profile
│   └── use                # Use a profile
│
└── 🚀 More commands coming soon!
```

## ⚙️ Configuration

The CLI uses a configuration file named `config.json` to store settings. By default, this file is located in the `.dayz-tool` directory in your home directory.

## 📜 License

This project is licensed under the [MIT License](LICENSE).

## ⚠️ Disclaimer

This CLI tool is currently in development phase. To prevent potential data loss or file corruption, please ensure to create a backup of your files before using this tool.
