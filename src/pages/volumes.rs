use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::BindKeyApp;
use crate::protocol::protocol::{
    ApiMessage, LsblkOutput, UsbDevice, VolumeInitInfo, VolumeInitResponse, VolumeTab,
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
                ui.selectable_value(&mut app.active_tab, VolumeTab::Gestion, "Gestion des Volumes");
                ui.selectable_value(&mut app.active_tab, VolumeTab::Formatage, "Formatage clé USB");
            });
            ui.separator();
            ui.add_space(10.0);

            ui.label("Gérez vos espaces sécurisés directement depuis votre BindKey.");
            ui.add_space(30.0);

            match app.active_tab {
                // =================================================================
                // ONGLET 1 : GESTION DES VOLUMES
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

    // ==========================================
    // 1. PARTIE MATÉRIELLE : Scan brut (lsblk)
    // ==========================================
                                let output = Command::new("lsblk")
                                    .args(&["-J", "-b", "-o", "NAME,MODEL,SIZE,TRAN,FSTYPE,PTTYPE"])
                                    .output()
                                    .expect("Erreur lsblk");

                                let ouput_str = String::from_utf8_lossy(&output.stdout);

                                println!("DEBUG LSBLK : {}", ouput_str);
                                let parsed: LsblkOutput = serde_json::from_str(&ouput_str).unwrap_or(LsblkOutput { blockdevices: vec![] });

                                let mut devices = Vec::new();

    // On prépare des variables pour stocker les capacités de la clé qu'on va trouver
                                let mut target_total_gb = 0.0;
                                let mut target_free_gb = 0.0;
                                let mut target_name = String::new();

                                for disk in parsed.blockdevices {
                                    if let Some(tran) = disk.tran {
                                        if tran.trim() == "usb" {

                                            let model = disk.model.unwrap_or("Inconnu".to_string());
                                            println!("Model : {}", model);

                                            if !model.to_uppercase().contains("BINDKEY") {
                                                continue;
                                            }

                                            let size_gb = (disk.size as f64) / (1024.0 * 1024.0 * 1024.0);
                                            let label = format!("{} ({:.1} GB) - /dev/{}", model.trim(), size_gb, disk.name);

                // --- LE CALCUL MAGIQUE DE L'ESPACE LIBRE ---
                                            let mut used_bytes = 0;
                                            let mut partitions_paths = Vec::new();

                // Si le disque a des partitions (children)
                                            if let Some(children) = &disk.children {
                                                for child in children {
                                                    used_bytes += child.size; // On additionne la taille des partitions
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

                // On met à jour nos variables globales pour cette clé
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

    // ==========================================
    // 2. PARTIE LOGICIELLE : Port Série (BindKey)
    // ==========================================
                                let clone_sender = app.sender.clone();
                                let clone_port_name = app.current_port_name.clone();
                                let bypass_usb = true;

                                let clone_total_gb = (target_total_gb * 100.0).round() / 100.0;
                                let clone_free_gb = (target_free_gb * 100.0).round() / 100.0;
                                let clone_device_name = if target_name.is_empty() { "BINDKEY".to_string() } else { target_name };

                                tokio::spawn(async move {
                                    let resultat_usb: Result<UsbResponse, String>;

                                    if bypass_usb {
                                        println!(">> SIMULATION SCAN DISQUE");
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
                                        ui.colored_label(
                                        egui::Color32::BLUE,
                                        format!("{} Go", app.device_size),
                                        );
                                        ui.end_row();

                                        ui.label("Espace Disponible :");
                                        let color = if app.device_available_space < 5.0 {
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
                                    ui.add(
                                    egui::TextEdit::singleline(&mut app.volume_created_name)
                                        .min_size(egui::vec2(200.0, 20.0)),
                                    );
                                });

                                ui.add_space(10.0);

                                let max_size = if app.device_available_space > 0.0 {
                                    app.device_available_space
                                } else {
                                    1.0
                                };
                                ui.horizontal(|ui| {
                                    ui.label("Taille allouée :");
                                    ui.add(
                                    egui::Slider::new(&mut app.volume_created_size, 1..=max_size as u32)
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
    // =================================================================
    // 0. LES VARIABLES DE TEST (SIMULATION)
    // =================================================================
                                            let bypass_server = true;
                                            let bypass_usb = true;

    // =================================================================
    // 1. VALIDATION SERVEUR (API)
    // =================================================================
                                            let serveur_volume_id = if bypass_server {
        // --- MODE SIMULATION SERVEUR ---
                                                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("[SIMU] Nom validé. Création de la partition...".to_string()));

                                                "ID-SIMULATION-12345".to_string()

                                            } else {
        // --- MODE RÉEL ---
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
                                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Nom validé par le serveur. Création de la partition...".to_string()));

                                                            data.volume_id // C'est ici qu'on récupère le vrai ID
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

// =================================================================
    // ÉTAPE 2 : INITIALISATION DE LA CLÉ DE CHIFFREMENT (BINDKEY)
    // =================================================================
                                            let mut encrypted_key_storage = String::new();

                                                if bypass_usb {
        // --- MODE SIMULATION USB (INIT) ---
                                                    println!(">> SIMULATION INIT CRYPTO avec ID: {}", serveur_volume_id);
                                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                                    encrypted_key_storage = "SIMULATED-KEY-XYZ-999".to_string();
                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("[SIMU] Clé générée. Découpage du disque...".to_string()));
                                                } else {
        // --- MODE RÉEL USB (INIT) ---
                                                    if !clone_port_name.is_empty() {
                                                        match serialport::new(&clone_port_name, 115200).timeout(std::time::Duration::from_secs(2)).open() {
                                                            Ok(mut port) => {
                                                                let _ = port.write_data_terminal_ready(true);
                                                                let _ = port.write_request_to_send(true);
                                                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    // On envoie la demande de génération de clé
                                                                let cmd_init = format!(
                                                                    "volume_name={}\n size_gb={}\n volume_id={}\n", 
                                                                    clone_volume_name, clone_volume_size, serveur_volume_id
                                                                );

                                                                match crate::usb_service::send_text_command(&mut *port, &cmd_init) {
                                                                    Ok(map) => {
                                                                        if let Some(key) = map.get("KEY").cloned() {
                                                                            encrypted_key_storage = key;
                                                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Clé générée. Découpage du disque...".to_string()));
                                                                        } else {
                                                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Erreur: La puce n'a pas renvoyé la clé".to_string()));
                                                                            return;
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Échec USB (Init): {}", e)));
                                                                        return;
                                                                    }
                                                                }
                                                            },
                                                            Err(e) => {
                                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Impossible d'ouvrir le port: {}", e)));
                                                                return;
                                                            }
                                                        }
                                                    } else {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Aucune Bindkey détectée".to_string()));
                                                        return;
                                                    }
                                                }

    // =================================================================
    // ÉTAPE 3 : DÉCOUPAGE ET FORMATAGE (OS)
    // =================================================================
    let partition_result = create_and_format_partition(
        &clone_device_path,
        clone_volume_size as f64,
        &clone_volume_name
    );

    let (start_sec, end_sec) = match partition_result {
        Ok(secteurs) => secteurs,
        Err(e) => {
            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur disque: {}", e)));
            return;
        }
    };

    let _ = clone_sender.send(
        ApiMessage::VolumeCreationStatus("✅ Partition prête. Verrouillage final...".to_string())
    );

    // =================================================================
    // ÉTAPE 4 : VERROUILLAGE PHYSIQUE DES SECTEURS (BINDKEY)
    // =================================================================
    if bypass_usb {
        // --- MODE SIMULATION USB (LOCK) ---
        println!(">> SIMULATION LOCK avec secteurs {} à {}", start_sec, end_sec);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // C'est terminé, on envoie le succès à l'UI !
        let _ = clone_sender.send(ApiMessage::VolumeCreationSuccess(UsbResponse::Success(SuccessData::VolumeCreated {
            encrypted_key: encrypted_key_storage,
            volume_id: serveur_volume_id,
        })));
    } else {
        // --- MODE RÉEL USB (LOCK) ---
        if let Ok(mut port) = serialport::new(&clone_port_name, 115200).timeout(std::time::Duration::from_secs(2)).open() {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // On envoie l'ID et les secteurs pour que la puce verrouille la zone physique
            let cmd_lock = format!(
                "assign_sectors={} start_sec={} end_sec={}\n", 
                serveur_volume_id, start_sec, end_sec
            );

            let _ = crate::usb_service::send_text_command(&mut *port, &cmd_lock);

            // C'est terminé, on envoie le succès à l'UI !
            let _ = clone_sender.send(ApiMessage::VolumeCreationSuccess(UsbResponse::Success(SuccessData::VolumeCreated {
                encrypted_key: encrypted_key_storage,
                volume_id: serveur_volume_id
            })));
        } else {
            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Erreur USB (Lock): Port inaccessible".to_string()));
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
                // ONGLET 2 : FORMATAGE BRUT
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
                            match force_format(&device.path, &device.partitions) {
                                Ok(_) => app.formatage_status = format!("Succès : {} est vide (Espace non alloué)", device.path),
                                Err(e) => app.formatage_status = format!("Erreur : {}", e)
                            }
                        }
                    }
                    else if app.available_devices.len() > 1 {
                        ui.colored_label(egui::Color32::RED, "Sécurité : Plusieurs clés détectées. Veuillez n'en brancher qu'une seule pour le formatage.");
                    }
                    else {
                        ui.label("Branchez une Binkey et lancez l'analyse dans l'onglet 'Gestion des volumes' pour la formater");
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

fn create_and_format_partition(
    device_path: &str,
    size_gb: f64,
    volume_name: &str,
) -> Result<(u64, u64), String> {
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
                    if size >= target_sectors {
                        start_sector = Some(start);
                        end_sector = Some(start + target_sectors - 1);
                        break;
                    }
                }
            }
        }
    }

    let start = start_sector.ok_or("Espace libre insuffisant sur la clé.")?;
    let end = end_sector.unwrap();

    let status_mkpart = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/sbin/parted",
            "-s",
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

    thread::sleep(Duration::from_millis(1500));

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
        partition_id
    };
    let partition_path = format!("{}{}", device_path, part_suffix);

    let status_format = Command::new("/usr/bin/pkexec")
        .args([
            "/usr/bin/mkfs.vfat",
            "-I",
            "-F",
            "32",
            "-n",
            volume_name,
            &partition_path,
        ])
        .status()
        .map_err(|e| format!("Erreur mkfs: {}", e))?;

    if status_format.success() {
        Ok((start, end))
    } else {
        Err(format!("Le formatage de {} a échoué.", partition_path))
    }
}

fn force_format(device_path: &str, partitions: &Vec<String>) -> Result<(), String> {
    // 1. Démonter le disque principal
    let _ = Command::new("/usr/bin/udisksctl")
        .arg("unmount")
        .arg("-b")
        .arg(device_path)
        .output();

    // 2. Démonter toutes les partitions existantes (sda1, sda2...)
    for part in partitions {
        let _ = Command::new("/usr/bin/udisksctl")
            .arg("unmount")
            .arg("-b")
            .arg(part)
            .output();
    }

    // Petite pause pour laisser le temps à l'OS de libérer les fichiers
    thread::sleep(Duration::from_secs(1));

    // 3. Créer une nouvelle table de partition vide (Wipe total)
    let status = Command::new("/usr/bin/pkexec")
        .args(["/usr/sbin/parted", "-s", device_path, "mklabel", "msdos"])
        .status()
        .map_err(|e| format!("Impossible de lancer pkexec: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("La réinitialisation a échoué ou a été annulée par l'utilisateur".to_string())
    }
}
