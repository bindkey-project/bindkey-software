use crate::protocol::Command;
use serde_json::to_string;
use std::time::Duration;

pub fn send_command_bindkey(port: &String, command: Command) -> Result<String, String> {
    
    let mut com_port = serialport::new(port, 115200)
        .timeout(Duration::from_secs(1))
        .open()
        .map_err(|e| format!("erreur : {e}"))?;
    
    let message = to_string(&command).expect("erreur message");
    com_port.write(message.as_bytes())
        .map_err(|e| format!("erreur : {e}"))?;

    let mut serial_hash: Vec<u8> = vec![0; 1024];
    let n = com_port.read(serial_hash.as_mut_slice())
        .map_err(|e| format!("erreur : {e}"))?;
    
    let hash = String::from_utf8_lossy(&serial_hash[..n]).to_string();
    Ok(hash)
}
