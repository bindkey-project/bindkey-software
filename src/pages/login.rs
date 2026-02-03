use std::time::Duration;

use crate::protocol::protocol::{ApiMessage, ChallengeResponse, LoginSuccessResponse};
use crate::usb_service::send_text_command;
use crate::{BindKeyApp, pages::enrollment::hash_password_with_salt};
use eframe::egui;
use serde_json::json;
use validator::ValidateEmail;

pub fn show_login_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    let area_id = ui.make_persistent_id("login_area");

    egui::Area::new(area_id)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .movable(false)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(400.0);

            egui::Frame::none()
                .fill(ui.visuals().window_fill())
                .rounding(10.0)
                .stroke(ui.visuals().window_stroke())
                .inner_margin(30.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("🔐 BindKey Access");
                        ui.add_space(20.0);
                    });

                    ui.label("Email professionnel :");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.login_email)
                            .hint_text("jean.mattei@entreprise.fr")
                            .min_size(egui::vec2(340.0, 32.0)),
                    );

                    ui.add_space(15.0);

                    ui.label("Mot de passe :");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.login_password)
                            .password(true)
                            .min_size(egui::vec2(340.0, 32.0)),
                    );

                    ui.add_space(30.0);

                    ui.checkbox(&mut app.is_admin_mode, "Mode administrateur (Sans USB)");

                    ui.add_space(30.0);

                    ui.vertical_centered(|ui| {
                        let btn_text = if app.is_admin_mode {
                            "Se connecter (Admin)"
                        } else {
                            "Se connecter avec Clé"
                        };

                        let btn = egui::Button::new(btn_text).min_size(egui::vec2(200.0, 40.0));

                        if ui.add(btn).clicked() {
                            if app.is_admin_mode {
                                handle_admin_login(app);
                            } else {
                                handle_login(app, ui.ctx().clone());
                            }
                        }

                        ui.add_space(20.0);

                        if !app.login_status.is_empty() {
                            let color = if app.login_status.contains("cours") {
                                egui::Color32::from_rgb(100, 200, 255)
                            } else {
                                egui::Color32::from_rgb(255, 100, 100)
                            };
                            ui.colored_label(color, &app.login_status);
                        }
                    });
                });
        });
}

fn handle_admin_login(app: &mut BindKeyApp) {
    if app.login_email.is_empty()
        || app.login_password.is_empty()
        || !app.login_email.validate_email()
    {
        app.login_status = "Champs invalides".to_string();
        return;
    }
    app.login_status = "Authentification Admin en cours...".to_string();

    let clone_sender = app.sender.clone();
    let clone_email = app.login_email.clone();
    let clone_pass = hash_password_with_salt(&app.login_password);
    let clone_url = app.config.api_url.clone();
    let clone_api_client = app.api_client.clone();

    tokio::spawn(async move {

        let url = format!("{}/sessions/test", clone_url);
        let payload = json!({
            "email": clone_email,
            "password": clone_pass
        });

        match clone_api_client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(data) = response.json::<LoginSuccessResponse>().await {
                        let _ = clone_sender.send(ApiMessage::LoginSuccess(
                            data.role,
                            data.server_token,
                            data.first_name,
                            data.local_token,
                        ));
                    } else {
                        let _ = clone_sender.send(ApiMessage::LoginError(
                            "Erreur format réponse serveur".into(),
                        ));
                    }
                } else {
                    let _ = clone_sender.send(ApiMessage::LoginError(format!(
                        "Refus Admin: {}",
                        response.status()
                    )));
                }
            }
            Err(e) => {
                let _ = clone_sender.send(ApiMessage::LoginError(format!("Erreur réseau : {}", e)));
            }
        }
    });
    app.login_password.clear();
}

fn handle_login(app: &mut BindKeyApp, ctx: egui::Context) {
    if app.login_email.is_empty()
        || app.login_password.is_empty()
        || !app.login_email.validate_email()
    {
        app.login_status = " Champs invalides".to_string();
        return;
    }
    if !app.usb_connected {
        app.login_status = " Veuillez brancher votre BindKey".to_string();
        return;
    }

    app.login_status = " Lecture de la BindKey...".to_string();

    let clone_sender = app.sender.clone();
    let clone_email = app.login_email.clone();
    let clone_pass = hash_password_with_salt(&app.login_password);
    let clone_url = app.config.api_url.clone();
    let clone_port_name = app.current_port_name.clone();
    let clone_api_client = app.api_client.clone();
    let bypass_usb = false;

    tokio::spawn(async move {
        let mut bindkey_uid = String::new();

        if bypass_usb {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            bindkey_uid = "SIMULATED-BK-UID-999".to_string();
        } else {
            if !clone_port_name.is_empty() {
                match serialport::new(&clone_port_name, 115200)
                    .timeout(Duration::from_secs(2))
                    .open()
                {
                    Ok(mut port) => {
                        let _ = port.write_data_terminal_ready(true);
                        std::thread::sleep(Duration::from_millis(100));

                        match send_text_command(&mut *port, "uid") {
                            Ok(map) => {
                                if let Some(sn) = map.get("SN") {
                                    bindkey_uid = sn.clone();
                                } else {
                                    let _ = clone_sender.send(ApiMessage::LoginError(
                                        "Clé muette (SN manquant)".into(),
                                    ));
                                    return;
                                }
                            }
                            Err(e) => {
                                let _ = clone_sender.send(ApiMessage::LoginError(format!(
                                    "Erreur lecture Clé: {}",
                                    e
                                )));
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender
                            .send(ApiMessage::LoginError(format!("Erreur Lecture Clé: {}", e)));
                    }
                }
            } else {
                let _ = clone_sender.send(ApiMessage::LoginError("Port introuvable".to_string()));
            }
        }

        let _ = clone_sender.send(ApiMessage::LoginError(
            "UID récupéré, envoi au serveur...".to_string(),
        ));

        let payload = json!( {
            "email": clone_email,
            "password": clone_pass,
            "bindkey_id": bindkey_uid,
        });

        let url = format!("{}/sessions/login", clone_url);

        match clone_api_client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(chall) = response.json::<ChallengeResponse>().await {
                        let _ = clone_sender.send(ApiMessage::ReceivedChallenge(
                            chall.auth_challenge,
                            chall.session_id,
                        ));
                    }
                } else {
                    let _ = clone_sender.send(ApiMessage::LoginError(format!(
                        "Refus Serveur (Clé inconnue ?): {}",
                        response.status()
                    )));
                }
            }
            Err(e) => {
                let _ = clone_sender.send(ApiMessage::LoginError(format!("Erreur Réseau: {}", e)));
            }
        }
    });
    app.login_password.clear();
}
