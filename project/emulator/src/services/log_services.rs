use crate::layers::underlay_network::underlay_multi_tx;
use crate::models::network_models::EmulatorSocket;

use super::time_services::get_current_timestamp;

pub async fn log_info(emulator_socket: &EmulatorSocket, info_message: &str) 
{
    
    // get current timestamp
    match get_current_timestamp() {
        Ok(current_timestamp) => {

            // send info log with NID and time
            let message = format!("INFO;0x{:016X};{};{}", emulator_socket.local_network.local_nid, current_timestamp, info_message);
            match underlay_multi_tx(emulator_socket, &"log".to_string(), message.as_bytes()).await {
                Ok(()) => {},
                Err(err) => {
                    eprintln!("SYSTEMLOG: log_info(): failed to send log: {}", err);
                }
            }

        },
        Err(err) => {
            eprintln!("SYSTEMLOG: log_info(): failed to send log: {}", err);
        }
    }
}

pub async fn log_error(emulator_socket: &EmulatorSocket, error_message: &str) 
{

    // get current timestamp
    match get_current_timestamp() {
        Ok(current_timestamp) => {

            // send error log with NID and time
            let message = format!("ERROR;0x{:016X};{};{}", emulator_socket.local_network.local_nid, current_timestamp, error_message);
            match underlay_multi_tx(emulator_socket, &"log".to_string(), message.as_bytes()).await {
                Ok(()) => {},
                Err(err) => {
                    eprintln!("SYSTEMLOG: log_error(): failed to send log: {}", err);
                }
            }

        },
        Err(err) => {
            eprintln!("SYSTEMLOG: log_error(): failed to send log: {}", err);
        }
    }
}
