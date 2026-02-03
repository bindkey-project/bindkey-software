use std::time::Duration;

use crate::protocol::protocol::{ApiMessage, VolumeInitInfo, VolumeInitResponse};
use crate::protocol::share_protocol::{self, SuccessData, UsbResponse};
use eframe::egui;

pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    let usb_connected = app.usb_connected;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.set_max_width(600.0);

            ui.add_space(20.0);
            ui.heading("💾 Volumes & Chiffrement");
            ui.add_space(10.0);
            ui.label("Gérez vos espaces sécurisés directement depuis votre BindKey.");
            ui.add_space(30.0);

            let frame_style = egui::Frame::none()
                .fill(ui.visuals().window_fill())
                .rounding(10.0)
                .stroke(ui.visuals().window_stroke())
                .inner_margin(20.0);

            frame_style.show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.heading("1. Détection");
                ui.add_space(10.0);
                ui.label("Branchez votre clé et lancez l'analyse.");
                ui.add_space(15.0);

                ui.add_enabled_ui(usb_connected, |ui| {
                    let btn_scan = egui::Button::new("🔍 Analyser le périphérique USB")
                        .min_size(egui::vec2(250.0, 40.0));

                    if ui.add(btn_scan).clicked() {
                        app.volume_status = "🔌 Recherche des infos du disque...".to_string();

                        let clone_sender = app.sender.clone();
                        let clone_port_name = app.current_port_name.clone();
                        let bypass_usb = true;

                        tokio::spawn(async move {
                            let resultat_usb: Result<UsbResponse, String>;

                            if bypass_usb {
                                println!(">> SIMULATION SCAN DISQUE");
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                resultat_usb = Ok(UsbResponse::Success(SuccessData::DeviceInfo {
                                    device_name: "Clé USB de Willy".to_string(),
                                    device_size: 64,
                                    device_available_size: 45,
                                    mount_id: 2,
                                }));
                            } else {
                                if !clone_port_name.is_empty() {
                                    match serialport::new(&clone_port_name, 115200)
                                        .timeout(Duration::from_secs(2))
                                        .open()
                                    {
                                        Ok(mut port) => {
                                            resultat_usb = send_command(
                                                &mut port,
                                                share_protocol::Command::GetVolume,
                                            );
                                        }
                                        Err(e) => {
                                            resultat_usb =
                                                Err(format!("Erreur d'ouvertur port: {}", e));
                                        }
                                    }
                                } else {
                                    resultat_usb = Err("Aucune Bindkey détectée".to_string());
                                }
                            }

                            match resultat_usb {
                                Ok(data) => {
                                    let _ = clone_sender.send(ApiMessage::VolumeInfoReceived(data));
                                }
                                Err(e) => {
                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                        format!("Erreur Scan: {}", e),
                                    ));
                                }
                            }
                        });
                    }
                });
                if !usb_connected {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("Veuillez brancher la Bindkey")
                            .color(egui::Color32::RED)
                            .size(12.0),
                    );
                }
            });

            ui.add_space(20.0);

            if !app.device_name.is_empty() {
                frame_style.show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    ui.horizontal(|ui| {
                        ui.heading("📊 Informations du disque");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(egui::Color32::GREEN, "● Connecté");
                        });
                    });

                    ui.separator();
                    ui.add_space(10.0);

                    egui::Grid::new("disk_info_grid")
                        .num_columns(2)
                        .spacing([40.0, 10.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Nom du périphérique :");
                            ui.strong(&app.device_name);
                            ui.end_row();

                            ui.label("Espace Total :");
                            ui.colored_label(
                                egui::Color32::BLUE,
                                format!("{} Go", app.device_size),
                            );
                            ui.end_row();

                            ui.label("Espace Disponible :");
                            let color = if app.device_available_space < 5 {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };
                            ui.colored_label(color, format!("{} Go", app.device_available_space));
                            ui.end_row();
                        });
                });

                ui.add_space(20.0);
            }

            let conditions = !app.device_name.is_empty() && app.device_available_space > 0;

            if conditions {
                frame_style.show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    ui.heading("🛠️ Création d'un Volume");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Nom du volume :");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.volume_created_name)
                                .min_size(egui::vec2(200.0, 20.0)),
                        );
                    });

                    ui.add_space(10.0);

                    let max_size = if app.device_available_space > 0 {
                        app.device_available_space
                    } else {
                        1
                    };
                    ui.horizontal(|ui| {
                        ui.label("Taille allouée :");
                        ui.add(
                            egui::Slider::new(&mut app.volume_created_size, 1..=max_size)
                                .text("Go"),
                        );
                    });

                    ui.add_space(20.0);

                    ui.add_enabled_ui(usb_connected, |ui| {
                        let btn_create = egui::Button::new(" Créer le volume chiffré")
                            .min_size(egui::vec2(250.0, 45.0));

                        if ui.add(btn_create).clicked() {
                            let bypass_usb = true;

                            app.volume_status = if bypass_usb {
                                "🛠️ SIMULATION : Init Serveur...".to_string()
                            } else {
                                "⏳ Initialisation sur le serveur...".to_string()
                            };

                            let clone_sender = app.sender.clone();
                            let clone_volume_name = app.volume_created_name.clone();
                            let clone_volume_size = app.volume_created_size;
                            let clone_device_name = app.device_name.clone();
                            let clone_mount_id = app.mount_id;
                            let clone_url = app.config.api_url.clone();
                            let clone_auth_token = app.server_token.clone();
                            let clone_port_name = app.current_port_name.clone();

                            tokio::spawn(async move {
                                let client = reqwest::Client::new();
                                let url = format!("{}/verify_volume", clone_url);
                                let payload = VolumeInitInfo {
                                    name: clone_volume_name.clone(),
                                    disk_id: clone_device_name,
                                };

                                let resultat = client
                                    .post(url)
                                    .json(&payload)
                                    .bearer_auth(clone_auth_token)
                                    .send()
                                    .await;

                                match resultat {
                                    Ok(response) if response.status().is_success() => {
                                        if let Ok(data) =
                                            response.json::<VolumeInitResponse>().await
                                        {

                                            if data.exists {
                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur le vomue '{}' existe déjà", clone_volume_name)));
                                                return;
                                            }


                                            let serveur_volume_id = data.volume_id;

                                                let _ = clone_sender.send(
                                                ApiMessage::VolumeCreationStatus("Nom validé par le serveur. Écriture sur la clé".to_string())
                                            );

                                            let resultat_usb: Result<UsbResponse, String>;

                                            if bypass_usb {
                                                println!(
                                                    ">> SIMULATION CREATION avec ID: {}",
                                                    serveur_volume_id
                                                );
                                                tokio::time::sleep(std::time::Duration::from_secs(
                                                    2,
                                                ))
                                                .await;
                                                resultat_usb = Ok(UsbResponse::Success(
                                                    SuccessData::VolumeCreated {
                                                        encrypted_key: "SIMULATED-KEY-XYZ-999"
                                                            .to_string(),
                                                        volume_id: serveur_volume_id.clone(),
                                                    },
                                                ));
                                            } else {
                                                if !clone_port_name.is_empty() {
                                                    match serialport::new(&clone_port_name, 115200)
                                                    .timeout(Duration::from_secs(2))
                                                    .open() {
                                                        Ok(mut port) => {
                                                            let volumepayload =
                                                        share_protocol::VolumeCreationPayload {
                                                            volume_name: clone_volume_name,
                                                            size_gb: clone_volume_size,
                                                            volume_id: serveur_volume_id,
                                                            mount_id: clone_mount_id,
                                                        };

                                                        resultat_usb = send_command(
                                                            &mut port,
                                                            share_protocol::Command::CreateVolume(volumepayload)
                                                        );
                                                        },
                                                        Err(e) => {
                                                            resultat_usb = Err(format!("Impossible d'ouvrir le port: {}", e));
                                                        }
                                                    }
                                                } else {
                                                    resultat_usb =
                                                        Err("Aucune Bindkey détectée".to_string());
                                                }
                                            }

                                            match resultat_usb {
                                                Ok(data) => {
                                                    let _ = clone_sender.send(
                                                        ApiMessage::VolumeCreationSuccess(data),
                                                    );
                                                }
                                                Err(e) => {
                                                    let _ = clone_sender.send(
                                                        ApiMessage::VolumeCreationStatus(format!(
                                                            " Erreur USB: {}",
                                                            e
                                                        )),
                                                    );
                                                }
                                            }
                                        } else {
                                            let _ = clone_sender.send(
                                                ApiMessage::VolumeCreationStatus(
                                                    " Erreur lecture réponse serveur".to_string(),
                                                ),
                                            );
                                        }
                                    }
                                    Ok(response) => {
                                        let _ =
                                            clone_sender.send(ApiMessage::VolumeCreationStatus(
                                                format!(" Refus serveur: {}", response.status()),
                                            ));
                                    }
                                    Err(e) => {
                                        let _ =
                                            clone_sender.send(ApiMessage::VolumeCreationStatus(
                                                format!(" Erreur Réseau: {}", e),
                                            ));
                                    }
                                }
                            });
                        }
                    });
                    if !usb_connected {
                        ui.label(
                            egui::RichText::new("Clé déconnectée")
                                .color(egui::Color32::RED)
                                .size(12.0),
                        );
                    }
                });
            }

            ui.add_space(20.0);

            if !app.volume_status.is_empty() {
                let color = if app.volume_status.contains("Erreur")
                    || app.volume_status.contains("Refus")
                {
                    egui::Color32::from_rgb(255, 100, 100)
                } else {
                    egui::Color32::from_rgb(100, 200, 255)
                };
                ui.colored_label(color, &app.volume_status);
            }
        });
    });
}
