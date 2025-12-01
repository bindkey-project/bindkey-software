use crate::protocol::Command;
use eframe::{App, egui, run_native};

mod api_service;
mod protocol;
mod usb_service;

#[derive(PartialEq)]
enum EnrollmentState {
    Formulaire,      // Étape 1 : On tape le nom
    AttenteDoigt,    // Étape 2 : On demande à l'utilisateur de poser le doigt
    Communication,   // Étape 3 : Envoi au serveur (Spinner)
    Succes(String),  // Fin : On affiche le résultat
    Erreur(String),  // Oups : On affiche l'erreur
}

struct BindKeyApp {
    status_text: String,
    devices: Vec<usb_service::DeviceInfo>,
    username_input: String,
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
            username_input: String::new(),
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
        
        // --- ÉTAPE 1 : LE FORMULAIRE ---
        EnrollmentState::Formulaire => {
            ui.label("Étape 1/3 : Informations Utilisateur");
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.label("Nom :");
                ui.text_edit_singleline(&mut self.username_input);
            });

            ui.add_space(20.0);

            // Bouton "Suivant" (au lieu d'Enrôler direct)
            if ui.button("Suivant ➡").clicked() {
                if !self.username_input.is_empty() {
                    // On passe à l'étape suivante !
                    self.enroll_state = EnrollmentState::AttenteDoigt;
                    
                    // (Optionnel) Ici tu pourrais envoyer une commande "PREPARE_ENROLL" à la clé
                    // pour qu'elle allume sa LED en bleu.
                }
            }
        },

        // --- ÉTAPE 2 : SCAN BIOMÉTRIQUE ---
        EnrollmentState::AttenteDoigt => {
            ui.label("Étape 2/3 : Capture Biométrique");
            ui.add_space(20.0);
            
            // Un gros texte ou une icône
            ui.colored_label(egui::Color32::YELLOW, "👆 Veuillez demander à l'utilisateur de poser son doigt sur la BindKey.");
            
            ui.add_space(20.0);

            // Simulation de la détection du doigt
            if ui.button("Simuler : Doigt Détecté ✅").clicked() {
                // Ici, dans la vraie vie, tu bouclerais en interrogeant la clé
                // Pour l'instant, on simule que la clé a répondu "OK, j'ai le hash"
                self.enroll_state = EnrollmentState::Communication;
                
                // On lance l'envoi au serveur (simulé ici dans la boucle UI pour l'exemple)
                // Dans une vraie app, on ferait ça en thread, mais restons simple.
                let fake_hash = "hash_biometrique_secure_123".to_string();
                
                match api_service::register_user(self.username_input.clone(), fake_hash) {
                    Ok(msg) => self.enroll_state = EnrollmentState::Succes(msg),
                    Err(e) => self.enroll_state = EnrollmentState::Erreur(e),
                }
            }
            
            if ui.button("Annuler").clicked() {
                self.enroll_state = EnrollmentState::Formulaire;
            }
        },

        // --- ÉTAPE 3 : RÉSULTAT ---
        EnrollmentState::Succes(msg) => {
            ui.colored_label(egui::Color32::GREEN, "✅ Enrôlement Terminé !");
            ui.label(msg);
            
            if ui.button("Enrôler un autre utilisateur").clicked() {
                self.username_input.clear();
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
        
        // Cas Communication (si on avait des threads, on afficherait un spinner ici)
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
