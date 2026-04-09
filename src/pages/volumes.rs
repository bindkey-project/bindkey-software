use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::BindKeyApp;
use crate::protocol::protocol::{
    ApiMessage, LsblkOutput, UsbDevice, VolumeInfo, VolumeInitInfo, VolumeInitResponse, VolumeTab,
};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use eframe::egui;

pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    let usb_connected = app.usb_connected;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.set_max_width(600.0);

            ui.add_space(20.0);
            ui.heading("💾 Volumes & Chiffrement");
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.active_tab, VolumeTab::Dashboard, "Dashboard");
                ui.selectable_value(&mut app.active_tab, VolumeTab::Gestion, "Gestion des Volumes");
                ui.selectable_value(&mut app.active_tab, VolumeTab::Formatage, "Formatage clé USB");
            });
            ui.separator();
            ui.add_space(10.0);

            ui.label("Gérez vos espaces sécurisés directement depuis votre BindKey.");
            ui.add_space(30.0);

            match app.active_tab {
                // =================================================================
                // ONGLET 1 : DASHBOARD (TABLEAU DE BORD)
                // =================================================================
                VolumeTab::Dashboard => {
                    let frame_style = egui::Frame::none()
                        .fill(ui.visuals().window_fill())
                        .rounding(10.0)
                        .stroke(ui.visuals().window_stroke())
                        .inner_margin(20.0);

                    frame_style.show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.horizontal(|ui| {
                            ui.heading("Vos volumes sécurisés");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Bouton pour rafraîchir la liste
                                if ui.button("🔄 Actualiser").clicked() {
                                    if let Ok(output) = Command::new("lsblk")
                                        .args(&["-J", "-b", "-o", "NAME,MODEL,SIZE,TRAN,FSTYPE,PTTYPE,MOUNTPOINT,LABEL"])
                                        .output()
                                    {
                                        let ouput_str = String::from_utf8_lossy(&output.stdout);
                                        if let Ok(parsed) = serde_json::from_str::<LsblkOutput>(&ouput_str) {
                                            let mut extracted_volumes = Vec::new();
                                            for disk in parsed.blockdevices {
                                                if disk.tran.as_deref() == Some("usb") {
                                                    if let Some(children) = disk.children {
                                                        for part in children {
                                                            // On ignore les très petites partitions
                                                            if part.size < 10_000_000 { continue; }

                                                            let total_gb = part.size as f64 / 1_073_741_824.0;
                                                            extracted_volumes.push(VolumeInfo {
                                                                name: part.label.unwrap_or_else(|| part.name.clone()),
                                                                device_path: format!("/dev/{}", part.name),
                                                                total_space_gb: (total_gb * 10.0).round() / 10.0,
                                                                is_mounted: part.mountpoint.is_some(),
                                                                mount_point: part.mountpoint,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                            app.dashboard_volumes = extracted_volumes;
                                        }
                                    }
                                }
                            });
                        });

                        ui.add_space(20.0);

                        // Affichage dynamique des cartes
                        if app.dashboard_volumes.is_empty() {
                            ui.label(egui::RichText::new("Aucun volume BindKey détecté. Branchez votre clé et cliquez sur Actualiser.").italics());
                        } else {
                            for vol in &app.dashboard_volumes {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {

                                        // Info à gauche
                                        ui.vertical(|ui| {
                                            ui.strong(&vol.name);
                                            ui.label(format!("Taille : {} Go", vol.total_space_gb));
                                            ui.label(format!("Chemin : {}", vol.device_path));
                                        });

                                        // Boutons à droite
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {

                                            if vol.is_mounted {
                                                ui.colored_label(egui::Color32::GREEN, "🔓 Monté");

                                                if ui.button("Démonter").clicked() {
                                                    let clone_path = vol.device_path.clone();
                                                    let clone_sender = app.sender.clone();

                                                    tokio::spawn(async move {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Démontage de {}...", clone_path)));

                                                        let umount_status = Command::new("/usr/bin/udisksctl")
                                                            .args(["unmount", "-b", &clone_path])
                                                            .output();

                                                        if let Ok(output) = umount_status {
                                                            if output.status.success() {
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Volume démonté avec succès.".to_string()));
                                                            } else {
                                                                let err = String::from_utf8_lossy(&output.stderr);
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur démontage: {}", err)));
                                                            }
                                                        }
                                                    });
                                                }

                                                if ui.button("📂 Ouvrir").clicked() {
                                                    if let Some(mount_path) = &vol.mount_point {
                                                        let _ = Command::new("xdg-open").arg(mount_path).spawn();
                                                    }
                                                }
                                            } else {
                                                ui.colored_label(egui::Color32::RED, "🔒 Verrouillé / Non Monté");

                                                if ui.button("Monter le volume").clicked() {
                                                    let clone_path = vol.device_path.clone();
                                                    let clone_sender = app.sender.clone();

                                                    tokio::spawn(async move {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Montage en cours... (Assurez-vous d'avoir validé votre empreinte)".to_string()));

                                                        let mount_status = Command::new("/usr/bin/udisksctl")
                                                            .args(["mount", "-b", &clone_path])
                                                            .output();

                                                        if let Ok(output) = mount_status {
                                                            if output.status.success() {
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Volume monté avec succès !".to_string()));
                                                            } else {
                                                                let err = String::from_utf8_lossy(&output.stderr);
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur OS au montage: {}", err)));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        });
                                    });
                                });
                                ui.add_space(10.0);
                            }
                        }
                    });
                },

                // =================================================================
                // ONGLET 2 : GESTION DES VOLUMES
                // =================================================================
                VolumeTab::Gestion => {
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

                                let output = Command::new("lsblk")
                                    .args(&["-J", "-b", "-o", "NAME,MODEL,SIZE,TRAN,FSTYPE,PTTYPE"])
                                    .output()
                                    .expect("Erreur lsblk");

                                let ouput_str = String::from_utf8_lossy(&output.stdout);
                                let parsed: LsblkOutput = serde_json::from_str(&ouput_str).unwrap_or(LsblkOutput { blockdevices: vec![] });

                                let mut devices = Vec::new();
                                let mut target_total_gb = 0.0;
                                let mut target_free_gb = 0.0;
                                let mut target_name = String::new();

                                for disk in parsed.blockdevices {
                                    if let Some(tran) = disk.tran {
                                        if tran.trim() == "usb" {
                                            let model = disk.model.unwrap_or("Inconnu".to_string());

                                            if !model.to_uppercase().contains("BINDKEY") {
                                                continue;
                                            }

                                            let size_gb = (disk.size as f64) / (1024.0 * 1024.0 * 1024.0);
                                            let label = format!("{} ({:.1} GB) - /dev/{}", model.trim(), size_gb, disk.name);

                                            let mut used_bytes = 0;
                                            let mut partitions_paths = Vec::new();

                                            if let Some(children) = &disk.children {
                                                for child in children {
                                                    used_bytes += child.size;
                                                    partitions_paths.push(format!("/dev/{}", child.name));
                                                }
                                            }

                                            let free_bytes = if disk.fstype.is_some() && disk.children.is_none() {
                                                0
                                            } else if disk.pttype.is_none() {
                                                0
                                            } else {
                                                disk.size.saturating_sub(used_bytes)
                                            };

                                            target_total_gb = (disk.size as f64) / (1024.0 * 1024.0 * 1024.0);
                                            target_free_gb = (free_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                                            target_name = model.trim().to_string();

                                            devices.push(UsbDevice {
                                                path: format!("/dev/{}", disk.name),
                                                display_name: label,
                                                partitions: partitions_paths,
                                            });
                                        }
                                    }
                                }

                                app.available_devices = devices;

                                if app.available_devices.is_empty() {
                                    app.volume_status = "Aucune clé USB détectée par le système.".to_string();
                                    return;
                                }

                                let clone_sender = app.sender.clone();
                                let clone_port_name = app.current_port_name.clone();
                                let bypass_usb = true;

                                let clone_total_gb = (target_total_gb * 100.0).round() / 100.0;
                                let clone_free_gb = (target_free_gb * 100.0).round() / 100.0;
                                let clone_device_name = if target_name.is_empty() { "BINDKEY".to_string() } else { target_name };

                                tokio::spawn(async move {
                                    let resultat_usb: Result<UsbResponse, String>;

                                    if bypass_usb {
                                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                        resultat_usb = Ok(UsbResponse::Success(SuccessData::DeviceInfo {
                                            device_name: clone_device_name,
                                            device_size: clone_total_gb,
                                            device_available_size: clone_free_gb
                                        }));
                                    } else {
                                        if !clone_port_name.is_empty() {
                                            match serialport::new(&clone_port_name, 115200).timeout(Duration::from_secs(2)).open() {
                                                Ok(mut port) => {
                                                    let _ = port.write_data_terminal_ready(true);
                                                    let _ = port.write_request_to_send(true);
                                                    tokio::time::sleep(Duration::from_secs(2)).await;

                                                    match crate::usb_service::send_text_command(&mut *port, "getdevice") {
                                                        Ok(map) => {
                                                            let name_str = map.get("DN").cloned().unwrap_or(clone_device_name);
                                                            resultat_usb = Ok(UsbResponse::Success(SuccessData::DeviceInfo {
                                                                device_name: name_str,
                                                                device_size: clone_total_gb,
                                                                device_available_size: clone_free_gb
                                                            }));
                                                        },
                                                        Err(e) => resultat_usb = Err(format!("Echec communication série: {}", e))
                                                    }
                                                }
                                                Err(e) => resultat_usb = Err(format!("Erreur d'ouverture port: {}", e))
                                            }
                                        } else {
                                            resultat_usb = Err("Aucune Bindkey détectée".to_string());
                                        }
                                    }

                                    match resultat_usb {
                                        Ok(data) => { let _ = clone_sender.send(ApiMessage::VolumeInfoReceived(data)); }
                                        Err(e) => { let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur Scan: {}", e))); }
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

                    ui.add_space(40.0);
                    ui.add_space(20.0);

                    if app.available_devices.len() > 1 {
                        ui.add_space(20.0);
                        let frame_style = egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(255, 0, 0, 20))
                            .rounding(10.0)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::RED))
                            .inner_margin(20.0);

                        frame_style.show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.heading("Sécurité stricte activée");
                            });
                            ui.add_space(10.0);
                            ui.label(
                            egui::RichText::new("Plusieurs périphériques USB ont été détectés.")
                                    .color(egui::Color32::RED)
                                    .strong(),
                            );
                            ui.label("Pour éviter toute erreur de formatage ou de chiffrement, veuillez débrancher les autres clés et ne conserver que la BindKey cible, puis relancez l'analyse.");
                        });
                    }
                    else if app.available_devices.len() == 1 {
                        if !app.device_name.is_empty() {
                            frame_style.show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                ui.horizontal(|ui| {
                                    ui.heading("Informations du disque");
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
                                        ui.colored_label(egui::Color32::BLUE, format!("{} Go", app.device_size));
                                        ui.end_row();

                                        ui.label("Espace Disponible :");
                                        let color = if app.device_available_space < 5.0 { egui::Color32::RED } else { egui::Color32::GREEN };
                                        ui.colored_label(color, format!("{} Go", app.device_available_space));
                                        ui.end_row();
                                    });
                            });

                            ui.add_space(20.0);
                        }

                        if !app.device_name.is_empty() && app.device_available_space == 0.0 && app.device_size > 0.0 {
                            ui.add_space(10.0);

                            let has_partitions = !app.available_devices.is_empty() && !app.available_devices[0].partitions.is_empty();

                            if !has_partitions {
                                ui.label(
                                egui::RichText::new("⚠️ Clé non initialisée ou mal formatée.")
                                     .color(egui::Color32::from_rgb(255, 140, 0))
                                     .strong()
                                );
                                ui.label("Veuillez utiliser l'onglet 'Formatage clé USB' pour réinitialiser la clé à zéro et recréer la table de partition avant d'ajouter des volumes.");
                            } else {
                                ui.label(
                                egui::RichText::new("ℹ️ Espace non alloué insuffisant.")
                                    .color(egui::Color32::LIGHT_BLUE)
                                    .strong()
                                );
                                ui.label("La clé est remplie par les volumes existants. Il n'y a plus d'espace libre pour en créer un nouveau.");
                            }
                            ui.add_space(10.0);
                        }

                        let conditions = !app.device_name.is_empty() && app.device_available_space > 0.0;

                        if conditions {
                            frame_style.show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                ui.heading("Création d'un Volume");
                                ui.add_space(10.0);

                                ui.horizontal(|ui| {
                                    ui.label("Nom du volume :");
                                    ui.add(egui::TextEdit::singleline(&mut app.volume_created_name).min_size(egui::vec2(200.0, 20.0)));
                                });

                                ui.add_space(10.0);

                                let max_size = if app.device_available_space > 0.0 { app.device_available_space } else { 1.0 };
                                ui.horizontal(|ui| {
                                    ui.label("Taille allouée :");
                                    ui.add(egui::Slider::new(&mut app.volume_created_size, 1..=max_size as u32).text("Go"));
                                });

                                ui.add_space(20.0);

                                ui.add_enabled_ui(usb_connected, |ui| {
                                    let btn_create = egui::Button::new(" Créer le volume chiffré")
                                        .min_size(egui::vec2(250.0, 45.0));

                                    if ui.add(btn_create).clicked() {
                                        let bypass_usb_ui = false; // Valeur par défaut pour l'UI

                                        app.volume_status = if bypass_usb_ui {
                                            "SIMULATION : Init Serveur...".to_string() 
                                        } else {
                                            "Initialisation sur le serveur...".to_string() 
                                        };

                                        let clone_sender = app.sender.clone();
                                        let clone_volume_name = app.volume_created_name.clone();
                                        let clone_volume_size = app.volume_created_size;
                                        let clone_device_name = app.device_name.clone();
                                        let clone_url = app.config.api_url.clone();
                                        let clone_auth_token = app.server_token.clone();
                                        let clone_port_name = app.current_port_name.clone();
                                        let clone_api_client = app.api_client.clone();
                                        let clone_device_path = app.available_devices[0].path.clone();

                                        tokio::spawn(async move {
                                            let bypass_server = true;
                                            // =========================================================
                                            // 1. VÉRIFICATION SERVEUR (Inchangé)
                                            // =========================================================
                                            let serveur_volume_id = if bypass_server {
                                                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("[SIMU] Nom validé. Calcul des secteurs...".to_string()));
                                                "ID-SIMULATION-12345".to_string()
                                            } else {
                                                let url = format!("{}/verify_volume", clone_url);
                                                let payload = VolumeInitInfo {
                                                    name: clone_volume_name.clone(),
                                                    disk_id: clone_device_name,
                                                };

                                                let resultat = clone_api_client.post(url).json(&payload).bearer_auth(clone_auth_token).send().await;

                                                match resultat {
                                                    Ok(response) if response.status().is_success() => {
                                                        if let Ok(data) = response.json::<VolumeInitResponse>().await {
                                                            if data.exists {
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur : le volume '{}' existe déjà", clone_volume_name)));
                                                                return;
                                                            }
                                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Nom validé par le serveur. Calcul des secteurs...".to_string()));
                                                            data.volume_id
                                                        } else {
                                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Erreur lecture réponse serveur".to_string()));
                                                            return;
                                                        }
                                                    }
                                                    Ok(response) => {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Refus serveur: {}", response.status())));
                                                        return;
                                                    }
                                                    Err(e) => {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur Réseau: {}", e)));
                                                        return;
                                                    }
                                                }
                                            };

                                            // =========================================================
                                            // 2. CRÉATION, COMMUNICATION USB & FORMATAGE (Tout-en-un)
                                            // =========================================================
                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Synchronisation matérielle et découpage...".to_string()));

                                            // Clonage des variables pour le thread bloquant
                                            let device_path_for_thread = clone_device_path.clone();
                                            let volume_name_for_thread = clone_volume_name.clone();
                                            let volume_id_for_thread = serveur_volume_id.clone();
                                            let port_name_for_thread = clone_port_name.clone();

                                            // On utilise spawn_blocking car create_and_format_partition exécute des commandes OS synchrones
                                            let partition_result = tokio::task::spawn_blocking(move || {
                                                // ⚠️ Remplace "crate::ton_module" par le chemin réel vers ta fonction
                                                create_and_format_partition(
                                                    &device_path_for_thread,
                                                    clone_volume_size as f64,
                                                    &volume_name_for_thread,
                                                    &volume_id_for_thread,
                                                    &port_name_for_thread,
                                                )
                                            }).await.unwrap_or_else(|e| Err(format!("Erreur critique du thread OS : {}", e)));

                                            // =========================================================
                                            // 3. RETOUR D'ÉTAT
                                            // =========================================================
                                            match partition_result {
                                                Ok((_start_sec, _end_sec, num_partition)) => {
                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("✅ Volume créé et synchronisé avec succès !".to_string()));

                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationSuccess(UsbResponse::Success(SuccessData::VolumeCreated {
                                                        volume_id: serveur_volume_id,
                                                        device_path: clone_device_path,
                                                        partition_number: num_partition,
                                                    })));
                                                },
                                                Err(e) => {
                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur système/USB : {}", e)));
                                                }
                                            }
                                        });
                                    }

                                    if !usb_connected {
                                        ui.label(
                                            egui::RichText::new("Clé déconnectée")
                                                .color(egui::Color32::RED)
                                                .size(12.0),
                                        );
                                    }
                                });
                            });
                        }
                    }
                    ui.add_space(20.0);

                    if !app.volume_status.is_empty() {
                        let color = if app.volume_status.contains("Erreur") || app.volume_status.contains("Refus") || app.volume_status.contains("❌") {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 200, 255)
                        };
                        ui.colored_label(color, &app.volume_status);
                    }
                },

                // =================================================================
                // ONGLET 3 : FORMATAGE BRUT
                // =================================================================
                VolumeTab::Formatage => {
                    ui.add_space(20.0);

                    if app.available_devices.len() == 1 {
                        let device = &app.available_devices[0];

                        ui.horizontal(|ui| {
                            ui.label("Périphérique cible : ");
                            ui.strong(&device.display_name);
                        });

                        ui.add_space(20.0);

                        let format_button = egui::Button::new("Réinitialiser la clé à zéro");

                        if ui.add(format_button).clicked() {
                            // 🚀 ON DÉCLENCHE LA TÂCHE ASYNCHRONE ICI
                            // (Assure-toi d'avoir le port name disponible, ex: app.current_port_name)
                            let _ = app.sender.send(ApiMessage::StartFormatBindKey {
                                device_path: device.path.clone(),
                                partitions: device.partitions.clone(),
                                port_name: app.current_port_name.clone(),
                            });
                        }
                    }
                    else if app.available_devices.len() > 1 {
                        ui.colored_label(egui::Color32::RED, "Sécurité : Plusieurs clés détectées. Veuillez n'en brancher qu'une seule pour le formatage.");
                    }
                    else {
                        ui.label("Branchez une Binkey et lancez l'analyse dans l'onglet 'Gestion des volumes' pour la formater");
                    }

                    ui.add_space(20.0);

                    // Affichage dynamique du statut
                    if !app.formatage_status.is_empty() {
                        let color = if app.formatage_status.contains("Erreur") || app.formatage_status.contains("Refus") {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 200, 255)
                        };
                        ui.colored_label(color, &app.formatage_status);
                    }

                    ui.add_space(20.0);

                    if !app.formatage_status.is_empty() {
                        let color = if app.formatage_status.contains("Erreur") || app.formatage_status.contains("Refus") {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 200, 255)
                        };
                        ui.colored_label(color, &app.formatage_status);
                    }
                }
            }
        });
    });
}

