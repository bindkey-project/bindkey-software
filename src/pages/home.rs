use crate::BindKeyApp;
use eframe::egui;

pub fn show_home_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.label(format!("Bonjour {:?}", app.first_name_user));
    ui.add_space(20.0);

    ui.label(format!("Votre rôle : {:?}", app.role_user));

    ui.add_space(20.0);
}
