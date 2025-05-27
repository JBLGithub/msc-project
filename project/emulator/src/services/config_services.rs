use std::fs;
use std::process::Command;

use crate::models::config_models::Config;

/// Function to retrieve config from config.toml
pub fn get_config() 
    -> Result<Config, String> 
{
    // open config file
    match fs::read_to_string("config/Config.toml") {
        Ok(config_content) => {

            // parse the config file
            match toml::from_str::<Config>(&config_content) {
                Ok(config) => {

                    // return config
                    Ok(config)
                    
                },
                Err(err) => {
                    Err(format!("get_config(): {}", err))
                }
            }
        }, 
        Err(err) => {
            Err(format!("get_config(): {}", err))
        }
    }
}


/// Function to retrieve the user's UID
pub fn get_uid()
    -> Result<u16, String>
{
    // run "id -u"
    match Command::new("id").arg("-u").output() {
        Ok(output) => {
            if output.status.success() {

                // retrieve uid from command output
                let mut uid_string: String = String::from_utf8_lossy(&output.stdout).to_string();
                uid_string = uid_string.trim().to_string();

                // convert uid to u16
                match uid_string.parse::<u16>() {
                    Ok(uid) => {
                        Ok(uid)
                    },
                    Err(err)  => {
                        Err(format!("get_uid(): {}", err))
                    }
                }
            }
            else {
                let err: String = String::from_utf8_lossy(&output.stderr).to_string();
                Err(format!("get_uid(): {}", err))
            }
        },
        Err(err) => {
            Err(format!("get_uid(): {}", err))
        }
    }
}
    
