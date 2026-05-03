use std::future::Pending;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, current};
use std::time::Duration;

// Un compteur global, persistant et thread-safe, initialisé à 1
static SIMU_VOLUME_COUNTER: AtomicUsize = AtomicUsize::new(1);

use crate::BindKeyApp;
use crate::protocol::protocol::{
    ApiMessage, FetchedUserInfo, LsblkOutput, PendingShare, ShareAckPayload, ShareCompletePayload,
    ShareRequestPayload, ShareRequestResponse, UsbDevice, VolumeInfo, VolumeInitInfo,
    VolumeInitResponse, VolumeTab,
};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use eframe::egui;
use serialport::SerialPort;

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

                        // =========================================================
                        // VUE 1 : INTERFACE DE PARTAGE (Si un volume est sélectionné)
                        // =========================================================
                        if let Some(active_vol) = app.sharing_active_volume.clone() {
                            ui.horizontal(|ui| {
                                if ui.button("⬅ Retour aux volumes").clicked() {
                                    // On annule le partage et on nettoie l'interface
                                    app.sharing_active_volume = None;
                                    app.share_input_email.clear();
                                    app.share_search_feedback.clear();
                                    app.share_target_name = None;
                                    app.share_target_email = None;
                                    app.share_target_role = None;
                                }
                            });

                            ui.add_space(10.0);
                            ui.heading(format!("🔗 Partager le volume : {}", active_vol.name));
                            ui.label(format!("Espace: {} Go | Chemin: {}", active_vol.total_space_gb, active_vol.device_path));
                            ui.separator();
                            ui.add_space(10.0);

                            // --- RECHERCHE DE L'UTILISATEUR ---
                            ui.horizontal(|ui| {
                                ui.label("Email du destinataire :");
                                ui.add(egui::TextEdit::singleline(&mut app.share_input_email).hint_text("utilisateur@bindkey.com"));
                            });

                            ui.add_space(10.0);

                            if ui.add_enabled(!app.is_searching_user, egui::Button::new("🔍 Rechercher l'utilisateur")).clicked() {
                                if app.share_input_email.trim().is_empty() {
                                    app.share_search_feedback = "Veuillez entrer une adresse email.".to_string();
                                } else {
                                    app.is_searching_user = true;
                                    app.share_search_feedback = "Recherche en cours ⏳...".to_string();
                                    app.share_target_name = None;

                                    let clone_email = app.share_input_email.trim().to_string();
                                    let clone_sender = app.sender.clone();
                                    let clone_url = app.config.api_url.clone();
                                    let clone_token = app.server_token.clone();
                                    let clone_api_client = app.api_client.clone();

                                    tokio::spawn(async move {
                                        let url = format!("{}/users/search?email={}", clone_url, clone_email); 

                                        match clone_api_client.get(&url).bearer_auth(clone_token).send().await {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                                                        // ⚠️ À adapter selon ton JSON
                                                        let role_str = json["role"].as_str().unwrap_or("Utilisateur");

                                                        let parsed_role = match role_str {
                                                            "Admin" | "admin" => crate::protocol::protocol::Role::ADMIN,
                                                            "Utilisateur" | "utilisateur" | "User" | "user" => crate::protocol::protocol::Role::USER,
                                                            // Le cas par défaut si le serveur renvoie n'importe quoi
                                                            _ => crate::protocol::protocol::Role::USER,
                                                        };


                                                        let info = crate::protocol::protocol::FetchedUserInfo {
                                                            name: json["name"].as_str().unwrap_or("Inconnu").to_string(),
                                                            email: json["email"].as_str().unwrap_or("Inconnu").to_string(),
                                                            role: parsed_role,
                                                        };
                                                        let _ = clone_sender.send(ApiMessage::UserSearchResult(Ok(info)));
                                                    } else {
                                                        let _ = clone_sender.send(ApiMessage::UserSearchResult(Err("Erreur de format JSON".to_string())));
                                                    }
                                                } else if resp.status() == 404 {
                                                    let _ = clone_sender.send(ApiMessage::UserSearchResult(Err("Cet utilisateur n'existe pas.".to_string())));
                                                } else {
                                                    let _ = clone_sender.send(ApiMessage::UserSearchResult(Err(format!("Erreur serveur: {}", resp.status()))));
                                                }
                                            }
                                            Err(e) => {
                                                let _ = clone_sender.send(ApiMessage::UserSearchResult(Err(format!("Erreur réseau: {}", e))));
                                            }
                                        }
                                    });
                                }
                            }

                            if !app.share_search_feedback.is_empty() {
                                ui.add_space(5.0);
                                ui.label(egui::RichText::new(&app.share_search_feedback).color(egui::Color32::RED));
                            }

                            // --- AFFICHAGE DU RÉSULTAT ET BOUTON DE CONFIRMATION ---
                            // --- AFFICHAGE DU RÉSULTAT ET BOUTON DE CONFIRMATION ---
                            if let (Some(name), Some(email), Some(role)) = (&app.share_target_name, &app.share_target_email, &app.share_target_role) {
                                ui.add_space(20.0);
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.heading(egui::RichText::new("👤 Destinataire trouvé").strong());
                                    ui.add_space(5.0);

                                    ui.label(egui::RichText::new(format!("Nom : {}", name)).size(16.0));
                                    ui.label(egui::RichText::new(format!("Email : {}", email)).size(16.0));
                                    ui.label(egui::RichText::new(format!("Rôle : {:?}", role)).size(16.0));

                                    ui.add_space(15.0);

                                    // On bloque toute l'UI de ce bloc si le partage est en cours
                                    ui.add_enabled_ui(!app.is_sharing_in_progress, |ui| {
                                        if ui.button(egui::RichText::new(format!("🤝 Confirmer le partage à {}", name)).size(16.0)).clicked() {
                                            // 1. On verrouille l'interface
                                            app.is_sharing_in_progress = true;
                                            app.share_pipeline_status = "⏳ Étape 1/3 : Récupération du certificat sécurisé...".to_string();

                                            let clone_sender = app.sender.clone();
                                            let clone_api_client = app.api_client.clone();
                                            let clone_url = app.config.api_url.clone();
                                            let clone_token = app.server_token.clone();
                                            let port_name = app.current_port_name.clone();

                                            let local_volume_name = active_vol.name.clone();
                                            let target_email = email.clone();

                                            tokio::spawn(async move {
                                                let req_payload = ShareRequestPayload {
                                                    volume_name: local_volume_name,
                                                    target_user_email: target_email,
                                                };

                                                let res_phase1 = clone_api_client
                                                    .post(format!("{}/share_request", clone_url))
                                                    .bearer_auth(&clone_token)
                                                    .json(&req_payload)
                                                    .send()
                                                    .await;

                                                let target_info = match res_phase1 {
                                                    Ok(resp) if resp.status().is_success() => {
                                                        resp.json::<ShareRequestResponse>().await.unwrap()
                                                    },
                                                    Ok(resp) => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus(format!("Erreur serveur: {}", resp.status())));
                                                        return;
                                                    }
                                                    Err(e) => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus(format!("Erreur réseau: {}", e)));
                                                        return;
                                                    }
                                                };

                                                let _ = clone_sender.send(ApiMessage::SharePipelineStatus(" Étape 2/3 : Chiffrement matériel (NE débranchez pas la clé)...".to_string()));

                                                let target_sn = target_info.target_sn.clone();
                                                let volume_id = target_info.volume_id.clone();

                                                let hw_target_pubkey = target_info.target_pubkey_ecdh;
                                                let hw_target_slot = target_info.target_slot;

                                                let phase2_result = tokio::task::spawn_blocking(move || {
                                                    if port_name.is_empty() {return Err("Aucune clé connectée.".to_string());}
                                                    let mut port = serialport::new(&port_name, 115200).timeout(std::time::Duration::from_secs(5)).open().map_err(|e| e.to_string())?;

                                                    generate_hardware_share(&mut port, &volume_id, &target_sn, &hw_target_pubkey, hw_target_slot)
                                                }).await.unwrap_or(Err("Crash du thread matériel".to_string()));

                                                let (bk_sn,wrapped_key) = match phase2_result {
                                                    Ok(data) => data,
                                                    Err(e) => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus(format!("Refus matériel: {}", e)));
                                                        return;
                                                    }
                                                };

                                                let _ = clone_sender.send(ApiMessage::SharePipelineStatus("Étape 3/3 : Finalisation sur le serveur...".to_string()));

                                                let complete_payload = ShareCompletePayload {
                                                    source_sn: bk_sn,
                                                    target_sn: target_info.target_sn,
                                                    volume_id: target_info.volume_id,
                                                    wrapped: wrapped_key,
                                                };

                                                match clone_api_client
                                                    .post(format!("{}/share_complete", clone_url))
                                                    .bearer_auth(clone_token)
                                                    .json(&complete_payload)
                                                    .send()
                                                    .await
                                                {
                                                    Ok (resp) if resp.status().is_success() => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus("Partage Réussi ! Le destinataire peut accéder au volume.".to_string()));
                                                    }
                                                    Ok(resp) => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus(format!("Erreur finale serveur: {}", resp.status())));
                                                    }
                                                    Err(_) => {
                                                        let _ = clone_sender.send(ApiMessage::SharePipelineStatus("Échec de la confirmation réseau.".to_string()));
                                                    }
                                                }
                                            });
                                        }
                                    });

                                    // Affichage dynamique du statut de l'opération
                                    if !app.share_pipeline_status.is_empty() {
                                        ui.add_space(10.0);

                                        // Si c'est une erreur (contient '❌'), on affiche en rouge, sinon en bleu clair
                                        let status_lower = app.share_pipeline_status.to_lowercase();
                                        let color = if status_lower.contains("erreur") || status_lower.contains("refus") || status_lower.contains("échec") {
                                            egui::Color32::LIGHT_RED
                                        } else {
                                            egui::Color32::LIGHT_BLUE
                                        };

                                        ui.label(egui::RichText::new(&app.share_pipeline_status).color(color).italics());
                                    }
                                });
                            }
                        }
                        // =========================================================
                        // VUE 2 : LISTE DES VOLUMES (Ton code actuel)
                        // =========================================================
                        else {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Vos volumes sécurisés").size(24.0).strong());

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {

                                    // 1. On stocke l'état du clic dans une variable
                                    let refresh_clicked = ui.button(egui::RichText::new("🔄 Actualiser").size(16.0)).clicked();

                                    // 2. On déclenche si l'utilisateur clique OU si l'application le demande en arrière-plan
                                    if refresh_clicked || app.needs_volume_refresh {

                                        // 3. On réinitialise la demande pour éviter de rafraîchir en boucle
                                        app.needs_volume_refresh = false;

                                        if let Ok(output) = Command::new("/usr/bin/lsblk")
                                            .args(&["-J", "-b", "-o", "NAME,MODEL,SIZE,TRAN,FSTYPE,PTTYPE,MOUNTPOINT,LABEL,FSUSED"])
                                            .output()
                                        {
                                            let ouput_str = String::from_utf8_lossy(&output.stdout);
                                            if let Ok(parsed) = serde_json::from_str::<LsblkOutput>(&ouput_str) {
                                                let mut extracted_volumes = Vec::new();
                                                for disk in parsed.blockdevices {
                                                    if disk.tran.as_deref() == Some("usb") {
                                                        if let Some(children) = disk.children {
                                                            for part in children {
                                                                if part.size < 10_000_000 { continue; }

                                                                let total_gb = part.size as f64 / 1_073_741_824.0;

                                                                let used_gb = part.fsused.as_ref().and_then(|val| {
                                                                    if let Some(bytes) = val.as_u64() {
                                                                        Some(bytes as f64 / 1_073_741_824.0)
                                                                    } else if let Some(s) = val.as_str() {
                                                                        s.parse::<f64>().ok().map(|b| b / 1_073_741_824.0)
                                                                    } else {
                                                                        None
                                                                    }
                                                                });
                                                                extracted_volumes.push(VolumeInfo {
                                                                    name: part.label.unwrap_or_else(|| part.name.clone()),
                                                                    device_path: format!("/dev/{}", part.name),
                                                                    total_space_gb: (total_gb * 10.0).round() / 10.0,
                                                                    used_space_gb: used_gb.map(|v| (v * 10.0).round() / 10.0),
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

                            if app.dashboard_volumes.is_empty() {
                                ui.label(egui::RichText::new("Aucun volume BindKey détecté. Branchez votre clé et cliquez sur Actualiser.").italics());
                            } else {
                                for vol in &app.dashboard_volumes {
                                egui::Frame::group(ui.style())
                                    .inner_margin(15.0)
                                    .rounding(8.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());

                                        // ==========================================
                                        // LIGNE 1 : Titre et Statut
                                        // ==========================================
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&vol.name).size(25.0).strong());

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if vol.is_mounted {
                                                    ui.label(egui::RichText::new("🔓 Monté").color(egui::Color32::GREEN).size(22.0));
                                                } else {
                                                    ui.label(egui::RichText::new("🔒 Verrouillé / Non Monté").color(egui::Color32::RED).size(22.0));
                                                }
                                            });
                                        });

                                        ui.add_space(8.0);

                                        // ==========================================
                                        // LIGNE 2 : Informations (Taille & Chemin)
                                        // ==========================================
                                        ui.label(egui::RichText::new(format!("Chemin : {}", vol.device_path)).size(16.0).italics());
                                        ui.add_space(8.0);
                                        if vol.is_mounted {
                                            if let Some(used) = vol.used_space_gb {
                                                let fraction = (used / vol.total_space_gb) as f32;

                                                let color = if fraction > 0.9 {
                                                    egui::Color32::from_rgb(220, 50, 50)
                                                } else if fraction > 0.75 {
                                                    egui::Color32::from_rgb(220, 150, 50)
                                                } else {
                                                    egui::Color32::from_rgb(50, 150, 220)
                                                };

                                                ui.style_mut().visuals.selection.bg_fill = color;

                                                let progress = egui::ProgressBar::new(fraction)
                                                    .text(format!(" {:.1} Go / {:.1} Go utilisés", used, vol.total_space_gb))
                                                    .desired_height(18.0);

                                                ui.add(progress);
                                            } else {
                                                ui.add(egui::ProgressBar::new(0.0)
                                                    .text(format!(" Taille totale : {:.1} Go (Calcul en cours...)", vol.total_space_gb))
                                                    .desired_height(18.0)
                                                    .animate(true));

                                            }
                                        } else {
                                            ui.style_mut().visuals.selection.bg_fill = egui::Color32::from_rgb(80, 80, 80);
                                            let progress = egui::ProgressBar::new(1.0)
                                                .text(format!(" {:.1} Go - Espace indisponible (Volume verrouillé)", vol.total_space_gb))
                                                .desired_height(18.0);

                                            ui.add(progress);
                                        }
                                        ui.add_space(15.0);

                                        // ==========================================
                                        // LIGNE 3 : Boutons d'action (Alignés à droite)
                                        // ==========================================
                                        ui.horizontal(|ui| {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {

                                                // Attention: en right_to_left, le premier bouton ajouté est le plus à droite !

                                                if ui.button(egui::RichText::new("Supprimer le volume").color(egui::Color32::RED)).clicked() {
                                                    app.dashboard_status = "Suppression en cours...".to_string();
                                                    app.is_loading = true;
                                                    let _ = app.sender.send(ApiMessage::StartVolumeDeletion(vol.name.clone()));
                                                }


                                                // 1. Bouton Partager (Tout à droite)
                                                if ui.button(egui::RichText::new("🤝 Partager").size(20.0)).clicked() {
                                                    app.sharing_active_volume = Some(vol.clone());
                                                }

                                                ui.add_space(10.0); // Espace pour séparer le partage des actions de base

                                                // 2. Boutons d'état
                                                if vol.is_mounted {
                                                    if ui.button(egui::RichText::new("📂 Ouvrir").size(20.0)).clicked() {
                                                        if let Some(mount_path) = &vol.mount_point {
                                                            let _ = Command::new("xdg-open").arg(mount_path).spawn();
                                                        }
                                                    }

                                                    if ui.button(egui::RichText::new("🔌 Démonter").size(20.0)).clicked() {
                                                        let clone_path = vol.device_path.clone();
                                                        let clone_sender = app.sender.clone();
                                                        tokio::spawn(async move {
                                                            let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(format!("Démontage de {}...", clone_path)));
                                                            let umount_status = Command::new("/usr/bin/udisksctl").args(["unmount", "-b", &clone_path]).output();
                                                            if let Ok(output) = umount_status {
                                                                if output.status.success() {
                                                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Volume démonté avec succès.".to_string()));
                                                                    let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
                                                                } else {
                                                                    let err = String::from_utf8_lossy(&output.stderr);
                                                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(format!("Erreur démontage: {}", err)));
                                                                }
                                                            }
                                                        });
                                                    }
                                                } else {
                                                    if ui.button(egui::RichText::new("🔑 Monter le volume").size(20.0)).clicked() {
                                                        let clone_path = vol.device_path.clone();
                                                        let clone_sender = app.sender.clone();
                                                        tokio::spawn(async move {
                                                            let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Montage en cours... (Assurez-vous d'avoir validé votre empreinte)".to_string()));
                                                            let mount_status = Command::new("/usr/bin/udisksctl").args(["mount", "-b", &clone_path]).output();
                                                            if let Ok(output) = mount_status {
                                                                if output.status.success() {
                                                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Volume monté avec succès !".to_string()));
                                                                    let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
                                                                } else {
                                                                    let err = String::from_utf8_lossy(&output.stderr);
                                                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(format!("Erreur OS au montage: {}", err)));
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(15.0);
                            }
                            }
                        }
                        ui.add_space(20.0);

                        if ui.button("Vérifier les partages entrant").clicked() {
                            let clone_sender = app.sender.clone();
                            let clone_api_client = app.api_client.clone();
                            let clone_url = app.config.api_url.clone();
                            let clone_token = app.server_token.clone();
                            let port_name = app.current_port_name.clone();

                            let local_sn = app.local_bindkey_sn.clone().unwrap_or_default();

                            if local_sn.is_empty() {
                                let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Veuillez vous enrôler/connecter d'abord.".to_string()));
                            } else {
                                let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Recherche de partage en cours...".to_string()));

                                tokio::spawn(async move {

                                    let get_url = format!("{}/shares/pending?target_sn={}", clone_url, local_sn);

                                    let pending_res = clone_api_client.get(&get_url).bearer_auth(&clone_token).send().await;
                                    let pending_share: Vec<PendingShare> = match pending_res {
                                        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_else(|_| vec![]),
                                        _ => {
                                            let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Erreur de récupération du volume.".to_string()));
                                            return;
                                        }
                                    };
                                    if pending_share.is_empty() {
                                        let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Aucun nouveau partage en attente.".to_string()));
                                        return;
                                    }

                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(format!("Installation de {} partage", pending_share.len())));
                                    let mut success_count = 0;

                                    for share in pending_share{
                                        let hw_pubkey = share.source_pubkey_ecdh.clone();
                                        let hw_wrapped = share.wrapped.clone();
                                        let hw_slot = share.slot.clone();
                                        let port_name_clone = port_name.clone();

                                        let hw_result = tokio::task::spawn_blocking(move || {
                                            if port_name_clone.is_empty() {return Err("Clé Débranchée.".to_string()); }
                                            let mut port = serialport::new(&port_name_clone, 115200).timeout(std::time::Duration::from_secs(3)).open().map_err(|e| e.to_string())?;
                                            process_hardware_recv_share(&mut port, hw_slot, &hw_pubkey, &hw_wrapped)
                                        }).await.unwrap_or(Err("Crash thrad matériel".to_string()));

                                        match hw_result {
                                            Ok(_) => {
                                                let ack_payload = ShareAckPayload {share_id: share.share_id.clone() };
                                                let ack_res = clone_api_client
                                                    .post(format!("{}/share_acknowledged", clone_url))
                                                    .bearer_auth(&clone_token)
                                                    .json(&ack_payload)
                                                    .send()
                                                    .await;

                                                if let Ok(resp) = ack_res {
                                                    if resp.status().is_success() {
                                                        success_count += 1;
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                println!("Erreur matérielle sur le share {}: {}", share.share_id, e);
                                            }
                                        }
                                    }
                                    if success_count > 0 {
                                        let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(format!("{} partage(s) installé(s) ! REDÉMARREZ votre BindKey avec le disque pour les activer.", success_count)));
                                    } else {
                                        let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Échecde l'installation matérielle des partages.".to_string()));
                                    }
                                });
                            }
                        }

                        if !app.dashboard_status.is_empty() {
                            let color = if app.dashboard_status.contains("Erreur") || app.dashboard_status.contains("Refus") || app.dashboard_status.contains("❌") {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(100, 200, 255)
                            };
                            ui.colored_label(color, &app.dashboard_status);
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

                                let output = Command::new("/usr/bin/lsblk")
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

                                            if let Some(children) = disk.children {
                                                for child in children {
                                                    used_bytes += child.size;
                                                    partitions_paths.push(format!("/dev/{}", child.name));
                                                }
                                            }

                                            let free_bytes = if disk.fstype.is_some() && partitions_paths.is_empty() {
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
                                    ui.add(egui::Slider::new(&mut app.volume_created_size, 1..=max_size as i64).text("Go"));
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
                                        let clone_url = app.config.api_url.clone();
                                        let clone_auth_token = app.server_token.clone();
                                        let clone_port_name = app.current_port_name.clone();
                                        let clone_api_client = app.api_client.clone();
                                        let clone_device_path = app.available_devices[0].path.clone();

                                        tokio::spawn(async move {
                                           /*  let bypass_server = true;
                                            // =========================================================
                                            // 1. VÉRIFICATION SERVEUR (Inchangé)
                                            // =========================================================
                                            let serveur_volume_id = if bypass_server {
                                                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("[SIMU] Nom validé. Calcul des secteurs...".to_string()));

                                                // 🟢 On incrémente le compteur de 1 à chaque fois
                                                // (Ordering::SeqCst garantit que l'incrémentation est synchronisée entre tous les threads)
                                                let count = SIMU_VOLUME_COUNTER.fetch_add(1, Ordering::SeqCst);

                                                // On génère une chaîne unique : ID-SIMULATION-00001, ID-SIMULATION-00002...
                                                format!("ID-SIMU-{:05}", count)
                                            } else {
                                                
                                            };
                                            */

                                        let url = format!("{}/volumes/verify", clone_url);
                                        let payload = VolumeInitInfo {
                                            name: clone_volume_name.clone(),
                                        };

                                        let resultat = clone_api_client.post(url).json(&payload).bearer_auth(clone_auth_token).send().await;

                                        // 🌟 On assigne le résultat à une variable et on attend un Uuid
                                        let mon_id_serveur: String = match resultat {
                                        Ok(response) if response.status().is_success() => {
                                            let raw_text = response.text().await.unwrap_or_default();

                                            match serde_json::from_str::<VolumeInitResponse>(&raw_text) {
                                                Ok(data) => {
                                                    if data.exists {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur : le volume '{}' existe déjà", clone_volume_name)));
                                                        return;
                                                    }

                                                    // On récupère directement la String pure du serveur
                                                    if let Some(id) = data.volume_id {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Nom validé par le serveur. Calcul des secteurs...".to_string()));
                                                        id
                                                    } else {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Erreur : Le serveur n'a pas fourni d'ID.".to_string()));
                                                        return;
                                                    }
                                                }
                                                    Err(e) => {
                                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(format!("Erreur lecture JSON serveur : {}", e)));
                                                        return;
                                                    }
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
                                        };

                                            // À partir d'ici, tu peux utiliser `volume_id_str` (qui est une String) pour l'envoyer à la BindKey
                                            // =========================================================
                                            // 2. CRÉATION, COMMUNICATION USB & FORMATAGE (Tout-en-un)
                                            // =========================================================
                                            let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Synchronisation matérielle et découpage...".to_string()));

                                            // Clonage des variables pour le thread bloquant
                                            let device_path_for_thread = clone_device_path.clone();
                                            let volume_name_for_thread = clone_volume_name.clone();
                                            let volume_id_for_thread = mon_id_serveur.clone();
                                            let port_name_for_thread = clone_port_name.clone();

                                            // On utilise spawn_blocking car create_and_format_partition exécute des commandes OS synchrones
                                            let partition_result = tokio::task::spawn_blocking(move || {
                                                // ⚠️ Remplace "crate::ton_module" par le chemin réel vers ta fonction
                                                create_and_format_partition(
                                                    &device_path_for_thread,
                                                    clone_volume_size as f64,
                                                    &volume_name_for_thread,
                                                    &volume_id_for_thread,
                                                    &port_name_for_thread
                                                )
                                            }).await.unwrap_or_else(|e| Err(format!("Erreur critique du thread OS : {}", e)));

                                            // =========================================================
                                            // 3. RETOUR D'ÉTAT
                                            // =========================================================
                                             match partition_result {
                                                Ok((_start_sec, _end_sec, num_partition)) => {
                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("✅ Volume créé et synchronisé avec succès !".to_string()));

                                                    let _ = clone_sender.send(ApiMessage::VolumeCreationSuccess(UsbResponse::Success(SuccessData::VolumeCreated {
                                                        volume_id: mon_id_serveur,
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
                            app.is_loading = true;
                            let volume_names: Vec<String> = app.dashboard_volumes.iter().map(|v|v.name.clone()).collect();
                            let _ = app.sender.send(ApiMessage::StartFormatBindKey {
                                device_path: device.path.clone(),
                                partitions: device.partitions.clone(),
                                port_name: app.current_port_name.clone(),
                                volume_names,
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
                },
            }
        });
    });
}

// =================================================================
// FONCTIONS UTILITAIRES SYSTÈME
// =================================================================

// ⚠️ N'oublie pas d'ajouter port_name dans les paramètres lors de l'appel !
pub fn create_and_format_partition(
    device_path: &str,
    size_gb: f64,
    volume_name: &str,
    volume_id: &str,
    port_name: &str,
) -> Result<(u64, u64, String), String> {
    // =========================================================
    // FIX 1 : FORCER LA LECTURE DU CACHE AVANT LE CALCUL (LBA)
    // =========================================================
    // Indispensable pour éviter que Linux ne mente et redonne toujours le LBA 2048
    // =========================================================
    // 0. FORCER LE KERNEL À OUVRIR LES YEUX
    // =========================================================
    // Indispensable pour que Linux arrête de croire que la clé est vide
    let _ = Command::new("/usr/bin/pkexec")
        .args(["partprobe", device_path])
        .output();

    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();

    // =========================================================
    // 1. LECTURE DE L'ESPACE LIBRE
    // =========================================================
    let output = Command::new("/usr/bin/pkexec")
        .args(["parted", "-s", "-m", device_path, "unit", "s", "print"])
        .output()
        .map_err(|e| format!("Erreur parted: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 🟢=== LE DÉBOGAGE EST ICI ===🟢
    println!(
        "=== DEBUG SORTIE PARTED ===\n{}\n=========================",
        stdout
    );

    let mut disk_size_sectors: u64 = 0;
    let mut occupied: Vec<(u64, u64)> = Vec::new();

    // A. On scanne la clé pour lister toutes les partitions existantes
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();

        // La ligne du disque (ex: /dev/sdb:15633408s:scsi:512:512:msdos:...)
        if line.starts_with("/dev/") && parts.len() >= 2 {
            if let Ok(size) = parts[1].trim_end_matches('s').parse::<u64>() {
                disk_size_sectors = size;
            }
        }
        // Les lignes de partitions (ex: 1:2048s:2099199s:...)
        else if parts.len() >= 4 && parts[0].parse::<u32>().is_ok() {
            let s = parts[1].trim_end_matches('s').parse::<u64>().unwrap_or(0);
            let e = parts[2].trim_end_matches('s').parse::<u64>().unwrap_or(0);
            occupied.push((s, e));
        }
    }

    // B. On s'assure que les partitions sont triées par secteur de début
    occupied.sort_by_key(|&(s, _)| s);

    // C. Calcul de la taille cible en secteurs (alignée sur 1 Mo / 2048 secteurs)
    let target_sectors = (size_gb * 1024.0 * 1024.0 * 1024.0 / 512.0) as u64;
    let mut final_target = target_sectors;
    final_target -= final_target % 2048;

    // D. Recherche du premier trou (gap) disponible
    let mut start_sector: u64 = 0;
    let mut current_search_start: u64 = 2048; // On commence toujours à 2048 minimum

    for &(part_start, part_end) in &occupied {
        if part_start > current_search_start {
            let gap_size = part_start - current_search_start;
            if gap_size >= final_target {
                start_sector = current_search_start;
                break;
            }
        }
        // On saute après la partition actuelle et on s'aligne pour le prochain trou potentiel
        current_search_start = part_end + 1;
        let remainder = current_search_start % 2048;
        if remainder != 0 {
            current_search_start += 2048 - remainder;
        }
    }

    // E. Si on n'a pas trouvé de trou entre les partitions, on regarde après la dernière
    if start_sector == 0 {
        if disk_size_sectors > current_search_start {
            let gap_after = disk_size_sectors - current_search_start;
            if gap_after >= final_target {
                start_sector = current_search_start;
            }
        }
    }

    if start_sector == 0 {
        return Err("Plus d'espace libre suffisant sur la BindKey pour ce volume.".to_string());
    }

    let start = start_sector;
    let end = start_sector + final_target - 1;
    // =========================================================
    // 2. COMMUNICATION USB (LBA -> BindKey)
    // =========================================================
    {
        println!("Ouverture du port USB pour envoyer les LBA...");
        let mut port = serialport::new(port_name, 115200)
            .timeout(Duration::from_secs(5))
            .open()
            .map_err(|e| format!("Impossible d'ouvrir le port USB : {}", e))?;

        let _ = port.write_data_terminal_ready(true);
        let _ = port.write_request_to_send(true);
        thread::sleep(Duration::from_millis(500));

        let cmd_sectors = format!(
            "volume_name={}\nvolume_id={}\nlba_start={}\nlba_end={}\n",
            volume_name, volume_id, start, end
        );

        let mut is_ready = false;
        let mut tentatives = 0;

        while !is_ready && tentatives < 5 {
            match crate::usb_service::send_text_command(&mut *port, &cmd_sectors) {
                Ok(map) => {
                    if map
                        .get("STATUS")
                        .map(|val| val.contains("OK"))
                        .unwrap_or(false)
                    {
                        is_ready = true;
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

        if !is_ready {
            return Err(
                "La BindKey n'a pas confirmé l'enregistrement des secteurs LBA.".to_string(),
            );
        }
    } // Le port USB se ferme ici

    // =========================================================
    // 🛡️ LE BOUCLIER "MEDIUM NOT PRESENT"
    // =========================================================
    println!("Attente de 5 secondes que la BindKey reconnecte sa mémoire flash...");
    thread::sleep(Duration::from_secs(5));

    // On force Linux à reprendre conscience de la clé avant de faire quoi que ce soit
    let _ = Command::new("/usr/sbin/partprobe")
        .arg(device_path)
        .output();
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();
    // =========================================================

    // 3. CRÉATION PHYSIQUE EXACTE
    println!("BindKey prête. Lancement des commandes OS...");

    let safe_volume_name: String = volume_name.to_uppercase().chars().take(11).collect();

    // Nettoyage strict pour FAT32 (11 caractères, majuscules, lettres/chiffres/espaces)
    let safe_volume_name: String = volume_name
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .take(11)
        .collect();

    let script_creation = r#"#!/bin/bash
set -e
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        
DEVICE="$1"
START_SEC="$2"
END_SEC="$3"
VOL_NAME="$4"

if [[ ! "$DEVICE" =~ ^/dev/(sd[a-z]|nvme[0-9]n[0-9])$ ]]; then 
    echo "ERREUR_CHEMIN_INVALIDE: $DEVICE"
    exit 1
fi

/usr/sbin/parted -s -a optimal "$DEVICE" unit s mkpart primary fat32 "${START_SEC}s" "${END_SEC}s"
/usr/sbin/partprobe "$DEVICE" || true 
/usr/bin/udevadm settle
sleep 2

PART_ID=$(/usr/sbin/parted -s -ms "$DEVICE" unit s print | awk -F':' -v start="${START_SEC}s" '$2 == start {print $1}')

if [ -z "$PART_ID" ]; then 
    echo "ERREUR_ID_INTROUVABLE"
    exit 1
fi

if [[ "$DEVICE" =~ [0-9]$ ]]; then 
    PART_PATH="${DEVICE}p$PART_ID"
else 
    PART_PATH="${DEVICE}$PART_ID"
fi

/usr/bin/udisksctl unmount -f -b "$PART_PATH" 2>/dev/null || true
/usr/sbin/wipefs -a "$PART_PATH"
sleep 1

/usr/sbin/mkfs.vfat -I -F 32 -n "$VOL_NAME" "$PART_PATH"
sync

/usr/sbin/partprobe "$PART_PATH" || true
/usr/bin/udevadm settle
sleep 1

echo "SUCCES:$PART_ID"
"#;

    // Exécution du script avec les droits root (un seul pop-up !)
    let output_creation = Command::new("/usr/bin/pkexec")
        .arg("/bin/bash")
        .arg("-c")
        .arg(script_creation)
        .arg("_")
        .arg(device_path)
        .arg(start.to_string())
        .arg(end.to_string())
        .arg(&safe_volume_name)
        .output()
        .map_err(|e| format!("Échec de l'appel système global: {}", e))?;

    let stdout = String::from_utf8_lossy(&output_creation.stdout);

    // 🟢 ON IMPRIME TOUT LE LOG DU SCRIPT DANS LE TERMINAL RUST
    println!("\n========= DÉBUG DU SCRIPT BASH =========");
    println!("{}", stdout);
    println!("========================================\n");

    if !output_creation.status.success()
        || stdout.contains("ERREUR_ID")
        || stdout.contains("ERREUR_FATALE")
    {
        return Err(format!(
            "L'OS a refusé de créer ou formater la partition. Regardez les logs dans le terminal."
        ));
    }

    // Extraction de l'ID de la partition depuis le retour Bash
    let mut partition_id = String::new();
    for line in stdout.lines() {
        if line.starts_with("SUCCES:") {
            partition_id = line.replace("SUCCES:", "").trim().to_string();
        }
    }

    if partition_id.is_empty() {
        return Err("Création réussie, mais impossible de lire l'ID de retour.".to_string());
    }

    Ok((start, end, partition_id))
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

pub fn generate_hardware_share(
    port: &mut Box<dyn SerialPort>,
    volume_id: &str,
    target_sn: &str,
    target_pubkey: &str,
    target_slot: u16,
) -> Result<(String, String), String> {
    let share_commands = format!(
        "share_volume_id={}\nshare_target_sn={}\nshare_target_pubkey={}\nshare_targe_slot={}\n",
        volume_id, target_sn, target_pubkey, target_slot
    );

    match crate::usb_service::send_text_command(&mut **port, &share_commands) {
        Ok(map) => {
            if let (Some(sn), Some(wrapped)) = (map.get("SN"), map.get("WRAPPED")) {
                Ok((sn.clone(), wrapped.clone()))
            } else if let Some(err_reason) = map.get("ERR") {
                Err(format!("Refus matériel: {}", err_reason))
            } else {
                Err("Réponse incomplète (SN ou WRAPPED manquant).".to_string())
            }
        }
        Err(e) => Err(format!("Erreur de communication USB: {}", e)),
    }
}

pub fn process_hardware_recv_share(
    port: &mut Box<dyn SerialPort>,
    slot: u16,
    source_pubkey: &str,
    wrapped: &str,
) -> Result<(), String> {
    let commands = format!(
        "recv_share_slot={}\nrecv_share_source_pubkey={}\nrecv_share_wrapped={}\n",
        slot, source_pubkey, wrapped
    );

    match crate::usb_service::send_text_command(&mut **port, &commands) {
        Ok(map) => {
            if let Some(status) = map.get("STATUS") {
                if status == "OK" {
                    return Ok(());
                }
            } else if let Some(err) = map.get("ERR") {
                return Err(format!("Erreur puce ATTEC: {}", err));
            }
            if map.contains_key("OK") {
                return Ok(());
            }

            Err("Réponse inattendue de la BindKey".to_string())
        }
        Err(e) => Err(format!("Erreur de communication USB: {}", e)),
    }
}

/*
// =========================================================
    // 3. CRÉATION PHYSIQUE (Maintenant que la puce écoute)
    // =========================================================
    println!("BindKey prête. Création de la partition OS...");
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
        .map_err(|e| format!("Erreur lancement mkpart: {}", e))?;

    if !status_mkpart.success() {
        return Err("Échec de la commande parted mkpart.".to_string());
    }

    let _ = Command::new("/usr/sbin/partprobe")
        .arg(device_path)
        .output();
    let _ = Command::new("/usr/bin/udevadm").arg("settle").output();

    thread::sleep(Duration::from_millis(2000));

    // =========================================================
    // 4. RÉCUPÉRATION DE L'ID
    // =========================================================
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
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 && parts[1] == format!("{}s", start) {
            partition_id = parts[0].to_string();
            break;
        }
    }

    if partition_id.is_empty() {
        return Err("Partition créée, mais impossible de trouver son ID.".to_string());
    }

    let partition_path = if device_path.chars().last().unwrap_or('a').is_ascii_digit() {
        format!("{}p{}", device_path, partition_id)
    } else {
        format!("{}{}", device_path, partition_id)
    };

    let safe_volume_name: String = volume_name.to_uppercase().chars().take(11).collect();

    // =========================================================
    // 5. FORMATAGE (Là où la magie du chiffrement opère)
    // =========================================================
    let _ = Command::new("/usr/bin/udisksctl")
        .args(["unmount", "-f", "-b", &partition_path])
        .output();

    let _ = Command::new("/usr/bin/pkexec")
        .args(["/usr/sbin/wipefs", "-a", &partition_path])
        .status();

    thread::sleep(Duration::from_millis(1000));

    let output_format = Command::new("/usr/bin/pkexec")
        .args([
            // Sur Kali et Debian, mkfs.vfat est très souvent dans /usr/sbin/ ou /sbin/, pas dans /usr/bin/ !
            "/usr/sbin/mkfs.vfat",
            "-I",
            "-F",
            "32",
            "-n",
            &safe_volume_name,
            &partition_path,
        ])
        .output() // 🟢 FIX : On utilise output() au lieu de status()
        .map_err(|e| format!("Erreur fatale d'exécution de la commande: {}", e))?;

    if output_format.status.success() {
        let _ = Command::new("sync").status();
        Ok((start, end, partition_id)) // Tout est parfait !
    } else {
        // 🟢 FIX : On extrait la VRAIE erreur envoyée par l'OS
        let stderr = String::from_utf8_lossy(&output_format.stderr);
        Err(format!(
            "Le formatage de {} a échoué.\nRaison renvoyée par Linux : {}",
            partition_path, stderr
        ))
    }
}*/
