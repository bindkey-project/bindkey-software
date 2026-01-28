pub const API_URL: &str = "https://api.bindkey.local";
use crate::share_protocol::{SuccessData, UsbResponse};
use crate::usb_service::send_command_bindkey;
use eframe::egui;
use serialport::{SerialPortInfo, SerialPortType};

use std::sync::mpsc::{Receiver, Sender, channel};
mod config;
mod pages;
mod protocol;
mod share_protocol;
mod usb_service;
use crate::config::AppConfig;
use crate::pages::enrollment::hash_password_with_salt;
use crate::protocol::{
    ApiMessage, ChallengeResponse, LoginPayload, LoginSuccessResponse, ModifyPayload, Page,
    RegisterPayload, Role, VerifyPayload, VolumeCreatedInfo,
};
use validator::Validate;

// device port_name : "/dev/ttyACM0", device port_type :
//UsbPort(UsbPortInfo { vid: 0x1a86, pid: 0x55d3, serial_number: Some("5A47013078"), manufacturer: Some("1a86"), product: Some("USB Single Serial") })
//Info quand bindkey branchée

#[derive(Validate)]
struct BindKeyApp {
    pub current_page: Page,
    pub first_name_user: String,
    pub role_user: Role,
    pub enroll_firstname: String,
    pub enroll_lastname: String,
    #[validate(email)]
    pub enroll_email: String,
    #[validate(length(min = 14))]
    pub enroll_password: String,
    pub enroll_role: protocol::Role,
    pub device_name: String,
    pub device_size: String,
    pub device_available_space: u32,
    pub volume_created_name: String,
    pub volume_created_size: u32,
    pub receiver: Receiver<ApiMessage>,
    pub sender: Sender<ApiMessage>,
    pub login_status: String,
    pub enroll_status: String,
    pub volume_status: String,
    #[validate(email)]
    pub login_email: String,
    pub login_password: String,
    pub auth_token: String,
    pub config: AppConfig,
}