// =================================================================
// FONCTIONS UTILITAIRES SYSTÈME
// =================================================================

// ⚠️ N'oublie pas d'ajouter port_name dans les paramètres lors de l'appel !
fn create_and_format_partition(
    device_path: &str,
    size_gb: f64,
    volume_name: &str,
    volume_id: &str,
    port_name: &str,
) -> Result<(u64, u64, String), String> {
    let output = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/sbin/parted",
            "-s",
            "-m",
            device_path,
            "unit",
            "s",
            "print",
            "free",
        ])
        .output()
        .map_err(|e| format!("Erreur parted: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let target_sectors = (size_gb * 1024.0 * 1024.0 * 1024.0 / 512.0) as u64;
    let mut start_sector = None;
    let mut end_sector = None;

    for line in stdout.lines() {
        if line.ends_with("free;") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let start_str = parts[1].trim_end_matches('s');
                let size_str = parts[3].trim_end_matches('s');

                if let (Ok(start), Ok(size)) = (start_str.parse::<u64>(), size_str.parse::<u64>()) {
                    let mut actual_start = start;
                    if actual_start < 2048 {
                        actual_start = 2048;
                    }

                    if size >= target_sectors {
                        start_sector = Some(actual_start);
                        end_sector = Some(actual_start + target_sectors - 1);
                        break;
                    }
                }
            }
        }
    }

    let start = start_sector.ok_or("Espace libre insuffisant sur la clé.")?;
    let end = end_sector.unwrap();

    // =========================================================
    // 🟢 NOUVEAU BLOC : Envoi des secteurs à la BindKey
    // =========================================================
    {
        // On ouvre le port de manière synchrone (pas de .await ici car on est hors de tokio)
        let mut port = serialport::new(port_name, 115200)
            .timeout(Duration::from_secs(5))
            .open()
            .map_err(|e| format!("Impossible d'ouvrir le port USB : {}", e))?;

        let _ = port.write_data_terminal_ready(true);
        let _ = port.write_request_to_send(true);
        thread::sleep(Duration::from_millis(500));

        // 🎯 TON NOUVEAU FORMAT EXACT
        let cmd_sectors = format!(
            "volume_name={}\nvolume_id={}\nlba_start={}\nlba_end={}\n",
            volume_name, volume_id, start, end
        );

        let mut is_ready = false;
        let mut tentatives = 0;
        let max_tentatives = 5;

        // 🔄 TANT QUE ce n'est pas prêt ET qu'on n'a pas dépassé la limite
        while !is_ready && tentatives < max_tentatives {
            match crate::usb_service::send_text_command(&mut *port, &cmd_sectors) {
                Ok(map) => {
                    if map.get("STATUS").map(|val| val == "OK").unwrap_or(false) {
                        is_ready = true; // ✅ La puce a enregistré les LBA !
                    } else {
                        tentatives += 1;
                        thread::sleep(Duration::from_millis(1000));
                    }
                }
                Err(_) => {
                    tentatives += 1;
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }

        // 🛑 VÉRIFICATION FINALE AVANT FORMATAGE
        if !is_ready {
            return Err(
                "La BindKey n'a pas confirmé l'enregistrement des secteurs LBA. Formatage annulé."
                    .to_string(),
            );
        }
    } // Le port série se ferme proprement et automatiquement à la fin de ce bloc `{ ... }`
    // =========================================================

    let status_mkpart = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/sbin/parted",
            "-s",
            "-a",
            "optimal",
            device_path,
            "unit",
            "s",
            "mkpart",
            "primary",
            "fat32",
            &format!("{}s", start),
            &format!("{}s", end),
        ])
        .status()
        .map_err(|e| format!("Erreur mkpart: {}", e))?;

    if !status_mkpart.success() {
        return Err("parted a refusé de créer la partition.".to_string());
    }

    let _ = Command::new("/usr/sbin/partprobe")
        .arg(device_path)
        .output();
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();

    thread::sleep(Duration::from_millis(2000));

    let output_post = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/sbin/parted",
            "-s",
            "-m",
            device_path,
            "unit",
            "s",
            "print",
        ])
        .output()
        .map_err(|e| format!("Erreur de vérification: {}", e))?;

    let stdout_post = String::from_utf8_lossy(&output_post.stdout);
    let mut partition_id = String::new();

    for line in stdout_post.lines() {
        if line.contains(&format!(":{}s:", start)) {
            if let Some(id) = line.split(':').next() {
                partition_id = id.to_string();
                break;
            }
        }
    }

    if partition_id.is_empty() {
        return Err("Partition créée mais introuvable par le système.".to_string());
    }

    let part_suffix = if device_path.chars().last().unwrap_or('a').is_ascii_digit() {
        format!("p{}", partition_id)
    } else {
        partition_id.clone()
    };
    let partition_path = format!("{}{}", device_path, part_suffix);

    let _ = Command::new("/usr/bin/udisksctl")
        .args(["unmount", "-f", "-b", &partition_path])
        .output();

    let safe_volume_name: String = volume_name.to_uppercase().chars().take(11).collect();

    let script_formatage = format!(
        "/usr/sbin/wipefs -a {0} && /usr/sbin/mkfs.vfat -I -F 32 -n '{1}' {0} && sync",
        partition_path, safe_volume_name
    );

    let status_format = Command::new("/usr/bin/pkexec")
        .args(["bash", "-c", &script_formatage])
        .status()
        .map_err(|e| format!("Erreur lors de l'exécution du bloc de formatage: {}", e))?;

    if status_format.success() {
        Ok((start, end, partition_id))
    } else {
        Err(format!(
            "Le formatage de {} a échoué (IO Error ou annulation de l'utilisateur).",
            partition_path
        ))
    }
}

