use crate::BindKeyApp;
use crate::protocol::Command;
use egui::Ui;
use serde_json::to_string;
use serialport::SerialPortType;
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

pub fn get_bindkey(
    app: &mut BindKeyApp,
    ui: &mut egui::Ui,
    cmd: Command,
) -> Result<String, String> {
    app.enroll_status = "🔌 Recherche de la clé USB...".to_string();

    println!("{:?}", app.enroll_role);

    let bypass_usb = true;

    app.enroll_status = if bypass_usb {
        "🛠️ MODE SIMULATION : Bypass USB activé...".to_string()
    } else {
        "🔌 Recherche de la clé USB...".to_string()
    };

    let resultat_usb: Result<String, String>;

    if bypass_usb {
        println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
        resultat_usb = Ok(r#"{"status": "SUCCESS", "uid": "SIMULATED-BK-999", "public_key": "simulated-key-xyz"}"#
        .to_string());
    } else {
        app.devices.clear();
        if let Ok(liste_devices) = serialport::available_ports() {
            for device in liste_devices {
                if let SerialPortType::UsbPort(_) = device.port_type {
                    app.devices.push(device);
                };
            }
        }

        if let Some(device) = app.devices.first() {
            resultat_usb = send_command_bindkey(&device.port_name, cmd);
        } else {
            resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
        }
    }
    return resultat_usb;
}