impl BindKeyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        let config = AppConfig::load();
        BindKeyApp {
            current_page: Page::Login,
            first_name_user: String::new(),
            role_user: Role::NONE,
            enroll_firstname: String::new(),
            enroll_lastname: String::new(),
            enroll_email: String::new(),
            enroll_password: String::new(),
            enroll_role: Role::NONE,
            device_name: String::new(),
            device_size: String::new(),
            device_available_space: 0,
            volume_created_name: String::new(),
            volume_created_size: 1,
            receiver: rx,
            sender: tx,
            login_status: String::new(),
            enroll_status: String::new(),
            volume_status: String::new(),
            login_email: String::new(),
            login_password: String::new(),
            auth_token: String::new(),
            config,
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
                ApiMessage::EnrollmentUsbSuccess(data) => {
                    match serde_json::from_str::<UsbResponse>(&data) {
                        Ok(UsbResponse::Success(SuccessData::EnrollmentInfo {
                            uid,
                            public_key,
                        })) => {
                            let clone_sender = self.sender.clone();
                            let clone_firstname = self.enroll_firstname.clone();
                            let clone_lastname = self.enroll_lastname.clone();
                            let clone_email = self.enroll_email.clone();
                            let hash_password = hash_password_with_salt(&self.enroll_password);
                            let clone_user_role = self.enroll_role.clone();
                            let clone_auth_token = self.auth_token.clone();
                            let clone_bk_pk = public_key;
                            let clone_bk_uid = uid;
                            let clone_url = self.config.api_url.clone();

                            tokio::spawn(async move {
                                let payload = RegisterPayload {
                                    first_name: clone_firstname,
                                    last_name: clone_lastname,
                                    email: clone_email,
                                    password: hash_password,
                                    user_role: clone_user_role,
                                    bindkey_status: crate::protocol::StatusBindkey::ACTIVE,
                                    public_key: clone_bk_pk,
                                    bindkey_uid: clone_bk_uid,
                                };
                                let client = reqwest::Client::new();
                                let url = format!("{}/users", clone_url);
                                let resultat = client
                                    .post(&url)
                                    .json(&payload)
                                    .bearer_auth(clone_auth_token)
                                    .send()
                                    .await;

                                match resultat {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            let _ =
                                                clone_sender.send(ApiMessage::EnrollmentSuccess(
                                                    " Enrolé (API OK) !".to_string(),
                                                ));
                                        } else {
                                            let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                                " Refus serveur (API KO)".to_string(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                            format!(" Erreur Réseau : {}", e),
                                        ));
                                    }
                                }
                            });
                            self.enroll_password.clear();
                        }
                        Ok(UsbResponse::Error(msg)) => {
                            self.enroll_status = format!(" Erreur Clé USB : {}", msg);
                        }
                        Ok(_) => {
                            self.enroll_status =
                                "Erreur Protocole : Données inattendues reçues".to_string();
                        }
                        Err(e) => {
                            self.enroll_status = format!("Erreur lecture JSON : {}", e);
                        }
                    };
                }
                ApiMessage::ModificationUsbSuccess(data) => {
                    match serde_json::from_str::<UsbResponse>(&data) {
                        Ok(UsbResponse::Success(SuccessData::Ack)) => {
                            let clone_sender = self.sender.clone();
                            let clone_email = self.enroll_email.clone();
                            let clone_user_role = self.enroll_role.clone();
                            let clone_auth_token = self.auth_token.clone();
                            let clone_url = self.config.api_url.clone();
                            tokio::spawn(async move {
                                let payload = ModifyPayload {
                                    email: clone_email,
                                    user_role: clone_user_role,
                                };
                                let client = reqwest::Client::new();
                                let url = format!("{}/users/modify", clone_url);
                                let resultat = client
                                    .post(&url)
                                    .json(&payload)
                                    .bearer_auth(clone_auth_token)
                                    .send()
                                    .await;
                                match resultat {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            let _ =
                                                clone_sender.send(ApiMessage::EnrollmentSuccess(
                                                    " Modifié (API OK) !".to_string(),
                                                ));
                                        } else {
                                            let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                                " Refus serveur (API KO)".to_string(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                            format!(" Erreur Réseau : {}", e),
                                        ));
                                    }
                                }
                            });
                        }
                        Ok(UsbResponse::Error(msg)) => {
                            self.enroll_status = format!(" Erreur Clé USB : {}", msg);
                        }
                        Ok(_) => {
                            self.enroll_status =
                                "Erreur Protocole : Données inattendues reçues".to_string();
                        }
                        Err(e) => {
                            self.enroll_status = format!("Erreur lecture JSON : {}", e);
                        }
                    };
                }
                ApiMessage::LoginError(texte) => {
                    self.login_status = texte.to_string();
                }
                ApiMessage::EnrollmentError(texte) => self.enroll_status = texte.to_string(),

                ApiMessage::ReceivedChallenge(le_challenge, session_id) => {
                    self.login_status =
                        "Challenge reçue, communication avec la bindkey en cours".to_string();
                    let clone_sender = self.sender.clone();
                    tokio::spawn(async move {
                        let mut port_name = String::new();
                        if let Ok(ports) = serialport::available_ports() {
                            for p in ports {
                                if let SerialPortType::UsbPort(_) = p.port_type {
                                    port_name = p.port_name;
                                    break;
                                }
                            }
                        }
                        if !port_name.is_empty() {
                            match send_command_bindkey(
                                &port_name,
                                share_protocol::Command::SignChallenge(le_challenge),
                            ) {
                                Ok(response) => {
                                    let _ = clone_sender
                                        .send(ApiMessage::SignedChallenge(response, session_id));
                                }
                                Err(message_erreur) => {
                                    let _ =
                                        clone_sender.send(ApiMessage::LoginError(message_erreur));
                                }
                            }
                        } else {
                            let _ = clone_sender
                                .send(ApiMessage::LoginError("Clé non détectée".to_string()));
                        }
                    });
                }
                ApiMessage::SignedChallenge(signature, session_id) => {
                    self.login_status =
                        "Signature générée. Vérification finale auprès du serveur".to_string();
                    let clone_session_id = session_id.clone();
                    let clone_signature = signature.clone();
                    let clone_sender = self.sender.clone();
                    let clone_url = self.config.api_url.clone();

                    tokio::spawn(async move {
                        let payload = VerifyPayload {
                            session_id: clone_session_id,
                            signature: clone_signature,
                        };
                        let client = reqwest::Client::new();
                        let resultat = client
                            .post(format!("{}/sessions/verify", clone_url))
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
                                                response.local_token,
                                                response.first_name,
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
                ApiMessage::LoginSuccess(role, token, first_name) => {
                    self.role_user = role;
                    self.auth_token = token;
                    self.first_name_user = first_name;

                    self.login_status = String::new();
                    self.login_password = String::new();

                    self.current_page = Page::Home;
                }
                ApiMessage::VolumeCreationSuccess(data) => {
                    match serde_json::from_str::<UsbResponse>(&data) {
                        Ok(UsbResponse::Success(SuccessData::VolumeCreated { encrypted_key })) => {
                            let clone_sender = self.sender.clone();
                            let clone_auth_token = self.auth_token.clone();
                            let clone_volume_name = self.volume_created_name.clone();
                            let clone_volume_size = self.volume_created_size.clone();
                            let clone_device_name = self.device_name.clone();
                            let clone_url = self.config.api_url.clone();
                            let clone_encrypted_key = encrypted_key;
                            tokio::spawn(async move {
                                let payload = VolumeCreatedInfo {
                                    device_name: clone_device_name,
                                    volume_name: clone_volume_name,
                                    volume_size_gb: clone_volume_size,
                                    encrypted_key: clone_encrypted_key,
                                };
                                let client = reqwest::Client::new();
                                let url = format!("{}/users/modify", clone_url);
                                let resultat = client
                                    .post(&url)
                                    .json(&payload)
                                    .bearer_auth(clone_auth_token)
                                    .send()
                                    .await;
                                match resultat {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            let _ = clone_sender.send(
                                                ApiMessage::VolumeCreationStatus(
                                                    "Volume enregistré sur le serv !".to_string(),
                                                ),
                                            );
                                        } else {
                                            let _ = clone_sender.send(
                                                ApiMessage::VolumeCreationStatus(
                                                    " Refus serveur (API KO)".to_string(),
                                                ),
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        let _ =
                                            clone_sender.send(ApiMessage::VolumeCreationStatus(
                                                format!(" Erreur Réseau : {}", e),
                                            ));
                                    }
                                }
                            });
                        }
                        Ok(UsbResponse::Error(msg)) => {
                            self.volume_status = format!(" Erreur Clé USB : {}", msg);
                        }
                        Ok(_) => {
                            self.volume_status =
                                "Erreur Protocole : Données inattendues reçues".to_string();
                        }
                        Err(e) => {
                            self.volume_status = format!("Erreur lecture JSON : {}", e);
                        }
                    };
                }
                ApiMessage::VolumeCreationStatus(texte) => {
                    self.volume_status = texte.to_string();
                }
                ApiMessage::VolumeInfoReceived(data) => {
                    match serde_json::from_str::<UsbResponse>(&data) {
                        Ok(UsbResponse::Success(SuccessData::VolumeInfo {
                            device_name,
                            device_size,
                            device_available_size,
                        })) => {
                            self.device_name = device_name;
                            self.device_size = device_size;
                            self.device_available_space = device_available_size;
                            self.volume_status = "Disque analysé avec succès.".to_string();
                        }
                        Ok(UsbResponse::Error(msg)) => {
                            self.volume_status = format!(" Erreur Clé USB : {}", msg);
                        }
                        Ok(_) => {
                            self.volume_status =
                                "Erreur Protocole : Données inattendues reçues".to_string();
                        }
                        Err(e) => {
                            self.volume_status = format!("Erreur lecture JSON : {}", e);
                        }
                    };
                }
            }
        }

        if self.current_page != Page::Login {
            egui::SidePanel::left("menu").show(ctx, |ui| {
                if ui.button("Accueil").clicked() {
                    self.current_page = Page::Home;
                };
                ui.add_space(10.0);
                if (self.role_user == Role::ENROLLEUR || self.role_user == Role::ADMIN)
                    && ui.button("Enrôlment").clicked()
                {
                    self.current_page = Page::Enrollment;
                };
                ui.add_space(10.0);
                if ui.button("Unlock").clicked() {
                    self.current_page = Page::Unlock;
                };
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                if ui.button("Déconnexion").clicked() {
                    self.current_page = Page::Login;
                    self.role_user = Role::NONE;
                    self.login_password.clear();
                    self.login_status.clear();
                    self.auth_token.clear();
                    self.first_name_user.clear();
                    self.enroll_firstname.clear();
                    self.enroll_lastname.clear();
                };
                ui.add_space(10.0);
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
