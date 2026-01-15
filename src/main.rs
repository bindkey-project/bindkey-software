use crate::{protocol::CreationState, usb_service::send_command_bindkey};
use eframe::egui;
use serialport::{SerialPortInfo, SerialPortType};
use std::sync::mpsc::{Receiver, Sender, channel};
mod pages;
mod protocol;
mod usb_service;
use crate::protocol::{
    ApiMessage, ChallengeResponse, LoginPayload, LoginSuccessResponse, Page, Role, VerifyPayload, VolumeInfo,
};

// device port_name : "/dev/ttyACM0", device port_type :
//UsbPort(UsbPortInfo { vid: 0x1a86, pid: 0x55d3, serial_number: Some("5A47013078"), manufacturer: Some("1a86"), product: Some("USB Single Serial") })
//Info quand bindkey branchée

struct BindKeyApp {
    pub current_page: Page,
    pub role_user: Role,
    pub enroll_firstname: String,
    pub enroll_lastname: String,
    pub enroll_email: String,
    pub enroll_password: String,
    pub enroll_role: protocol::Role,
    pub devices: Vec<SerialPortInfo>,
    pub receiver: Receiver<ApiMessage>,
    pub sender: Sender<ApiMessage>,
    pub login_status: String,
    pub enroll_status: String,
    pub login_email: String,
    pub login_password: String,
    pub auth_token: String,
    pub detected_volumes: Vec<VolumeInfo>,
    pub creation_state: CreationState,
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
            enroll_password: String::new(),
            enroll_role: Role::NONE,
            devices: Vec::new(),
            receiver: rx,
            sender: tx,
            login_status: String::new(),
            enroll_status: String::new(),
            login_email: String::new(),
            login_password: String::new(),
            auth_token: String::new(),
            detected_volumes: Vec::new(),
            creation_state: CreationState { 
                is_open: false, 
                selected_disk_index: 0, 
                volume_name: String::new(), 
                volume_size_gb: 1,
                status: String::new(),
            }
        }
    }
}

impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(message) = self.receiver.try_recv() {
            match message {
                ApiMessage::EnrollmentSuccess(texte) => {
                    self.enroll_status = texte.to_string();
                }
                ApiMessage::LoginError(texte) => {
                    self.login_status = texte.to_string();
                }
                ApiMessage::EnrollmentError(texte) => self.enroll_status = texte.to_string(),

                ApiMessage::ReceivedChallenge(le_challenge) => {
                    self.login_status =
                        "Challenge reçue, communication avec la bindkey en cours".to_string();
                    self.devices.clear();
                    if let Ok(liste_devices) = serialport::available_ports() {
                        for device in liste_devices {
                            if let SerialPortType::UsbPort(_) = device.port_type {
                                self.devices.push(device);
                            };
                        }
                    }
                    if let Some(device) = self.devices.first() {
                        let clone_sender = self.sender.clone();
                        let port_name = device.port_name.clone();
                        tokio::spawn(async move {
                            match send_command_bindkey(
                                &port_name,
                                protocol::Command::SignChallenge(le_challenge),
                            ) {
                                Ok(response) => {
                                    let _ =
                                        clone_sender.send(ApiMessage::SignedChallenge(response));
                                }
                                Err(message_erreur) => {
                                    let _ =
                                        clone_sender.send(ApiMessage::LoginError(message_erreur));
                                }
                            }
                        });
                    } else {
                        self.login_status = "Aucune BindKey détectée. Branchez la clé.".to_string();
                    }
                }
                ApiMessage::SignedChallenge(signature) => {
                    self.login_status =
                        "Signature générée. Vérification finale auprès du serveur".to_string();
                    let clone_email = self.login_email.clone();
                    let clone_signature = signature.clone();
                    let clone_sender = self.sender.clone();

                    tokio::spawn(async move {
                        let payload = VerifyPayload {
                            email: clone_email,
                            signature: clone_signature,
                        };
                        let client = reqwest::Client::new();
                        let resultat = client
                            .post("http://localhost:3000/login/verify")
                            .json(&payload)
                            .send()
                            .await;
                        match resultat {
                            Ok(response) => {
                                if response.status().is_success() {
                                    match response.json::<LoginSuccessResponse>().await {
                                        Ok(response) => {
                                            let _ = clone_sender.send(ApiMessage::LoginSuccess(
                                                response.role,
                                                response.token,
                                            ));
                                        }
                                        Err(e) => {
                                            let _ = clone_sender
                                                .send(ApiMessage::LoginError(e.to_string()));
                                        }
                                    }
                                } else {
                                    let _ = clone_sender.send(ApiMessage::LoginError(
                                        "Signature refusée par le serveur".to_string(),
                                    ));
                                }
                            }
                            Err(error) => {
                                let _ =
                                    clone_sender.send(ApiMessage::LoginError(error.to_string()));
                            }
                        }
                    });
                }
                ApiMessage::LoginSuccess(role, token) => {
                    self.role_user = role;
                    self.auth_token = token;

                    self.login_status = String::new();
                    self.login_password = String::new();

                    self.current_page = Page::Home;
                }
            }
        }

        if self.current_page != Page::Login {
            egui::SidePanel::left("menu").show(ctx, |ui| {
                if ui.button("Accueil").clicked() {
                    self.current_page = Page::Home;
                };
                if (self.role_user == Role::ENROLLEUR || self.role_user == Role::ADMIN)
                    && ui.button("Enrôlment").clicked()
                {
                    self.current_page = Page::Enrollment;
                };
                if ui.button("Unlock").clicked() {
                    self.current_page = Page::Unlock;
                };

                ui.separator();

                if ui.button("Déconnexion").clicked() {
                    self.current_page = Page::Login;
                    self.role_user = Role::NONE;
                    self.login_password.clear();
                };
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.current_page {
            Page::Login => {
                pages::login::show_login_page(self, ui);
            }
            Page::Enrollment => {
                pages::enrollment::show_enrollment_page(self, ui);
            }
            Page::Home => {
                pages::home::show_home_page(self, ui);
            }
            Page::Unlock => {
                pages::volumes::show_volumes_page(self, ui);
            }
        });
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
