use crate::{ApiMessage, BindKeyApp, ChallengeResponse, LoginPayload, pages::enrollment::hash_password_with_salt};
use eframe::egui;

pub fn show_login_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(50.0);
        ui.heading("BindKey Secure Access");
        ui.add_space(30.0);

        ui.label("Email :");
        ui.add(
            egui::TextEdit::singleline(&mut app.login_email).hint_text("jean.mattei@entreprise.fr"),
        );
        ui.add_space(10.0);

        ui.label("Mot de passe :");
        ui.add(egui::TextEdit::singleline(&mut app.login_password).password(true));

        ui.add_space(30.0);

        if ui.button(" Se connecter avec BindKey").clicked() {
            if app.login_email.is_empty() || app.login_password.is_empty() {
                app.login_status = "Veuillez remplir tous les champs".to_string();
            } else {
                app.login_status = "Connexion en cours...".to_string();
                app.role_user = crate::protocol::Role::ADMIN;
                app.current_page = crate::protocol::Page::Home;
                
                let clone_sender = app.sender.clone();
                let clone_login_email = app.login_email.clone();
                let clone_login_password = hash_password_with_salt(&app.login_password);
                let ctx = ui.ctx().clone();

                tokio::spawn(async move {
                    let payload = LoginPayload {
                        email: clone_login_email,
                        password: clone_login_password,
                    };
                    let client = reqwest::Client::new();
                    let resultat = client
                        .post("http://localhost:3000/login")
                        .json(&payload)
                        .send()
                        .await;
                    match resultat {
                        Ok(response) => {
                            if response.status().is_success() {
                                let challenge = response.json::<ChallengeResponse>().await;
                                match challenge {
                                    Ok(chall) => {
                                        let le_challenge = chall.challenge;
                                        let _ = clone_sender
                                            .send(ApiMessage::ReceivedChallenge(le_challenge));
                                    }
                                    Err(_) => {
                                        let _ = clone_sender.send(ApiMessage::LoginError(
                                            "Erreur de communication avec le serveur".to_string(),
                                        ));
                                    }
                                }
                            } else {
                                let _ = clone_sender.send(ApiMessage::LoginError(
                                    "Identifiants invalides".to_string(),
                                ));
                            }
                        }
                        Err(_) => {
                            let _ = clone_sender.send(ApiMessage::LoginError("Impossible de se connecter au serveur".to_string()));
                        }
                    }
                    ctx.request_repaint();
                });
            }
            app.login_password.clear();
        }
    });
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        if app.login_status.contains("cours") {
            ui.colored_label( egui::Color32::BLUE, &app.login_status);
        } else {
            ui.colored_label(egui::Color32::RED, &app.login_status);
        }
    });
}
