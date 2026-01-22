use crate::protocol::{Command, ModifyPayload, RegisterPayload};
use crate::usb_service::{get_bindkey, send_command_bindkey};
use crate::{API_URL, ApiMessage, BindKeyApp, Role};
use eframe::egui;
use serialport::{SerialPortInfo, SerialPortType};
use sha2::{Digest, Sha256};
use validator::{self, ValidateEmail, ValidateLength};

pub fn show_enrollment_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.label("Firstname :");
        ui.text_edit_singleline(&mut app.enroll_firstname);
        ui.add_space(20.0);

        ui.label("Lastname :");
        ui.text_edit_singleline(&mut app.enroll_lastname);
        ui.add_space(20.0);

        ui.label("Email :");
        ui.text_edit_singleline(&mut app.enroll_email);
        ui.add_space(20.0);

        ui.label("Password :");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.enroll_password).password(true));
            ui.label("Le mdp doit faire au minimum 14 caractères.");
        });

        ui.add_space(20.0);

        egui::ComboBox::from_label("Role")
            .selected_text(format!("{:?}", app.enroll_role))
            .show_ui(ui, |ui| {
                if app.role_user == Role::ADMIN {
                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                    ui.selectable_value(&mut app.enroll_role, Role::ENROLLEUR, "ENROLLEUR");
                } else if app.role_user == Role::ENROLLEUR {
                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                }
            });
    });
    let formulaire_valide = !app.enroll_firstname.is_empty()
        && !app.enroll_lastname.is_empty()
        && !app.enroll_email.is_empty()
        && app.enroll_email.validate_email()
        && !app.enroll_password.is_empty()
        && app.enroll_password.validate_length(Some(14), None, None)
        && app.enroll_role != Role::NONE;

    ui.add_space(20.0);

    ui.add_enabled_ui(formulaire_valide, |ui| {
        if ui.button("Validé").clicked() {
            let resultat_usb = get_bindkey(app, ui, Command::StartEnrollment);

            match resultat_usb {
                Ok(received_data) => {
                    if let Ok(json_value) =
                        serde_json::from_str::<serde_json::Value>(&received_data)
                    {
                        if json_value["status"] == "SUCCESS" {
                            let bk_pk = json_value["public_key"]
                                .as_str()
                                .unwrap_or("Unknown PK")
                                .to_string();

                            let bk_uid = json_value["uid"]
                                .as_str()
                                .unwrap_or("Unknown Uid")
                                .to_string();

                            let clone_sender = app.sender.clone();
                            let ctx = ui.ctx().clone();
                            let clone_firstname = app.enroll_firstname.clone();
                            let clone_lastname = app.enroll_lastname.clone();
                            let clone_email = app.enroll_email.clone();
                            let hash_password = hash_password_with_salt(&app.enroll_password);
                            let clone_user_role = app.enroll_role.clone();
                            let clone_bk_pk = bk_pk.clone();
                            let clone_bk_uid = bk_uid.clone();
                            println!("{:?}", clone_user_role);

                            tokio::spawn(async move {
                                let payload = RegisterPayload {
                                    first_name: clone_firstname,
                                    last_name: clone_lastname,
                                    email: clone_email,
                                    password: hash_password,
                                    user_role: clone_user_role,
                                    bindkey_status: crate::protocol::StatusBindkey::ACTIVE,
                                    public_key: clone_bk_pk,
                                    bindkey_uid: clone_bk_uid,
                                };
                                let client = reqwest::Client::new();
                                let url = format!("{}/users", API_URL);
                                let resultat = client.post(&url).json(&payload).send().await;

                                match resultat {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            let _ =
                                                clone_sender.send(ApiMessage::EnrollmentSuccess(
                                                    " Enrolé (API OK) !".to_string(),
                                                ));
                                        } else {
                                            let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                                " Refus serveur (API KO)".to_string(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                            format!(" Erreur Réseau : {}", e),
                                        ));
                                    }
                                }
                                ctx.request_repaint();
                            });
                            app.enroll_password.clear();
                        } else {
                            app.enroll_status = " Erreur USB : Statut non SUCCESS".to_string();
                        }
                    } else {
                        app.enroll_status = " Erreur USB : JSON invalide".to_string();
                    }
                }
                Err(e) => {
                    app.enroll_status = format!(" {}", e);
                }
            }
        };
    });

    let modif_valid = !app.enroll_email.is_empty()
        && app.enroll_role != Role::NONE
        && app.enroll_email.validate_email()
        && app.enroll_firstname.is_empty()
        && app.enroll_lastname.is_empty()
        && app.enroll_password.is_empty();

    ui.add_enabled_ui(modif_valid, |ui| {
        if ui.button("Modifié").clicked() {
            let resultat_usb = get_bindkey(app, ui, Command::Modify);

            match resultat_usb {
                Ok(received_data) => {
                    if let Ok(json_value) =
                        serde_json::from_str::<serde_json::Value>(&received_data)
                    {
                        if json_value["status"] == "SUCCESS" {
                            let clone_sender = app.sender.clone();
                            let ctx = ui.ctx().clone();
                            let clone_email = app.enroll_email.clone();
                            let clone_user_role = app.enroll_role.clone();

                            tokio::spawn(async move {
                                let payload = ModifyPayload {
                                    email: clone_email,
                                    user_role: clone_user_role,
                                };
                                let client = reqwest::Client::new();
                                let url = format!("{}/users", API_URL);
                                let resultat = client.put(&url).json(&payload).send().await;

                                match resultat {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            let _ =
                                                clone_sender.send(ApiMessage::EnrollmentSuccess(
                                                    " Rôle de l'utilisateur modifié !".to_string(),
                                                ));
                                        } else {
                                            let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                                " Refus serveur (API KO)".to_string(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                            format!(" Erreur Réseau : {}", e),
                                        ));
                                    }
                                }
                                ctx.request_repaint();
                            });
                            app.enroll_password.clear();
                        } else {
                            app.enroll_status = " Erreur USB : Statut non SUCCESS".to_string();
                        }
                    } else {
                        app.enroll_status = " Erreur USB : JSON invalide".to_string();
                    }
                }
                Err(e) => {
                    app.enroll_status = format!(" {}", e);
                }
            }
        }
    });

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        if !app.enroll_status.is_empty() {
            ui.colored_label(egui::Color32::BLUE, &app.enroll_status);
        }
    });
}

pub fn hash_password_with_salt(password: &str) -> String {
    let salt = "bindkey.com";
    let combined = format!("{}{}", password, salt);
    let mut hasher = Sha256::new();
    hasher.update(combined);
    format!("{:x}", hasher.finalize())
}
