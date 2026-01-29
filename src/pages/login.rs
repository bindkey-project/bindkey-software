use crate::{
    ApiMessage, BindKeyApp, ChallengeResponse, LoginPayload,
    pages::enrollment::hash_password_with_salt,
    protocol::{Page, Role},
};
use eframe::egui;
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

                    ui.vertical_centered(|ui| {
                        let btn =
                            egui::Button::new("Se connecter").min_size(egui::vec2(200.0, 40.0));

                        if ui.add(btn).clicked() {
                            handle_login(app, ui.ctx().clone());
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

fn handle_login(app: &mut BindKeyApp, ctx: egui::Context) {
    if app.login_email.is_empty() || !app.login_email.validate_email() {
        app.login_status = " Email invalide".to_string();
        return;
    }
    app.login_status = "⏳ Connexion...".to_string();

    app.role_user = Role::ADMIN;
    app.current_page = Page::Home;

    let clone_sender = app.sender.clone();
    let clone_email = app.login_email.clone();
    let clone_pass = hash_password_with_salt(&app.login_password);
    let clone_url = app.config.api_url.clone();

    tokio::spawn(async move {
        let payload = LoginPayload {
            email: clone_email,
            password_hash: clone_pass,
        };
        let client = reqwest::Client::new();
        let url = format!("{}/sessions/login", clone_url);

        match client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(chall) = response.json::<ChallengeResponse>().await {
                        let _ = clone_sender.send(ApiMessage::ReceivedChallenge(
                            chall.auth_challenge,
                            chall.session_id,
                        ));
                    }
                } else {
                    let _ = clone_sender.send(ApiMessage::LoginError(" Refus Serveur".to_string()));
                }
            }
            Err(e) => {
                let _ = clone_sender.send(ApiMessage::LoginError(e.to_string()));
            }
        }
        ctx.request_repaint();
    });
    app.login_password.clear();
}