pub fn force_format(device_path: &str, partitions: &Vec<String>) -> Result<(), String> {
    // 1. Démonter toutes les partitions existantes (sda1, sda2...)
    for part in partitions {
        let _ = Command::new("/usr/bin/udisksctl")
            .args(["unmount", "-f", "-b", part]) // On ajoute -f (force)
            .output();
    }

    // Démonter le disque principal au cas où
    let _ = Command::new("/usr/bin/udisksctl")
        .args(["unmount", "-f", "-b", device_path])
        .output();

    // Pause pour laisser l'OS libérer les accès
    thread::sleep(Duration::from_millis(800));

    // 2. L'ARME SECRÈTE : wipefs
    // On efface toutes les signatures (FS, partitions)
    let _ = Command::new("/usr/bin/pkexec")
        .args(["/usr/sbin/wipefs", "-a", device_path])
        .status();

    // =========================================================
    // LA CORRECTION DE L'EMBOUTEILLAGE :
    // 1. On dit au programme d'attendre que Linux ait fini de réagir au wipefs
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();

    // 2. On ajoute une seconde de sécurité pour la mémoire de la puce BindKey
    thread::sleep(Duration::from_millis(1500));
    // =========================================================

    // 3. Créer une nouvelle table de partition vide (MBR/dos)
    let status = Command::new("/usr/bin/pkexec")
        .args(["/usr/sbin/parted", "-s", device_path, "mklabel", "msdos"])
        .status()
        .map_err(|e| format!("Impossible de lancer parted: {}", e))?;

    // 4. FORCER LA MISE À JOUR DU KERNEL
    // partprobe dit à l'OS "Oublie l'ancien cache, relis la clé physiquement !"
    let _ = Command::new("/usr/sbin/partprobe")
        .arg(device_path)
        .output();

    // udevadm settle dit à l'OS "Attends d'avoir fini de traiter les changements"
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();

    if status.success() {
        Ok(())
    } else {
        Err("Linux a refusé d'écrire la table de partition (Disque verrouillé).".to_string())
    }
}

