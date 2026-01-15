use crate::protocol::{Command, RegisterPayload};
use crate::usb_service::send_command_bindkey;
use crate::{ApiMessage, BindKeyApp, Role};
use eframe::egui;
use serialport::{SerialPortInfo, SerialPortType};
use sha2::{Digest, Sha256};

pub fn show_enrollment_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
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
    ui.add(egui::TextEdit::singleline(&mut app.enroll_password).password(true));
    ui.add_space(20.0);

    egui::ComboBox::from_label("Role")
        .selected_text(format!("{:?}", app.enroll_role))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
            ui.selectable_value(&mut app.enroll_role, Role::ENROLLEUR, "ENROLLEUR");
            ui.selectable_value(&mut app.enroll_role, Role::ADMIN, "ADMIN");
        });

    let formulaire_valide = !app.enroll_firstname.is_empty()
        && !app.enroll_lastname.is_empty()
        && !app.enroll_email.is_empty()
        && !app.enroll_password.is_empty()
        && app.enroll_role != Role::NONE;

    ui.add_space(20.0);

    ui.add_enabled_ui(formulaire_valide, |ui| {
        if ui.button("Validé").clicked() {
            app.enroll_status = "🔌 Recherche de la clé USB...".to_string();

            app.devices.clear();
            if let Ok(liste_devices) = serialport::available_ports() {
                for device in liste_devices {
                    if let SerialPortType::UsbPort(_) = device.port_type {
                        app.devices.push(device);
                    };
                }
            }

            if let Some(device) = app.devices.first() {
                match send_command_bindkey(&device.port_name, Command::StartEnrollment) {
                    Ok(received_data) => {
                        if let Ok(json_value) =
                            serde_json::from_str::<serde_json::Value>(&received_data)
                        {
                            if json_value["status"] == "SUCCESS" {
                                let clone_sender = app.sender.clone();
                                let ctx = ui.ctx().clone();
                                let clone_firstname = app.enroll_firstname.clone();
                                let clone_lastname = app.enroll_lastname.clone();
                                let clone_email = app.enroll_email.clone();
                                let hash_password = hash_password_with_salt(&app.enroll_password);
                                let clone_user_role = app.enroll_role.clone();
                                tokio::spawn(async move {
                                    let payload = RegisterPayload {
                                        first_name: clone_firstname,
                                        last_name: clone_lastname,
                                        email: clone_email,
                                        password: hash_password,
                                        user_role: clone_user_role,
                                    };
                                    let client = reqwest::Client::new();
                                    let resultat = client
                                        .post("http://localhost:3000/users")
                                        .json(&payload)
                                        .send()
                                        .await;
                                    match resultat {
                                        Ok(response) => {
                                            if response.status().is_success() {
                                                let _ = clone_sender.send(
                                                    ApiMessage::EnrollmentSuccess(
                                                        "Enrolé !".to_string(),
                                                    ),
                                                );
                                            } else {
                                                let _ =
                                                    clone_sender.send(ApiMessage::EnrollmentError(
                                                        "Le serveur a dit non".to_string(),
                                                    ));
                                            }
                                        }
                                        Err(e) => {
                                            let _ = clone_sender
                                                .send(ApiMessage::EnrollmentError(e.to_string()));
                                        }
                                    }
                                    ctx.request_repaint();
                                });
                                app.enroll_password.clear();
                            } else {
                                app.enroll_status = "Erreur USB : Empreinte refusée".to_string();
                            }
                        } else {
                            app.enroll_status = "Erreur USB : Réponse invalide".to_string();
                        }
                    }
                    Err(e) => {
                        app.enroll_status = format!("Erreur communication : {}", e);
                    }
                }
            } else {
                app.enroll_status = "Aucune Bindkey détectée. Branchez-là !".to_string();
            }
        };
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
    let result = hasher.finalize();

    format!("{:x}", result)
}
