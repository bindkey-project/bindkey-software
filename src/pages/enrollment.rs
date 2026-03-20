use crate::BindKeyApp;
use crate::protocol::protocol::{ApiMessage, Role};
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
                        .spacing([10.0, 15.0]) // Espacement un peu réduit
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

                ui.add_space(20.0); // Espace entre les deux cartes de gauche

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
                                // Ton code Tokio ici...
                            }
                        } else if modif_valid {
                            if ui.add(egui::Button::new("✏️ Modifier les droits (Email + Rôle)").min_size(egui::vec2(ui.available_width(), 40.0))).clicked() {
                                // Ton code Tokio ici...
                            }
                        } else {
                            ui.label(egui::RichText::new("Remplissez tous les champs pour enrôler, ou seulement Email + Rôle pour modifier.")
                                .italics()
                                .weak()
                            );
                        }

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

            // =========================================================
            // COLONNE DROITE : LES LISTES (Utilisateurs et BindKeys)
            // =========================================================
            cols[1].vertical(|ui| {
                card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("📋 Utilisateurs");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 Actualiser").clicked() {
                                let _ = app.sender.send(ApiMessage::FetchUsers);
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(10.0);

                    // L'astuce ici : On autorise le défilement horizontal !
                    // Si l'email d'un utilisateur est trop long, ça ne cassera pas la colonne.
                    egui::ScrollArea::both()
                        .id_salt("user_scroll")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            egui::Grid::new("user_list_grid")
                                .striped(true)
                                .spacing([20.0, 10.0])
                                .show(ui, |ui| {
                                    ui.strong("Nom");
                                    ui.strong("Email");
                                    ui.strong("Rôle");
                                    ui.strong("Actions");
                                    ui.end_row();

                                    if app.users_list.is_empty() {
                                        ui.label("Aucun utilisateur chargé...");
                                        ui.label("-"); ui.label("-"); ui.label("-");
                                        ui.end_row();
                                    } else {
                                        for user in app.users_list.iter() {
                                            ui.label(format!("{} {}", user.first_name, user.last_name));
                                            ui.label(&user.email);

                                            let color = match user.role {
                                                Role::ENROLLER => egui::Color32::LIGHT_BLUE,
                                                Role::ADMIN => egui::Color32::LIGHT_RED,
                                                _ => egui::Color32::GRAY,
                                            };
                                            let role_text = match user.role {
                                                Role::ADMIN => "Admin",
                                                Role::ENROLLER => "Enrôleur",
                                                Role::USER => "Utilisateur",
                                                Role::NONE => "Aucun"
                                            };
                                            ui.colored_label(color, role_text);

                                            if user.email != app.login_email && app.role_user == Role::ADMIN {
                                                if ui.button("🗑️").on_hover_text("Supprimer").clicked() {
                                                    let _ = app.sender.send(ApiMessage::DeleteUser(user.id));
                                                }
                                            } else {
                                                ui.label(""); 
                                            }
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                });

                ui.add_space(20.0); // Espace entre les deux cartes de droite

                // --- CARTE DES BINDKEYS ---
                card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🔑 BindKeys");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 Actualiser").clicked() {
                                // Action d'actualisation des clés
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::both()
                        .id_salt("bk_scroll")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            egui::Grid::new("bk_list_grid")
                                .striped(true)
                                .spacing([20.0, 10.0])
                                .show(ui, |ui| {
                                    ui.strong("Serial Number");
                                    ui.strong("Public Key");
                                    ui.strong("Status");
                                    ui.strong("Actions");
                                    ui.end_row();

                                    if app.users_list.is_empty() {
                                        ui.label("Aucune BindKey chargée...");
                                        ui.label("-"); ui.label("-"); ui.label("-");
                                        ui.end_row();
                                    } else {
                                        for user in app.users_list.iter() {
                                            ui.label(format!("{} {}", user.first_name, user.last_name));
                                            ui.label(&user.email);
                                            ui.colored_label(egui::Color32::LIGHT_GREEN, "Actif");
                                            if ui.button("🗑️").clicked() { /* ... */ }
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
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
