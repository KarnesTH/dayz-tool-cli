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
* **`dayz-tool-cli mod install <mod_name>`:** Installs the specified mod from your Steam Workshop directory to your server.
* **(More commands to be added soon!)**

```meirmaid
---
title: dayz-tools-cli
---
flowchart LR
    A[Start] --> B{'.dayz-tool' Ordner existiert?}
    B -- Ja --> C{Config.json existiert?}
    C -- Ja --> D{CLI im Hauptmenü}
    C -- Nein --> E{Profil erstellen}
    B -- Nein --> E
    E --> F{Profil-Informationen eingeben}
    F --> G{Config.json erstellen}
    G --> D
    D --> H{Benutzer gibt Command ein}
    H -- "dayz-tool-cli guid <id>" --> I{GUID generieren}
    H -- "dayz-tool-cli dnc -d 8h -n 10min" --> J{Day/Night-Zyklus berechnen}
    H -- "dayz-tool-cli mod install" --> K{Mods laden & anzeigen}
    H -- "dayz-tool-cli mod update" --> L{Mod-Updates laden & installieren}
    H -- (weitere Commands) --> M{Ungültiger Command}
    I --> N{GUID ausgeben}
    J --> O{Einstellungen ausgeben}
    K --> P{Benutzer wählt Mod}
    L --> Q{Mod-Updates installieren}
    P --> R{Mod installieren}
    Q & R --> S{Erfolgsmeldung}
    M --> T{Fehlermeldung ausgeben}
    N & O & S --> D
    T --> D

    subgraph CLI im Hauptmenü
        D --> H
    end

    subgraph Profil erstellen
        E --> F
        F --> G
    end

    subgraph GUID generieren
        I --> N
    end

    subgraph Day/Night-Zyklus berechnen
        J --> O
    end

    subgraph Mod installieren
        K --> P
        P --> R
    end

    subgraph Mod aktualisieren
        L --> Q
        Q --> S
    end

    subgraph Fehlerbehandlung
        M --> T
    end
```
[![](https://mermaid.ink/img/pako:eNqFVM1uGjEQfhVrD00rZdP2VqEqVQOEnxBIQnJptweDza4b7xjZXqXEytv0MXLjxTrrNSxZQorEYWa-75vx_KyL5orxqBXFcZyAFVbyFmF09RhbpaSJ51Ik4GMLqR7mGdWWjG4SIPj7_nNq0f5F4viUnLmjky3viEw0A64J_yOMFVzbb08V5wzBZEg9pe3aChYiPfltFOxD2zvQjmuPBkTkpE-Lpc05rJ93UWMuwOO67kqrhZCEa2O5lBx289awytn1xnngxANYKJ1TKxRwrEdAymdbgXOP7b0suZGkV9VaGR1v9N0Zh8I-Yi9SMbOkrfKcAivVA6lfVpZE296VLSdpIRj5KthpEnmZgevdDTok5dhU7BF_k8xgTmJGvmQkBvL5Uy4gqAxdh64-jkWa2fjH6l4WhsxQbJ7B24K5YkSAsVTKoHThLhUzRFKGnXpHKDxykf5fpFgyannQGJUa8Z131VIhz94j3z9wYdG56aD54EUu3R2k62dpMb3exAJv4BHjqnW0MLvjHPrYxHXLfDjDAocNTdCFB13VI3xY_82kJVh3QIw84vrFS155wZWH3ZSw18LX-O4bD5m6rl4omZqcS4Y1BcClD966c55JrkOsWe0YVSb4n-5u4e3GqExTzFJNlxnZO6cqvrO4lYMD2-M2T6ymhoOqHeFqDms11rpmhuEdZh5e5VokTPmwSHMgNTXMvnaEGb6tRe9tQaUwDbGwJrXjupr2YbFq0jOe4TqXy1lTwy5sqdFxlONOUMHwK-5KdxLZjOd4Zi1_hPo-iRJ4QhwtrJquYB61rC74caRVkWZRa0GlQas6zo6gWEFeQZ7-AT5a5Gg?type=png)](https://mermaid.live/edit#pako:eNqFVM1uGjEQfhVrD00rZdP2VqEqVQOEnxBIQnJptweDza4b7xjZXqXEytv0MXLjxTrrNSxZQorEYWa-75vx_KyL5orxqBXFcZyAFVbyFmF09RhbpaSJ51Ik4GMLqR7mGdWWjG4SIPj7_nNq0f5F4viUnLmjky3viEw0A64J_yOMFVzbb08V5wzBZEg9pe3aChYiPfltFOxD2zvQjmuPBkTkpE-Lpc05rJ93UWMuwOO67kqrhZCEa2O5lBx289awytn1xnngxANYKJ1TKxRwrEdAymdbgXOP7b0suZGkV9VaGR1v9N0Zh8I-Yi9SMbOkrfKcAivVA6lfVpZE296VLSdpIRj5KthpEnmZgevdDTok5dhU7BF_k8xgTmJGvmQkBvL5Uy4gqAxdh64-jkWa2fjH6l4WhsxQbJ7B24K5YkSAsVTKoHThLhUzRFKGnXpHKDxykf5fpFgyannQGJUa8Z131VIhz94j3z9wYdG56aD54EUu3R2k62dpMb3exAJv4BHjqnW0MLvjHPrYxHXLfDjDAocNTdCFB13VI3xY_82kJVh3QIw84vrFS155wZWH3ZSw18LX-O4bD5m6rl4omZqcS4Y1BcClD966c55JrkOsWe0YVSb4n-5u4e3GqExTzFJNlxnZO6cqvrO4lYMD2-M2T6ymhoOqHeFqDms11rpmhuEdZh5e5VokTPmwSHMgNTXMvnaEGb6tRe9tQaUwDbGwJrXjupr2YbFq0jOe4TqXy1lTwy5sqdFxlONOUMHwK-5KdxLZjOd4Zi1_hPo-iRJ4QhwtrJquYB61rC74caRVkWZRa0GlQas6zo6gWEFeQZ7-AT5a5Gg)

## Configuration

The CLI uses a configuration file named `config.json` to store settings. By default, this file is located in the `.dayz-tool` directory in your home directory.

## License

This project is licensed under the [MIT License](LICENSE).
