use std::{env::consts::OS, fs::write};

use chrono::Local;
use inquire::{Confirm, MultiSelect, Text};
use log::{debug, error};

use crate::{ConfigError, Profile};

/// Generates a startup script for the DayZ server based on the provided profile.
///
/// This function creates either a .sh (Linux/Unix) or .bat (Windows) startup script
/// with configurable server parameters. It allows users to either use a predefined
/// template or customize their own parameter selection.
///
/// # Arguments
/// * `profile` - A Profile struct containing server configuration details
///
/// # Returns
/// * `Result<(), ConfigError>` - Ok(()) on success, or ConfigError on failure
pub fn generate_startup_script(profile: Profile) -> Result<(), ConfigError> {
    debug!("Starting generating start script");

    let available_parameters: Vec<String> = vec![
        "-mission=".to_string(),
        "-doLogs".to_string(),
        "-adminLog".to_string(),
        "-netLog".to_string(),
        "-freezeCheck".to_string(),
        "-filePatching".to_string(),
        "-BEpath=".to_string(),
        "-cpuCount=".to_string(),
        "-limitFPS=".to_string(),
        "-mod=".to_string(),
        "-serverMod=".to_string(),
        "-storage=".to_string(),
    ];

    let mut final_parameters = vec![];

    let port = Text::new("Server Port:")
        .with_default("2302")
        .with_help_message("The port of your server")
        .prompt()
        .expect("Failed to get input");

    let use_template = Confirm::new("Use template?")
        .with_default(true)
        .with_help_message("Use a template for the startup script")
        .prompt();

    match use_template {
        Ok(true) => {
            let template_parameters = vec![
                "-BEpath=battleye".to_string(),
                "-doLogs".to_string(),
                "-adminLog".to_string(),
                "-netLog".to_string(),
                "-freezeCheck".to_string(),
            ];
            final_parameters.extend(template_parameters);
        }
        Ok(false) => {
            let selected_parameters = MultiSelect::new("Select parameters", available_parameters)
                .with_help_message("Select the parameters you want to use")
                .prompt();

            match selected_parameters {
                Ok(parameters) => {
                    debug!("Selected parameters: {:?}", parameters);

                    for parameter in parameters {
                        if parameter.ends_with('=') {
                            let value = Text::new(&format!("Enter value for {}", parameter))
                                .with_help_message("Enter the value for this parameter")
                                .prompt()
                                .expect("Failed to get input");
                            final_parameters.push(format!("{}{}", parameter, value));
                        } else {
                            final_parameters.push(parameter);
                        }
                    }

                    debug!("Final parameters: {:?}", final_parameters);
                }
                Err(_) => error!("Failed to select parameters"),
            }
        }
        Err(_) => error!("Failed confirm use template"),
    }

    let os = OS;
    let template_content = match os {
        "windows" => include_str!("../../templates/start_server.bat.template"),
        _ => include_str!("../../templates/start_server.sh.template"),
    };

    let generation_date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let final_content = template_content
        .replace("{server_name}", &profile.name)
        .replace("{server_path}", &profile.workdir_path)
        .replace("{server_port}", &port)
        .replace("{generation_date}", &generation_date)
        .replace("{additional_parameters}", &final_parameters.join(" "));

    let filename = if os == "windows" {
        "start_server.bat"
    } else {
        "start_server.sh"
    };
    let target_path = format!("{}/{}", profile.workdir_path, filename);

    write(&target_path, final_content).unwrap();

    if os != "windows" {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_path, perms).unwrap();
    }

    Ok(())
}
