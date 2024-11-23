use inquire::{Confirm, MultiSelect, Text};
use log::{debug, error};

use crate::{ConfigError, Profile};

pub fn generate_startup_script(_profile: Profile) -> Result<(), ConfigError> {
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

    let _port = Text::new("Server Port:")
        .with_default("2302")
        .with_help_message("The port of your server")
        .prompt()
        .expect("Failed to get input");

    let use_template = Confirm::new("Use template?")
        .with_default(true)
        .with_help_message("Use a template for the startup script")
        .prompt();

    match use_template {
        Ok(true) => debug!("template selected"),
        Ok(false) => {
            let selected_parameters = MultiSelect::new("Select parameters", available_parameters)
                .with_help_message("Select the parameters you want to use")
                .prompt();

            match selected_parameters {
                Ok(parameters) => {
                    debug!("Selected parameters: {:?}", parameters);
                }
                Err(_) => error!("Failed to select parameters"),
            }
        }
        Err(_) => error!("Failed confirm use template"),
    }

    Ok(())
}
