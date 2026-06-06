use crate::BindKeyApp;
use crate::protocol::protocol::{ApiMessage, Role, StatusBindkey};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use eframe::egui;
use sha2::{Digest, Sha256};
use validator::{self, ValidateEmail, ValidateLength};

pub fn show_enrollment_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("👤 Gestion des Utilisateurs").size(28.0).strong());
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Enrôlez de nouveaux utilisateurs ou modifiez les droits existants.")
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(30.0);
        });

        let card_frame = egui::Frame::none()
            .fill(ui.visuals().window_fill())
            .rounding(12.0)
            .inner_margin(20.0) // On garde l'espace à l'intérieur
            // On a supprimé le outer_margin pour éviter les collisions !
            .shadow(eframe::egui::epaint::Shadow {
                offset: egui::vec2(0.0, 4.0),
                blur: 10.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(40),
            });

        ui.columns(2, |cols| {
            // =========================================================
            // COLONNE GAUCHE : FORMULAIRE ET ACTIONS
            // =========================================================
            cols[0].vertical(|ui| {
                card_frame.show(ui, |ui| {
                    // (Suppression de ui.set_width pour réparer le texte étiré)
                    ui.heading("📝 Nouvel Utilisateur");
                    ui.separator();
                    ui.add_space(10.0);

                    egui::Grid::new("enroll_form_grid")
                        .num_columns(2)
                        .spacing([10.0, 15.0])
                        .show(ui, |ui| {
                            ui.label("Prénom :");
                            // f32::INFINITY = "Prends l'espace dispo sans déborder"
                            ui.add(egui::TextEdit::singleline(&mut app.enroll_firstname).desired_width(f32::INFINITY));
                            ui.end_row();

                            ui.label("Nom :");
                            ui.add(egui::TextEdit::singleline(&mut app.enroll_lastname).desired_width(f32::INFINITY));
                            ui.end_row();

                            ui.label("Email :");
                            ui.add(egui::TextEdit::singleline(&mut app.enroll_email)
                                .hint_text("ex: user@bindkey.com")
                                .desired_width(f32::INFINITY)
                            );
                            ui.end_row();

                            ui.label("Mot de passe :");
                            ui.vertical(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut app.enroll_password)
                                    .password(true)
                                    .desired_width(f32::INFINITY)
                                );
                                ui.label(egui::RichText::new("Min. 14 caractères").size(12.0).weak());
                            });
                            ui.end_row();

                            ui.label("Rôle :");
                            egui::ComboBox::from_id_salt("role_combo")
                                .selected_text(format!("{:?}", app.enroll_role))
                                .show_ui(ui, |ui| {
                                    if app.role_user == Role::ADMIN {
                                        ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                                        ui.selectable_value(&mut app.enroll_role, Role::ENROLLER, "ENROLLER");
                                    } else if app.role_user == Role::ENROLLER {
                                        ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                                    }
                                });
                            ui.end_row();
                        });
                });

                ui.add_space(20.0);

                // --- CARTE DES ACTIONS ---
                card_frame.show(ui, |ui| {
                    ui.heading("🚀 Actions");
                    ui.separator();
                    ui.add_space(10.0);

                    let formulaire_valide = !app.enroll_firstname.is_empty()
                        && !app.enroll_lastname.is_empty()
                        && !app.enroll_email.is_empty()
                        && app.enroll_email.validate_email()
                        && !app.enroll_password.is_empty()
                        && app.enroll_password.validate_length(Some(14), None, None)
                        && app.enroll_role != Role::NONE
                        && app.usb_connected;

                    let modif_valid = !app.enroll_email.is_empty()
                        && app.enroll_role != Role::NONE
                        && app.enroll_email.validate_email()
                        && app.enroll_firstname.is_empty()
                        && app.enroll_lastname.is_empty()
                        && app.enroll_password.is_empty()
                        && app.usb_connected;

                    ui.vertical_centered(|ui| {
                        if formulaire_valide {
                            if ui.add(egui::Button::new("➕ Enrôler le nouvel utilisateur").min_size(egui::vec2(ui.available_width(), 40.0))).clicked() {
                                let sender = app.sender.clone();
                                let port_name = app.current_port_name.clone();

                                tokio::spawn(async move {
                                    let _ = sender.send(ApiMessage::EnrollmentError("Attente de la clé USB...".to_string()));

                                    if port_name.is_empty() {
                                        let _ = sender.send(ApiMessage::EnrollmentError("Erreur : Aucune clé détectée".to_string()));
                                        return;
                                    }

                                    match serialport::new(&port_name, 115200).timeout(std::time::Duration::from_secs(45)).open() {
                                        Ok(mut port) => {
                                            let _ = port.write_data_terminal_ready(true);
                                            std::thread::sleep(std::time::Duration::from_millis(100));

                                            let _ = sender.send(ApiMessage::EnrollmentError("👆 Veuillez placer votre doigt 2 fois sur le capteur...".to_string()));
                                            let cmd = "enroll".to_string(); 

                                            match crate::usb_service::send_text_command(&mut *port, &cmd) {
                                                Ok(map) => {
                                                    if let (Some(sn), Some(pub_sign), Some(pub_ecdh)) = (map.get("SN"), map.get("PUB_SIGN"), map.get("PUB_ECDH")) {
                                                        let data = UsbResponse::Success(SuccessData::EnrollmentInfo {
                                                            sn: sn.clone(),
                                                            pub_sign: pub_sign.clone(),
                                                            pub_ecdh: pub_ecdh.clone(),
                                                        });
                                                        // On envoie le succès à l'Event Handler qui fera l'appel API !
                                                        let _ = sender.send(ApiMessage::EnrollmentUsbSuccess(data));
                                                    } else {
                                                        let _ = sender.send(ApiMessage::EnrollmentError("Erreur: SN, PUB_SIGN ou PUB_ECDH manquant dans la réponse".to_string()));
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = sender.send(ApiMessage::EnrollmentError(format!("Erreur communication USB : {}", e)));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = sender.send(ApiMessage::EnrollmentError(format!("Impossible d'ouvrir le port : {}", e)));
                                        }
                                    }
                                });
                            }

                        } else if modif_valid {
                            if ui.add(egui::Button::new("✏️ Modifier les droits (Email + Rôle)").min_size(egui::vec2(ui.available_width(), 40.0))).clicked() {
                                let sender = app.sender.clone();
                                let port_name = app.current_port_name.clone();

                                tokio::spawn(async move {
                                    let _ = sender.send(ApiMessage::EnrollmentError("Modification sur la clé USB en cours...".to_string()));

                                    if port_name.is_empty() {
                                        let _ = sender.send(ApiMessage::EnrollmentError("Erreur : Aucune clé détectée".to_string()));
                                        return;
                                    }

                                    match serialport::new(&port_name, 115200).timeout(std::time::Duration::from_secs(15)).open() {
                                        Ok(mut port) => {
                                            let _ = port.write_data_terminal_ready(true);
                                            std::thread::sleep(std::time::Duration::from_millis(100));

                                            // /!\ IMPORTANT : Mets ici la vraie commande de modif /!\
                                            let cmd = "cmd_modify".to_string(); 

                                            match crate::usb_service::send_text_command(&mut *port, &cmd) {
                                                Ok(_) => {
                                                    // La clé a dit OK, on lance la requête API
                                                    let data = UsbResponse::Success(SuccessData::Ack);
                                                    let _ = sender.send(ApiMessage::ModificationUsbSuccess(data));
                                                }
                                                Err(e) => {
                                                    let _ = sender.send(ApiMessage::EnrollmentError(format!("Erreur communication USB : {}", e)));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = sender.send(ApiMessage::EnrollmentError(format!("Impossible d'ouvrir le port : {}", e)));
                                        }
                                    }
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Remplissez tous les champs pour enrôler, ou seulement Email + Rôle pour modifier.")
                                .italics()
                                .weak()
                            );
                        }
                    });
                });
            });

            // =========================================================
            // COLONNE DROITE : LES LISTES (Utilisateurs et BindKeys)
            // =========================================================
            cols[1].vertical(|ui| {
            // 🛡️ VÉRIFICATION DU RÔLE ICI
            if app.role_user == Role::ADMIN {
                // =========================================================
                // VUE ADMIN : Moteur de recherche amélioré
                // =========================================================
                card_frame.show(ui, |ui| {
                    ui.label(egui::RichText::new("🔍 Recherche Utilisateur").strong().size(24.0));
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(15.0);

                    // --- 1. BARRE DE RECHERCHE MODERNE ---
                    ui.horizontal(|ui| {
                        let search_box = ui.add_sized(
                            [ui.available_width() - 120.0, 35.0],
                            egui::TextEdit::singleline(&mut app.search_email_input)
                                .hint_text("✉️ Entrez l'adresse email de l'utilisateur...")
                                .margin(egui::vec2(10.0, 10.0)),
                        );

                        // Bouton plus lisible
                        let btn_clicked = ui.add_sized(
                            [110.0, 35.0],
                            egui::Button::new(egui::RichText::new("Rechercher").size(16.0))
                        ).clicked();

                        if btn_clicked || (search_box.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                            if !app.search_email_input.is_empty() {
                                let _ = app.sender.send(ApiMessage::SearchUserByEmail(app.search_email_input.clone()));
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // --- BOUTON VOIR TOUS LES UTILISATEURS ---
                    if ui.add_sized(
                        [ui.available_width(), 35.0],
                        egui::Button::new(egui::RichText::new("📋 Afficher tous les utilisateurs").size(16.0))
                    ).clicked() {
                        let _ = app.sender.send(ApiMessage::FetchUsers);
                    }

                    ui.add_space(20.0);

                    // --- 2. RÉSULTAT : CARTE UTILISATEUR ---
                    if let Some(user_data) = &mut app.search_result {

                        egui::Frame::group(ui.style())
                            .inner_margin(15.0)
                            .rounding(8.0)
                            .show(ui, |ui| {

                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(format!("👤 {} {}", user_data.first_name, user_data.last_name)).size(24.0).strong());
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(format!("📧 {}", user_data.email)).size(20.0).italics());
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(format!("Rôle : {:?}", user_data.role)).size(20.0));
                                    });

                                    // Bouton Supprimer aligné à droite avec icône et texte ajustés
                                    if user_data.email != app.login_email {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                            if ui.button(egui::RichText::new("Supprimer").size(20.0).color(egui::Color32::LIGHT_RED)).clicked() {
                                                let _ = app.sender.send(ApiMessage::DeleteUser(user_data.id.clone()));
                                            }
                                        });
                                    }
                                });

                                ui.add_space(20.0);
                                ui.separator();
                                ui.add_space(15.0);

                                // --- 3. CARTE BINDKEY ASSOCIÉE ---
                                ui.label(egui::RichText::new("🔑 BindKey Associée").size(24.0).strong());
                                ui.add_space(10.0);

                                if let Some(bk) = &mut user_data.bindkey {
                                    egui::Frame::none()
                                        .fill(ui.visuals().faint_bg_color)
                                        .rounding(6.0)
                                        .inner_margin(16.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {

                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("📌 Numéro de Série").size(20.0));
                                                    ui.add_space(2.0);
                                                    ui.label(egui::RichText::new(&bk.serial_number).size(20.0).monospace().strong());
                                                });

                                                // Actions BindKey à droite
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button(egui::RichText::new("💾 Appliquer").size(20.0)).clicked() {
                                                        let _ = app.sender.send(ApiMessage::UpdateBindKeyStatus(
                                                            bk.serial_number.clone(),
                                                            bk.status.clone(),
                                                        ));
                                                    }

                                                    let current_status_text = match bk.status {
                                                        StatusBindkey::ACTIVE => "Actif",
                                                        StatusBindkey::RESET => "Révoquée",
                                                        StatusBindkey::LOST => "Perdue",
                                                        StatusBindkey::BROKEN => "Cassée",
                                                    };

                                                    egui::ComboBox::from_id_salt("status_combo")
                                                        .selected_text(egui::RichText::new(current_status_text).size(20.0))
                                                        .show_ui(ui, |ui| {
                                                            ui.selectable_value(&mut bk.status, StatusBindkey::ACTIVE, "Actif");
                                                            ui.selectable_value(&mut bk.status, StatusBindkey::RESET, "Révoquée");
                                                            ui.selectable_value(&mut bk.status, StatusBindkey::LOST, "Perdue");
                                                            ui.selectable_value(&mut bk.status, StatusBindkey::BROKEN, "Cassée");
                                                        });

                                                    ui.label(egui::RichText::new("Statut :").size(20.0));
                                                });
                                            });
                                        });
                                } else {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgba_unmultiplied(255, 165, 0, 20))
                                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 165, 0)))
                                        .rounding(6.0)
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("⚠️").size(20.0));
                                                ui.label(egui::RichText::new("Aucune BindKey n'est assignée à cet utilisateur pour le moment.")
                                                    .size(15.0)
                                                    .color(egui::Color32::from_rgb(255, 200, 100)));
                                            });
                                        });
                                }
                            });
                    }

                    // --- 4. RÉSULTAT : LISTE DE TOUS LES UTILISATEURS ---
                    if !app.users_list.is_empty() {
                        ui.label(egui::RichText::new(format!("👥 Utilisateurs enregistrés ({})", app.users_list.len())).size(20.0).strong());
                        ui.add_space(10.0);

                        egui::ScrollArea::vertical().id_salt("users_list_scroll").max_height(400.0).show(ui, |ui| {
                            for user in &app.users_list {
                                egui::Frame::group(ui.style())
                                    .inner_margin(10.0)
                                    .rounding(6.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(format!("{} {}", user.first_name, user.last_name)).size(18.0).strong());
                                                ui.label(egui::RichText::new(&user.email).italics());
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{:?}", user.role)).strong());
                                                if user.email != app.login_email {
                                                    if ui.button("Supprimer").clicked() {
                                                        let _ = app.sender.send(ApiMessage::DeleteUser(user.id.clone()));
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(5.0);
                            }
                        });
                    }

                });

                } else {
                    // =========================================================
                    // VUE STANDARD : On bloque l'interface
                    // =========================================================
                    card_frame.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            ui.heading("🔒 Accès Restreint");
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("La gestion des utilisateurs et des BindKeys\nest réservée aux administrateurs.")
                                    .color(egui::Color32::GRAY)
                                    .italics(),
                            );
                            ui.add_space(50.0);
                        });
                    }); // Fin du card_frame.show (NON-ADMIN)
                }

                // =========================================================
                // AFFICHAGE DU STATUT DES ACTIONS (Pour tout le monde)
                // =========================================================
                ui.centered_and_justified(|ui| {
                    if !app.enroll_status.is_empty() {
                        ui.add_space(10.0);
                        let color = if app.enroll_status.contains("Erreur") || app.enroll_status.contains("Refus") {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 200, 255)
                        };
                        ui.colored_label(color, &app.enroll_status);
                    }
                });
            });
        });
    });
}

pub fn hash_password_with_salt(password: &str) -> String {
    let salt = "bindkey.com";
    let combined = format!("{}{}", password, salt);
    let mut hasher = Sha256::new();
    hasher.update(combined);
    format!("{:x}", hasher.finalize())
}
