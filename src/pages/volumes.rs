use crate::share_protocol::VolumeCreationPayload;
use crate::{BindKeyApp, protocol::ApiMessage, share_protocol, usb_service::send_command_bindkey};
use eframe::egui;
use serialport::SerialPortType;

pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.heading("Volumes et chiffrement");
    ui.label("Branchez une clé USB vierge pour créer un volume sécurisé.");

    ui.horizontal(|ui| {
        if ui.button("🔍 Afficher le périphérique branché sur la bindkey").clicked() {

            app.volume_status = "🔌 Recherche des infos du disque...".to_string();

            let clone_sender = app.sender.clone();
            let bypass_usb = true;

            tokio::spawn(async move {
                let resultat_usb: Result<String, String>;

                if bypass_usb {
                    println!(">> SIMULATION SCAN DISQUE");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    resultat_usb = Ok(r#"{"status": "SUCCESS", "device_name": "Clé USB de Willy", "device_size": "64", "device_available_size": "45"}"#.to_string());
                } else {
                    let mut port_name = String::new();
                    if let Ok(ports) = serialport::available_ports() {
                        for p in ports {
                            if let SerialPortType::UsbPort(_) = p.port_type {
                                port_name = p.port_name;
                                break;
                            }
                        }
                    }

                    if !port_name.is_empty() {
                        resultat_usb = send_command_bindkey(&port_name, share_protocol::Command::GetVolume);
                    } else {
                        resultat_usb = Err("Aucune Bindkey détectée".to_string());
                    }
                }

                match resultat_usb {
                    Ok(data) => {
                        let _ = clone_sender.send(ApiMessage::VolumeInfoReceived(data));
                    }
                    Err(e) => {
                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur Scan: {}", e)));
                    }
                }
            });
        }
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    if !app.device_name.is_empty() {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.heading("📊 Informations du disque");
                ui.horizontal(|ui| {
                    ui.label("Nom :");
                    ui.strong(&app.device_name);
                });
                ui.horizontal(|ui| {
                    ui.label("Total :");
                    ui.monospace(format!("{} Go", app.device_size));
                });
                ui.horizontal(|ui| {
                    ui.label("Disponible :");
                    let color = if app.device_available_space < 5 {
                        egui::Color32::RED
                    } else {
                        egui::Color32::GREEN
                    };
                    ui.colored_label(color, format!("{} Go", app.device_available_space));
                });
            });
            ui.add_space(5.0);
        });
    }

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    let conditions = !app.device_name.is_empty() && app.device_available_space > 0;

    ui.group(|ui| {
        ui.heading("🔒 Création du Volume");
        ui.add_space(5.0);

        ui.label("Nom du volume :");
        ui.text_edit_singleline(&mut app.volume_created_name);
        ui.add_space(5.0);

        let max_size = if app.device_available_space > 0 {
            app.device_available_space
        } else {
            1
        };
        ui.label(format!("Taille du volume : {} Go", app.volume_created_size));
        ui.add(egui::Slider::new(&mut app.volume_created_size, 1..=max_size).text("Go"));

        ui.add_enabled_ui(conditions, |ui| {
            if ui.button("🚀 Créer un volume chiffré").clicked() {
                let bypass_usb = true;

                app.volume_status = if bypass_usb {
                    "🛠️ SIMULATION : Création en cours...".to_string()
                } else {
                    "⏳ Création du volume en cours... Ne débranchez rien.".to_string()
                };

                let clone_sender = app.sender.clone();
                let clone_volume_name = app.volume_created_name.clone();
                let clone_volume_size = app.volume_created_size.clone();

                tokio::spawn(async move {
                    let resultat_usb: Result<String, String>;

                    if bypass_usb {
                        println!(">> SIMULATION CREATION");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        resultat_usb = Ok(
                            r#"{"status": "SUCCESS", "encrypted_key": "SIMULATED-Key-999"}"#
                                .to_string(),
                        );
                    } else {
                        let mut port_name = String::new();
                        if let Ok(ports) = serialport::available_ports() {
                            for p in ports {
                                if let SerialPortType::UsbPort(_) = p.port_type {
                                    port_name = p.port_name;
                                    break;
                                };
                            }
                        }

                        if !port_name.is_empty() {
                            let volumepayload = VolumeCreationPayload {
                                volume_name: clone_volume_name,
                                size_gb: clone_volume_size,
                            };
                            resultat_usb = send_command_bindkey(
                                &port_name,
                                share_protocol::Command::CreateVolume(volumepayload),
                            );
                        } else {
                            resultat_usb = Err("Aucune Bindkey détectée".to_string());
                        }
                    }

                    match resultat_usb {
                        Ok(data) => {
                            let _ = clone_sender.send(ApiMessage::VolumeCreationSuccess(data));
                        }
                        Err(e) => {
                            let _ = clone_sender
                                .send(ApiMessage::VolumeCreationStatus(format!("Erreur: {}", e)));
                        }
                    }
                });
            }
        });

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            if !app.volume_status.is_empty() {
                ui.colored_label(egui::Color32::BLUE, &app.volume_status);
            }
        });
    });
}
