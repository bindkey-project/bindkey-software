use crate::{
    BindKeyApp, VolumeInfo,
    protocol::{self, VolumeCreationPayload},
    usb_service::{get_bindkey, send_command_bindkey},
};
use eframe::egui;
use serialport::SerialPortType;
use sysinfo::Disks;

pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.heading("Volumes et chiffrement");
    ui.label("Branchez une clé USB vierge pour créer un volume sécurisé.");

    ui.horizontal(|ui| {
        if ui
            .button("Afficher le périphérique branché sur la bindkey")
            .clicked()
        {
            app.volume_status = "🔌 Recherche de la clé USB...".to_string();

    let bypass_usb = true;

    app.volume_status = if bypass_usb {
        "🛠️ MODE SIMULATION : Bypass USB activé...".to_string()
    } else {
        "🔌 Recherche de la clé USB...".to_string()
    };

    let resultat_usb: Result<String, String>;

    if bypass_usb {
        println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
        resultat_usb = Ok(r#"{"status": "SUCCESS", "device_name": "Clé USB de Willy le GOAT", "device_size": "64", "device_available_size": "45"}"#
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
            resultat_usb = send_command_bindkey(&device.port_name, protocol::Command::GetVolume);
        } else {
            resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
        }
    }
            match resultat_usb {
                Ok(received_data) => {
                    if let Ok(json_value) =
                        serde_json::from_str::<serde_json::Value>(&received_data)
                    {
                        if json_value["status"] == "SUCCESS" {
                            app.device_name = json_value["device_name"]
                                .as_str()
                                .unwrap_or("Unknown Name")
                                .to_string();

                            app.device_size = json_value["device_size"]
                                .as_str()
                                .unwrap_or("Unknown Size")
                                .to_string();

                            if let Some(str_val) = json_value["device_available_size"].as_str() {
                                app.device_available_space = str_val.parse::<u32>().unwrap_or(0);
                            } else {
                                app.device_available_space = 0;
                            }
                          app.volume_status = "Périphérique trouvé !".to_string();
                        } else {
                            app.volume_status = "Erreur : Status non SUCCESS".to_string();
                        }
                    }
                }
                Err(e) => {
                    app.volume_status = format!("{}", e);
                }
            }

        }
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    if !app.device_name.is_empty() {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Nom du périphérique :");
                    ui.strong(&app.device_name);
                });
                ui.horizontal(|ui| {
                    ui.label("Espace total sur le périphérique :");
                    ui.monospace(format!("{} Go", app.device_size));
                });
                ui.horizontal(|ui| {
                    ui.label("Espace disponible sur le périphérique :");
                    ui.monospace(format!("{} Go", app.device_available_space));
                });
            });
            ui.add_space(5.0);
        });
    }

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    let conditions = !app.device_name.is_empty() && !app.device_available_space > 0;

    ui.group(|ui| {
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
            if ui.button("Créer un volume chiffré").clicked() {
                //app.creation_state.is_open = true;
                //app.creation_state.status = String::new();
            }
        })
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        if !app.volume_status.is_empty() {
            ui.colored_label(egui::Color32::BLUE, &app.volume_status);
        }
    });
}
/*    if app.creation_state.is_open {
        // On ouvre une petite fenêtre au dessus du reste
        egui::Window::new("Assistant de Création de Volume")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.set_width(400.0);

                ui.heading("1. Choisissez le périphérique cible");
                ui.colored_label(egui::Color32::RED, "⚠️ ATTENTION : Le disque sera entièrement effacé !");

                if !app.detected_volumes.is_empty() {
                    let selected_idx = app.creation_state.selected_disk_index;
                    let safe_idx = if selected_idx < app.detected_volumes.len() { selected_idx } else { 0 };

                    let selected_vol = &app.detected_volumes[safe_idx];

                    egui::ComboBox::from_label("Disque cible")
                        .selected_text(format!("{} ({})", selected_vol.name, selected_vol.mount_point))
                        .show_ui(ui, |ui| {
                            for (i, vol) in app.detected_volumes.iter().enumerate() {
                                let label = format!("{} ({}) - {} Go", vol.name, vol.mount_point, bytes_to_gb(vol.total_space));
                                ui.selectable_value(&mut app.creation_state.selected_disk_index, i, label);
                            }
                        });
                } else {
                    ui.label("Aucun disque disponible.");
                }
                ui.add_space(10.0);

                ui.heading("2. Configuration");

                ui.label("Nom du volume :");
                ui.text_edit_singleline(&mut app.creation_state.volume_name);

                ui.add_space(5.0);

                ui.label(format!("Taille du volume : {} Go", app.creation_state.volume_size_gb));
                ui.add(egui::Slider::new(&mut app.creation_state.volume_size_gb, 1..=64).text("Go"));

                ui.horizontal(|ui| {
                    if ui.button("Annuler").clicked() {
                        app.creation_state.is_open = false;
                    }

                    let btn = egui::Button::new(" Formater et Chiffrer avec BindKey").fill(egui::Color32::DARK_RED);
                    if ui.add(btn).clicked() {
                        send_create_order(app, ui.ctx().clone());
                    }
                });

                if !app.creation_state.status.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::BLUE,&app.creation_state.status);
                }
            });
    }
}


fn bytes_to_gb(bytes: u64) -> u64 {
    bytes / 1024 / 1024 / 1024
}
*/
