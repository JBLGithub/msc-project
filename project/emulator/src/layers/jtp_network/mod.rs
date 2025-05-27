use once_cell::sync::Lazy;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use std::sync::Arc;
use std::time::Duration;

use crate::models::network_models::{EmulatorSocket, JTPResponse};
use crate::layers::overlay_network::{ open_ilnp_socket, close_ilnp_socket };
use crate::layers::overlay_network::{ilnp_nid_tx, ilnp_fqdn_tx};

/// JTP QUEUE
///     - this queue is used to store incoming data packets
///     - user may experience packet drops if this is too small
pub static JTP_QUEUE: Lazy<(mpsc::Sender<JTPResponse>, Arc<Mutex<mpsc::Receiver<JTPResponse>>>)> = Lazy::new(|| {
    let (tx, rx) = mpsc::channel(100);
    (tx, Arc::new(Mutex::new(rx)))
});


/// Open a JTP socket
///     - creates a receiver handler for JCMP Packets
///     - creates a receiver handler for JTP Packets
pub async fn open_jtp_socket()
    -> Result<EmulatorSocket, String>
{
    open_ilnp_socket().await
}

/// Close a JTP socket
pub async fn close_jtp_socket(emulator_socket: EmulatorSocket)
    -> Result<(), String>
{
    close_ilnp_socket(emulator_socket).await
}

/// Send a JTP packet using NID
pub async fn jtp_nid_tx(emulator_socket: &EmulatorSocket, destination_nid:&u64, buf:&[u8])
    -> Result<(), String>
{
    ilnp_nid_tx(emulator_socket, destination_nid, buf).await
}

/// Send a JTP packet using FQDN
pub async fn jtp_fqdn_tx(emulator_socket: &EmulatorSocket, destination_fqdn:&String, buf:&[u8])
    -> Result<(), String>
{
    ilnp_fqdn_tx(emulator_socket, destination_fqdn, buf).await
}

/// JTP receiver
///     - (-1) for blocking
///     - (0) for pool
///     - (+t) for timeout in seconds
pub async fn jtp_rx(timeout_millisecs: i64)
    -> Result<JTPResponse, String> 
{
    let rx = JTP_QUEUE.1.clone();
    let mut rx_lock = rx.lock().await;

    // blocking
    if timeout_millisecs < 0 {
        match rx_lock.recv().await {
            Some(packet) => Ok(packet),
            None => Err("jtp_rx(): JTP queue closed".to_string()),
        }
    }

    // pool
    else if timeout_millisecs == 0 {
        match rx_lock.try_recv() {
            Ok(packet) => Ok(packet),
            Err(TryRecvError::Empty) => Err("jtp_rx(): no packets".to_string()),
            Err(TryRecvError::Disconnected) => Err("jtp_rx(): JTP queue closed".to_string())
        }
    }

    // timeout
    else {
        let duration = Duration::from_millis(timeout_millisecs as u64);
        match timeout(duration, rx_lock.recv()).await {
            Ok(Some(packet)) => Ok(packet),
            Ok(None) => Err("jtp_rx(): JTP queue closed".to_string()),
            Err(_elapsed) => Err("jtp_rx(): timed out".to_string()),
        }
    }
}