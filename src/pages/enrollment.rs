use std::time::Duration;

use crate::BindKeyApp;
use crate::protocol::protocol::{ApiMessage, Role};
use crate::protocol::share_protocol::{self, SuccessData, UsbResponse};
use crate::usb_service::send_command;
use eframe::egui;
use sha2::{Digest, Sha256};
use validator::{self, ValidateEmail, ValidateLength};

pub fn show_enrollment_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {

        ui.vertical_centered(|ui| {
            ui.set_max_width(600.0);

            ui.add_space(20.0);
            ui.heading("👤 Gestion des Utilisateurs");
            ui.add_space(10.0);
            ui.label("Enrôlez de nouveaux utilisateurs ou modifiez les droits existants.");
            ui.add_space(30.0);

            let frame_style = egui::Frame::none()
                .fill(ui.visuals().window_fill())
                .rounding(10.0)
                .stroke(ui.visuals().window_stroke())
                .inner_margin(20.0);

            frame_style.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("📝 Informations");
                ui.add_space(15.0);

                egui::Grid::new("enroll_form_grid")
                    .num_columns(2)
                    .spacing([20.0, 15.0])
                    .show(ui, |ui| {

                        ui.label("Prénom :");
                        ui.add(egui::TextEdit::singleline(&mut app.enroll_firstname).min_size(egui::vec2(300.0, 25.0)));
                        ui.end_row();

                        ui.label("Nom :");
                        ui.add(egui::TextEdit::singleline(&mut app.enroll_lastname).min_size(egui::vec2(300.0, 25.0)));
                        ui.end_row();

                        ui.label("Email :");
                        ui.add(egui::TextEdit::singleline(&mut app.enroll_email)
                            .hint_text("ex: user@bindkey.com")
                            .min_size(egui::vec2(300.0, 25.0))
                        );
                        ui.end_row();

                        ui.label("Mot de passe :");
                        ui.vertical(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut app.enroll_password)
                                .password(true)
                                .min_size(egui::vec2(300.0, 25.0))
                            );
                            ui.label(egui::RichText::new("Min. 14 caractères").size(20.0).weak());
                        });
                        ui.end_row();

                        ui.label("Rôle :");
                        egui::ComboBox::from_id_salt("role_combo")
                            .selected_text(format!("{:?}", app.enroll_role))
                            .width(300.0)
                            .show_ui(ui, |ui| {
                                if app.role_user == Role::ADMIN {
                                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                                    ui.selectable_value(&mut app.enroll_role, Role::ENROLLEUR, "ENROLLEUR");
                                } else if app.role_user == Role::ENROLLEUR {
                                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                                }
                            });
                        ui.end_row();
                    });
            });

            ui.add_space(20.0);

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

            frame_style.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("🚀 Actions");
                ui.add_space(15.0);

                ui.vertical_centered(|ui| {

                    if formulaire_valide {
                        let btn = egui::Button::new(" Enrôler le nouvel utilisateur")
                            .min_size(egui::vec2(250.0, 45.0));

                        if ui.add(btn).clicked() {
                            let bypass_usb = false;
                            app.enroll_status = if bypass_usb {
                                "🛠️ SIMULATION : Bypass USB activé...".to_string()
                            } else {
                                "🔌 Recherche de la clé USB...".to_string()
                            };

                            let clone_sender = app.sender.clone();
                            let clone_port_name = app.current_port_name.clone();

                            tokio::spawn(async move {
                                let resultat_usb: Result<UsbResponse, String>;

                                if bypass_usb {
                                    println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    resultat_usb = Ok(UsbResponse::Success(SuccessData::EnrollmentInfo {
                                        uid: "SIMULATED-UID".to_string(),
                                        public_key: "SIM-KEY-123".to_string(),
                                    }));
                                } else {

                                    if !clone_port_name.is_empty() {
                                        match serialport::new(&clone_port_name, 115200)
                                        .timeout(std::time::Duration::from_secs(15))
                                        .open() {
                                            Ok(mut port) => {
                                                let _ = port.write_data_terminal_ready(true);
                                                let _ = port.write_request_to_send(true);
                                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                                match crate::usb_service::send_text_command(&mut *port, "enroll") {
                                                   Ok(map) => {
                                                    let uid_opt = map.get("SN").cloned();
                                                    let pub_opt = map.get("PUB").cloned();

                                                    if let (Some(uid), Some(public_key)) = (uid_opt, pub_opt) {
                                                        println!("{}, {}", uid, public_key);
                                                        resultat_usb =   Ok(UsbResponse::Success(SuccessData::EnrollmentInfo { uid, public_key }));
                                                    } else {
                                                        resultat_usb = Err("Données incomplètes (SN et PUB manquant)".to_string());
                                                    }
                                                   }, Err(e) => {
                                                    resultat_usb = Err(format!("Echec communication: {}", e));
                                                   }
                                                }
                                            },
                                            Err(e) =>  resultat_usb = Err(format!("Erreur ouverture port: {}", e)),
                                        }
                                    } else {
                                        resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
                                    }
                                }

                                match resultat_usb {
                                    Ok(data) => { let _ = clone_sender.send(ApiMessage::EnrollmentUsbSuccess(data)); }
                                    Err(e) => { let _ = clone_sender.send(ApiMessage::EnrollmentError(format!("Erreur USB: {}", e))); }
                                }
                            });
                        }
                    }

                    else if modif_valid {
                        let btn = egui::Button::new("✏️ Modifier les droits (Email + Rôle)")
                            .min_size(egui::vec2(250.0, 45.0));

                        if ui.add(btn).clicked() {
                            let bypass_usb = true;
                            app.enroll_status = if bypass_usb {
                                "🛠️ SIMULATION : Bypass USB activé...".to_string()
                            } else {
                                "🔌 Recherche de la clé USB...".to_string()
                            };

                            let clone_sender = app.sender.clone();
                            let clone_port_name = app.current_port_name.clone();

                            tokio::spawn(async move {
                                let resultat_usb: Result<UsbResponse, String>;

                                if bypass_usb {
                                    println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    resultat_usb = Ok(UsbResponse::Success(SuccessData::Ack));
                                } else {


                                    if !clone_port_name.is_empty() {
                                        match serialport::new(&clone_port_name, 115200)
                                        .timeout(Duration::from_secs(2))
                                        .open() {
                                            Ok(mut port) => {
                                                resultat_usb = send_command(&mut port, share_protocol::Command::Modify);
                                            },
                                            Err(e) => {
                                                resultat_usb = Err(format!("Impossible d'ouvrir le port {}: {}", clone_port_name, e));
                                            }
                                        }

                                    } else {
                                        resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
                                    }
                                }

                                match resultat_usb {
                                    Ok(data) => { let _ = clone_sender.send(ApiMessage::ModificationUsbSuccess(data)); }
                                    Err(e) => { let _ = clone_sender.send(ApiMessage::EnrollmentError(format!("Erreur USB: {}", e))); }
                                }
                            });
                        }
                    }

                    else {
                        ui.label(egui::RichText::new("Remplissez tous les champs pour enrôler, ou seulement Email + Rôle pour modifier.")
                            .italics()
                            .weak()
                        );
                    }
                });
            });
            ui.add_space(20.0);

            frame_style.show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.heading("Utilisateurs existant");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Actualiser").clicked() {
                            let _ = app.sender.send(ApiMessage::FetchUsers);
                        }
                    });
            });

            egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                egui::Grid::new("user_list_grid")
                .striped(true)
                .spacing([20.0, 10.0])
                .min_col_width(100.0)
                .show(ui, |ui|{
                    ui.strong("Nom");
                    ui.strong("Email");
                    ui.strong("Rôle");
                    ui.strong("Actions");
                    ui.end_row();

                    if app.users_list.is_empty() {
                        ui.label("Aucun utilisateur chargé...");
                        ui.label("-");
                        ui.label("-");
                        ui.label("-");
                        ui.end_row();
                    }else {
                        for (index, user) in app.users_list.iter().enumerate() {
                            ui.label(format!("{} {}", user.first_name, user.last_name));
                            ui.label(&user.email);

                            let color = match user.role {
                                Role::ENROLLEUR => egui::Color32::BLUE,
                                Role::ADMIN => egui::Color32::RED,
                                _ => egui::Color32::GRAY,

                            };
                            let role_text = match user.role {
                                Role::ADMIN => "Administrateur",
                                Role::ENROLLEUR => "Enrôleur",
                                Role::USER => "Utilisateur",
                                Role::NONE => "Aucun"
                            };
                            ui.colored_label(color, role_text);

                            if user.email != app.login_email {
                                if ui.button("Supprimer").on_hover_text("Supprimer cet utilisateur").clicked() {
                               let _ = app.sender.send(ApiMessage::DeleteUser(user.id));
                            }
                            }
                            ui.end_row();
                        }
                    }
                });
                });
            });

            ui.add_space(20.0);

            if !app.enroll_status.is_empty() {
                let color = if app.enroll_status.contains("Erreur") || app.enroll_status.contains("Refus") {
                    egui::Color32::from_rgb(255, 100, 100)
                } else {
                    egui::Color32::from_rgb(100, 200, 255)
                };
                ui.colored_label(color, &app.enroll_status);
            }
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