pub fn rollback_physical_volume(
    device_path: &str,
    partition_number: &str,
    port_name: &str,
    volume_id: &str,
) {
    println!("DÉCLENCHEMENT DU ROLLBACK pour le volume {}", volume_id);

    // 1. Démonter la partition au cas où l'OS l'aurait auto-montée
    let part_suffix = if device_path.chars().last().unwrap_or('a').is_ascii_digit() {
        format!("p{}", partition_number)
    } else {
        partition_number.to_string()
    };
    let partition_path = format!("{}{}", device_path, part_suffix);

    let _ = Command::new("/usr/bin/udisksctl")
        .args(["unmount", "-f", "-b", &partition_path])
        .output();

    // 2. Dire à la BindKey d'oublier les secteurs alloués pour cet ID
    if !port_name.is_empty() {
        if let Ok(mut port) = serialport::new(port_name, 115200)
            .timeout(Duration::from_secs(2))
            .open()
        {
            // TODO: Ajuste cette commande selon la syntaxe de ton firmware pour supprimer un volume
            let cmd_delete = format!("delete_volume={}\n", volume_id);
            let _ = crate::usb_service::send_text_command(&mut *port, &cmd_delete);
        }
    }

    // 3. Demander à Linux de supprimer la partition de la table
    let _ = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/sbin/parted",
            "-s",
            device_path,
            "rm",
            partition_number,
        ])
        .output();

    // 4. Forcer la mise à jour de l'OS
    let _ = Command::new("/usr/sbin/partprobe")
        .arg(device_path)
        .output();
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();
}
