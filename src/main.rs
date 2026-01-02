use eframe::egui;
use serde::{Deserialize, Serialize};
use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::protocol::Command::StartEnrollment;
use crate::usb_service::send_command_bindkey;
mod protocol;
mod usb_service;
use crate::protocol::RegisterPayload;

// device port_name : "/dev/ttyACM0", device port_type :
//UsbPort(UsbPortInfo { vid: 0x1a86, pid: 0x55d3, serial_number: Some("5A47013078"), manufacturer: Some("1a86"), product: Some("USB Single Serial") })
//Info quand bindkey branchée

struct BindKeyApp {
    current_page: Page,
    role_user: Role,
    enroll_firstname: String,
    enroll_lastname: String,
    enroll_email: String,
    enroll_role: Role,
    devices: Vec<SerialPortInfo>,
    receiver: Receiver<ApiMessage>,
    sender: Sender<ApiMessage>,
    status_message: String,
    login_user: String,
    login_password: String,
    is_authenticated: bool,
}

#[derive(PartialEq)]
enum Page {
    Login,
    Home,
    Enrollment,
    Unlock, // Page pour les volumes (à faire plus tard)
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
enum Role {
    USER,
    ENROLLEUR,
    ADMIN,
    NONE,
}

enum ApiMessage {
    Success(String),
    Error(String),
}

impl BindKeyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        BindKeyApp {
            current_page: Page::Login,
            role_user: Role::NONE,
            enroll_firstname: String::new(),
            enroll_lastname: String::new(),
            enroll_email: String::new(),
            enroll_role: Role::NONE,
            devices: Vec::new(),
            receiver: rx,
            sender: tx,
            status_message: String::new(),
            login_user: String::new(),
            login_password: String::new(),
            is_authenticated: false,
        }
    }
    fn show_login_page(&mut self, ui: &mut egui::Ui) {
        
        ui.vertical_centered(|ui| {
            ui.add_space(50.0); 
            ui.heading("BindKey Secure Access");
            ui.add_space(30.0);

            ui.label("Identifiant :");
            ui.add(egui::TextEdit::singleline(&mut self.login_user).hint_text("admin"));
            ui.add_space(10.0);

            ui.label("Mot de passe :");
            ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));
            
            ui.add_space(30.0);

            if ui.button("🔐 Se connecter avec BindKey").clicked() {
                // 1. Vérification simple (Simulation Backend)
                if !self.login_user.is_empty() && !self.login_password.is_empty() {
                    
                    println!("Tentative de connexion pour : {}", self.login_user);

                    self.is_authenticated = true;
                    self.role_user = Role::ADMIN; 
                    self.current_page = Page::Home;
                    self.status_message.clear();
                } else {
                    self.status_message = "Veuillez remplir les champs".to_string();
                }
            }
            
            if !self.status_message.is_empty() {
                ui.add_space(20.0);
                ui.colored_label(egui::Color32::RED, &self.status_message);
            }
        });
    }
    
}



impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
       
         if let Ok(message) = self.receiver.try_recv() {
                match message {
                    ApiMessage::Success(texte) => {
                        self.status_message = format!("{}", texte);
                    }
                    ApiMessage::Error(texte) => {
                        self.status_message = format!("{}", texte);
                    }
                }
            }
       
        if !self.is_authenticated {
           egui::CentralPanel::default().show(ctx, |ui| {
            self.show_login_page(ui);
           });
            
        } else {
            if self.current_page != Page::Login {
                egui::SidePanel::left("menu").show(ctx, |ui| {
                    if ui.button("Accueil").clicked() {
                        self.current_page = Page::Home;
                    };
                    if self.role_user == Role::ENROLLEUR || self.role_user == Role::ADMIN {
                        if ui.button("Enrôlment").clicked() {
                            self.current_page = Page::Enrollment;
                        };
                    };
                    if ui.button("Unlock").clicked() {
                        self.current_page = Page::Unlock;
                    };

                    ui.separator();

                    if ui.button("Déconnexion").clicked() {
                        self.is_authenticated = false;
                        self.role_user = Role::NONE;
                        self.login_password.clear();
                        self.current_page = Page::Login;
                        
                        
                    };
                });
            }

            egui::CentralPanel::default().show(ctx, |ui| match self.current_page {
                Page::Login => {
                        self.current_page = Page::Home;
                }
                Page::Enrollment => {
                    ui.label("Firstname :");
                    ui.text_edit_singleline(&mut self.enroll_firstname);
                    ui.label("Lastname :");
                    ui.text_edit_singleline(&mut self.enroll_lastname);
                    ui.label("Email :");
                    ui.text_edit_singleline(&mut self.enroll_email);
                    egui::ComboBox::from_label("Role")
                        .selected_text(format!("{:?}", self.enroll_role))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.enroll_role, Role::USER, "USER");
                            ui.selectable_value(
                                &mut self.enroll_role,
                                Role::ENROLLEUR,
                                "ENROLLEUR",
                            );
                            ui.selectable_value(&mut self.enroll_role, Role::ADMIN, "ADMIN");
                        });

                    let formulaire_valide = !self.enroll_firstname.is_empty()
                        && !self.enroll_lastname.is_empty()
                        && !self.enroll_email.is_empty()
                        && self.enroll_role != Role::NONE;

                    ui.add_enabled_ui(formulaire_valide, |ui| {
                        if ui.button("Validé").clicked() {
                            println!(
                                "Veuillez scanner le doigt de {} {}",
                                self.enroll_firstname, self.enroll_lastname
                            );
                            if let Some(device) = self.devices.first() {
                                match send_command_bindkey(&device.port_name, StartEnrollment) {
                                    Ok(received_data) => {
                                        if let Ok(json_value) =
                                            serde_json::from_str::<serde_json::Value>(
                                                &received_data,
                                            )
                                        {
                                            if json_value["status"] == "SUCCESS" {
                                                let clone_sender = self.sender.clone();
                                                let clone_firstname = self.enroll_firstname.clone();
                                                let clone_lastname = self.enroll_lastname.clone();
                                                let clone_email = self.enroll_email.clone();
                                                let clone_user_role = self.enroll_role.clone();
                                                tokio::spawn(async move {
                                                    let payload = RegisterPayload {
                                                        first_name: clone_firstname,
                                                        last_name: clone_lastname,
                                                        email: clone_email,
                                                        user_role: clone_user_role,
                                                    };
                                                    let client = reqwest::Client::new();
                                                    let resultat = client
                                                        .post("http://localhost:3000/users")
                                                        .json(&payload)
                                                        .send()
                                                        .await;
                                                    match resultat {
                                                        Ok(response) => {
                                                            if response.status().is_success() {
                                                                let _ = clone_sender.send(
                                                                    ApiMessage::Success(
                                                                        "Enrolé !".to_string(),
                                                                    ),
                                                                );
                                                            } else {
                                                                let _ = clone_sender.send(
                                                                    ApiMessage::Error(
                                                                        "Le serveur a dit non"
                                                                            .to_string(),
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let _ = clone_sender.send(
                                                                ApiMessage::Error(e.to_string()),
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    Err(message_erreur) => {
                                        println!("{}", message_erreur);
                                    }
                                }
                            };
                        };
                    });

                    if !self.status_message.is_empty() {
                        ui.add_space(10.0);
                        ui.label(&self.status_message);
                    }
                }
                Page::Home => {
                    ui.heading(format!("Bienvenue, {}", self.login_user));
                    ui.label(format!("Votre rôle : {:?}", self.role_user));

                    ui.add_space(20.0);

                    if ui.button("Scan des ports").clicked() {
                        self.devices.clear();
                        if let Ok(liste_devices) = serialport::available_ports() {
                            for device in liste_devices {
                                match device.port_type {
                                    SerialPortType::UsbPort(_) => {
                                        self.devices.push(device);
                                    }
                                    _ => {}
                                };
                            }
                        }
                    }
                    for device in &self.devices {
                            ui.label(format!("Detecté : {}", device.port_name));
                        }
                }
                Page::Unlock => {
                    ui.heading("Gestion des Volumes");
                    ui.label("Branchez une clé USB vierge pour créer un volume sécurisé.");
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Client BindKey",
        native_options,
        Box::new(|cc| Ok(Box::new(BindKeyApp::new(cc)))),
    )
}
