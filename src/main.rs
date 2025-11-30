use crate::protocol::Command;
use eframe::{App, egui, run_native};

mod api_service;
mod protocol;
mod usb_service;

struct BindKeyApp {
    status_text: String,
    devices: Vec<usb_service::DeviceInfo>,
    username_input: String,
    current_page: Page,
    is_unlocked: bool,
}

#[derive(PartialEq)]
enum Page {
    Home,
    Enrollment,
    Unlock,
    Volumes,
}

impl BindKeyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            status_text: "Prêt.".to_owned(),
            devices: Vec::new(),
            username_input: String::new(),
            current_page: Page::Home,
            is_unlocked: false,
        }
    }
}

impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("menu_side_panel").show(ctx, |ui| {
            ui.heading("Menu");
            ui.separator();

            if ui.button("🏠 Accueil").clicked() {
                self.current_page = Page::Home;
            }
            if ui.button("🚀 Enrôlement").clicked() {
                self.current_page = Page::Enrollment;
            }
            if ui.button("🔓 Déverrouiller").clicked() {
                self.current_page = Page::Unlock;
            }
            if ui.button("💾 Volumes").clicked() {
                self.current_page = Page::Volumes;
            }

            ui.add_space(20.0);
            ui.separator();
            ui.label("État système :");
            ui.label(&self.status_text);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.current_page {
            Page::Home => {
                ui.heading("Bienvenu sur BindKey Manager");
                ui.label("Sélectionner une action dans le menu");

                if ui.button("🔄 Scanner les ports USB").clicked() {
                    self.devices = usb_service::list_available_ports();
                }

                for device in &self.devices {
                    ui.label(format!("🔌 {}", device.description));
                }
            }

            Page::Enrollment => {
                ui.heading("Enroller un nouvel utilisateur");
                ui.horizontal(|ui| {
                    ui.label("Nom du nouvel utilisateur :");
                    ui.text_edit_singleline(&mut self.username_input);
                });
                if ui.button("🚀 Enrôler un nouvel utilisateur").clicked() {
                    if let Some(device) = self.devices.first() {
                        let cmd = Command::StartEnrollment {
                            username: self.username_input.clone().to_string(),
                        };

                        match usb_service::send_command(&device.port_name, cmd) {
                            Ok(reponse_usb) => {
                                println!(" [MAIN] USB OK : {}", reponse_usb);

                                let fake_hash = "hash_du_doigt_simulé".to_string();

                                match api_service::register_user(
                                    self.username_input.clone(),
                                    fake_hash,
                                ) {
                                    Ok(msg) => {
                                        self.status_text = format!("Tout est bon ! {}", msg);
                                    }
                                    Err(e) => {
                                        self.status_text = format!("Erreur API : {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                self.status_text = format!("Erreur : {}", e);
                            }
                        }
                    } else {
                        self.status_text = "Erreur : Aucune clé sélectionnée !".to_string();
                    }
                }
            }
            Page::Unlock => {
                ui.heading("État de la BindKey");
                ui.add_space(20.0);

                if self.is_unlocked {
                    ui.colored_label(egui::Color32::GREEN, "🔓 DÉVERROUILLÉ - PRÊT À L'EMPLOI");
                    ui.label("Le volume sécurisé est monté et accessible.");
                } else {
                    ui.colored_label(egui::Color32::RED, "🔒 VERROUILLÉ");
                    ui.label("En attente d'authentification biométrique...");
                }

                ui.separator();

                if ui.button("🔄 Vérifier le statut").clicked() {
                    if let Some(device) = self.devices.first() {
                        let cmd = crate::protocol::Command::GetStatus;

                        match usb_service::send_command(&device.port_name, cmd) {
                            Ok(json_response) => {
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(&json_response)
                                {
                                    if parsed["status"] == "UNLOCKED" {
                                        self.is_unlocked = true;
                                    } else {
                                        self.is_unlocked = false;
                                    }
                                    self.status_text =
                                        format!("Statut reçu : {}", parsed["status"]);
                                }
                            }
                            Err(e) => self.status_text = e,
                        }
                    }
                }

                if ui.button("🔑 Simuler Déverrouillage (Admin)").clicked() {
                    if let Some(device) = self.devices.first() {
                        let cmd = crate::protocol::Command::Unlock {
                            token: "1234".to_string(),
                        };

                        match usb_service::send_command(&device.port_name, cmd) {
                            Ok(json_response) => {
                                // --- ESPION ---
                                println!("DEBUG: J'ai reçu de l'USB : '{}'", json_response);
                                // --------------

                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(&json_response)
                                {
                                    // --- ESPION 2 ---
                                    println!("DEBUG: Le champ status est : {:?}", parsed["status"]);
                                    // ----------------

                                    if parsed["status"] == "UNLOCKED" {
                                        println!("DEBUG: C'est gagné, je passe au vert !");
                                        self.is_unlocked = true;
                                        self.status_text =
                                            "Succès : Clé déverrouillée !".to_string();
                                    } else {
                                        println!("DEBUG: Ce n'est pas égal à 'UNLOCKED'");
                                    }
                                }
                            }
                            Err(e) => self.status_text = e,
                        }
                    }
                }
            }
            Page::Volumes => {
                ui.heading("Gestion des volumes securisés");
                ui.label("Fonctionnalité pas encore disponible");
            }
        });
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    run_native(
        "BindKey Client",
        native_options,
        Box::new(|cc| Ok(Box::new(BindKeyApp::new(cc)))),
    )
}
