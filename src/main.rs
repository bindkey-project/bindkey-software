use crate::protocol::Command;
use eframe::{App, egui, run_native};

mod api_service;
mod protocol;
mod usb_service;

#[derive(PartialEq)]
enum EnrollmentState {
    Formulaire,     
    AttenteDoigt,    
    Communication,   
    Succes(String),  
    Erreur(String),  
}

struct BindKeyApp {
    status_text: String,
    devices: Vec<usb_service::DeviceInfo>,
    input_nom: String,
    input_prenom: String,
    input_poste: String,    
    current_page: Page,
    is_unlocked: bool,
    user_role: UserRole,
    enroll_state: EnrollmentState,
}

#[derive(PartialEq)]
enum Page {
    Login,
    Home,
    Enrollment,
    Unlock,
    Volumes,
}

#[derive(PartialEq)]
enum UserRole {
    None,
    User,
    Enroller,
}

impl BindKeyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            status_text: "Prêt.".to_owned(),
            devices: Vec::new(),
            input_nom: String::new(),
            input_prenom: String::new(),
            input_poste: String::new(),
            current_page: Page::Login,
            is_unlocked: false,
            user_role: UserRole::None,
            enroll_state: EnrollmentState::Formulaire,
        }
    }
}

impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.current_page == Page::Login {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("🔐 Connexion au Client BindKey");
                    ui.add_space(20.0);
                    ui.label("Veuillez sélectionner votre rôle (Simulation) :");
                    ui.add_space(20.0);

                    if ui.button("👤 Je suis un Utilisateur").clicked() {
                        self.user_role = UserRole::User;
                        self.current_page = Page::Home;
                    }

                    ui.add_space(10.0);

                    if ui.button("🛡️ Je suis un Enrôleur").clicked() {
                        self.user_role = UserRole::Enroller;
                        self.current_page = Page::Home;
                    }
                });
            });
            return;
        }

        egui::SidePanel::left("menu_side_panel").show(ctx, |ui| {
            ui.heading("Menu");
            ui.separator();

            if ui.button("🏠 Accueil").clicked() {
                self.current_page = Page::Home;
            }

            ui.add_space(10.0);

            if self.user_role == UserRole::Enroller {
                if ui.button("🚀 Enrôlement").clicked() {
                    self.current_page = Page::Enrollment;
                }
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

            ui.add_space(20.0);
            ui.separator();
            if ui.button("Déconnexion").clicked() {
                self.user_role = UserRole::None;
                self.current_page = Page::Login;
            }
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
    ui.heading("Assistant d'Enrôlement");
    ui.separator();

    match &self.enroll_state {
       
EnrollmentState::Formulaire => {
    ui.heading("Étape 1/3 : Informations Collaborateur");
    ui.add_space(10.0);
    
    // On utilise une Grid pour aligner "Label | Champ" proprement
    egui::Grid::new("enroll_form_grid")
        .num_columns(2)
        .spacing([10.0, 10.0]) // Espace entre les colonnes et lignes
        .striped(true)
        .show(ui, |ui| {
            
            ui.label("Nom :");
            ui.text_edit_singleline(&mut self.input_nom);
            ui.end_row(); // Fin de la ligne

            ui.label("Prénom :");
            ui.text_edit_singleline(&mut self.input_prenom);
            ui.end_row();

            ui.label("Poste / Fonction :");
            ui.text_edit_singleline(&mut self.input_poste);
            ui.end_row();
        });

    ui.add_space(20.0);

    // Validation : On vérifie que les 3 champs sont remplis
    let form_is_valid = !self.input_nom.is_empty() 
                     && !self.input_prenom.is_empty() 
                     && !self.input_poste.is_empty();

    ui.add_enabled_ui(form_is_valid, |ui| {
        if ui.button("Suivant ➡").clicked() {
            // On passe à l'étape suivante (le doigt)
            self.enroll_state = EnrollmentState::AttenteDoigt;
        }
    });

    if !form_is_valid {
        ui.small(egui::RichText::new("* Tous les champs sont obligatoires").color(egui::Color32::RED));
    }
},

        EnrollmentState::AttenteDoigt => {
            ui.label("Étape 2/3 : Capture Biométrique");
            ui.add_space(20.0);
            
            ui.colored_label(egui::Color32::YELLOW, "👆 Veuillez demander à l'utilisateur de poser son doigt sur la BindKey.");
            
            ui.add_space(20.0);

           if ui.button("Simuler : Doigt Détecté ✅").clicked() {
    self.enroll_state = EnrollmentState::Communication;
    
    let fake_hash = "hash_biometrique_secure_123".to_string();
    
    // --- CONCATÉNATION DES INFOS ---
    let identity_complete = format!("{} {} ({})", 
        self.input_nom.to_uppercase(), 
        self.input_prenom, 
        self.input_poste
    );
    
    // On envoie l'identité complète
    match api_service::register_user(identity_complete, fake_hash) {
        Ok(msg) => self.enroll_state = EnrollmentState::Succes(msg),
        Err(e) => self.enroll_state = EnrollmentState::Erreur(e),
    }
}
            
            if ui.button("Annuler").clicked() {
                self.enroll_state = EnrollmentState::Formulaire;
            }
        },

        EnrollmentState::Succes(msg) => {
    // ...
    if ui.button("Enrôler un autre utilisateur").clicked() {
        // RESET DES CHAMPS
        self.input_nom.clear();
        self.input_prenom.clear();
        self.input_poste.clear();
        
        self.enroll_state = EnrollmentState::Formulaire;
    }
},

        EnrollmentState::Erreur(err) => {
            ui.colored_label(egui::Color32::RED, "❌ Une erreur est survenue");
            ui.label(err);
            
            if ui.button("Réessayer").clicked() {
                self.enroll_state = EnrollmentState::Formulaire;
            }
        },
        
        EnrollmentState::Communication => {
            ui.label("Envoi au serveur en cours...");
            ui.spinner();
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
                                println!("DEBUG: J'ai reçu de l'USB : '{}'", json_response);

                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(&json_response)
                                {
                                    println!("DEBUG: Le champ status est : {:?}", parsed["status"]);

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
            _ => {}
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
