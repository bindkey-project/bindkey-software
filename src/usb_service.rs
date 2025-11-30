use crate::protocol::Command; // On importe ton enum
use serialport::{SerialPortType, available_ports};
use std::io::{self, Write};

pub struct DeviceInfo {
    pub port_name: String,
    pub description: String,
}

pub fn list_available_ports() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            match p.port_type {
                serialport::SerialPortType::UsbPort(info) => {
                    let desc = format!("USB: VID:{:04x} PID:{:04x}", info.vid, info.pid);
                    devices.push(DeviceInfo {
                        port_name: p.port_name,
                        description: desc,
                    });
                }
                _ => {}
            }
        }
    }
    if devices.is_empty() {
        println!("DEBUG: Mode Simulation activé (Aucun matériel détecté)");
        devices.push(DeviceInfo {
            port_name: "/dev/ttyUSB_SIMU".to_string(),
            description: "BindKey Virtual Device (Simulation)".to_string(),
        });
    }
    devices
}

pub fn send_command(port_name: &str, cmd: Command) -> Result<String, String> {
    let json_cmd = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;

    if port_name.contains("SIMU") {
        println!(" [SIMULATION USB] Envoi vers {}: {}", port_name, json_cmd);

        let reponse_simulee = match cmd {
            Command::GetStatus => r#"{"status": "LOCKED", "version": "1.0.0"}"#,

            Command::StartEnrollment { .. } => r#"{"status": "WAITING_FINGER", "led": "BLINKING"}"#,

            Command::Unlock { .. } => r#"{"status": "UNLOCKED", "drive": "MOUNTED"}"#,

            _ => "{}",
        };

        return Ok(reponse_simulee.to_string());
    } else {
        // --- VRAI CODE (Pour plus tard) ---
        // C'est ici qu'on ouvrira le vrai port série avec serialport::new()...
        println!(" [VRAI USB] Tentative d'écriture sur {}...", port_name);
        Err("Vrai matériel non connecté".to_string())
    }
}
