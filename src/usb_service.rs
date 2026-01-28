use crate::share_protocol::Command;
use serde_json::to_string;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

pub fn send_command_bindkey(port: &String, command: Command) -> Result<String, String> {
    let mut com_port = serialport::new(port, 115200)
        .timeout(Duration::from_secs(25))
        .open()
        .map_err(|e| format!("erreur : {e}"))?;

    let message =
        to_string(&command).map_err(|e| format!("Erreur de sérialisation JSON: {}", e))?;

    let message_finale = format!("{}\n", message);
    com_port
        .write(message_finale.as_bytes())
        .map_err(|e| format!("erreur : {e}"))?;

    let mut reader = BufReader::new(com_port);
    let mut response = String::new();

    reader
        .read_line(&mut response)
        .map_err(|e| format!("Erreur lecture (Timeout ?) : {}", e))?;
    let response_clean = response.trim().to_string();
    Ok(response_clean)
}
