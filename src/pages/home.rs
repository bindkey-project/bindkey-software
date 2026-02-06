use crate::BindKeyApp;
use eframe::egui;

pub fn show_home_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "Bienvenue sur l'App BindKey {}",
                app.first_name_user
            ));

            ui.add_space(10.0);

            ui.label(format!("Votre rôle est : {:?}", app.role_user));

            ui.add_space(30.0);

            ui.add(
                egui::Image::new(egui::include_image!("../../bindkey.png"))
                    .max_width(500.0)
                    .rounding(10.0),
            );
        });
    });
}
