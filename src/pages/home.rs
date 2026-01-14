use eframe::egui;
use crate::BindKeyApp;

pub fn show_home_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.label(format!("Votre rôle : {:?}", app.role_user));

    ui.add_space(20.0);
}
