use crate::BindKeyApp;
pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
ui.heading("Gestion des Volumes");
ui.label("Branchez une clé USB vierge pour créer un volume sécurisé.");
}