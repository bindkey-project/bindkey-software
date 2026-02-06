use eframe::egui;
use serialport::SerialPortType;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
mod config;
mod pages;
mod protocol;
use crate::protocol::protocol::{ApiMessage, LogOut, Page, Role, User, create_secure_client};
mod usb_service;
use crate::config::AppConfig;
use validator::Validate;
mod event_handler;

#[derive(Validate)]
struct BindKeyApp {
    pub is_loading: bool,
    pub current_page: Page,
    pub first_name_user: String,
    pub role_user: Role,
    pub enroll_firstname: String,
    pub enroll_lastname: String,
    #[validate(email)]
    pub enroll_email: String,
    #[validate(length(min = 14))]
    pub enroll_password: String,
    pub enroll_role: Role,
    pub device_name: String,
    pub device_size: f64,
    pub device_available_space: f64,
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
    pub is_admin_mode: bool,
    pub server_token: String,
    pub local_token: String,
    pub config: AppConfig,
    pub usb_connected: bool,
    pub last_usb_check: Instant,
    pub users_list: Vec<User>,
    pub current_port_name: String,
    pub api_client: reqwest::Client,
}

impl BindKeyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();

        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(32.0, egui::FontFamily::Proportional),
        );

        cc.egui_ctx.set_style(style);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(32, 33, 36);

        visuals.extreme_bg_color = egui::Color32::from_rgb(10, 10, 15);

        cc.egui_ctx.set_visuals(visuals);

        let client = match create_secure_client() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("ERREUR FATALE CLIENT HTTP : {}", e);
                reqwest::Client::new()
            }
        };

        let (tx, rx) = channel();
        let config = AppConfig::load();
        BindKeyApp {
            is_loading: false,
            current_page: Page::Login,
            first_name_user: String::new(),
            role_user: Role::NONE,
            enroll_firstname: String::new(),
            enroll_lastname: String::new(),
            enroll_email: String::new(),
            enroll_password: String::new(),
            enroll_role: Role::NONE,
            device_name: String::new(),
            device_size: 0.0,
            device_available_space: 0.0,
            volume_created_name: String::new(),
            volume_created_size: 1,
            receiver: rx,
            sender: tx,
            login_status: String::new(),
            enroll_status: String::new(),
            volume_status: String::new(),
            login_email: String::new(),
            login_password: String::new(),
            is_admin_mode: false,
            server_token: String::new(),
            local_token: String::new(),
            config,
            usb_connected: false,
            last_usb_check: Instant::now(),
            users_list: Vec::new(),
            current_port_name: String::new(),
            api_client: client,
        }
    }
}

impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        if self.last_usb_check.elapsed() > Duration::from_secs(1) {
            self.last_usb_check = Instant::now();
        }

        let mut found_port = String::new();

        if let Ok(ports) = serialport::available_ports() {
            for p in ports {
                match p.port_type {
                    SerialPortType::UsbPort(info) => {
                        if info.vid == 0x10c4 && info.pid == 0xea60 {
                            found_port = p.port_name;
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        self.usb_connected = true;
        self.current_port_name = found_port;

        ctx.request_repaint_after(Duration::from_secs(1));

        if let Ok(message) = self.receiver.try_recv() {
            event_handler::handke_api_message(self, message);
        }

        if self.current_page != Page::Login {
            egui::SidePanel::left("menu").show(ctx, |ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if self.usb_connected {
                        ui.colored_label(egui::Color32::GREEN, "●");
                        ui.label("Bindkey Connectée");
                    } else {
                        ui.colored_label(egui::Color32::RED, "●");
                        ui.label("Aucune clé détectée");
                    }
                });
                ui.add_space(20.0);

                if ui.button("Accueil").clicked() {
                    self.current_page = Page::Home;
                };
                ui.add_space(10.0);
                if (self.role_user == Role::ENROLLER || self.role_user == Role::ADMIN)
                    && ui.button("Enrôlment").clicked()
                {
                    self.current_page = Page::Enrollment;
                };
                ui.add_space(10.0);
                if ui.button("Volume").clicked() {
                    self.current_page = Page::Volume;
                };
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                if ui.button("Déconnexion").clicked() {
                    let url = self.config.api_url.clone();
                    let clone_auth_token = self.server_token.clone();
                    let clone_sender = self.sender.clone();
                    let clone_api_client = self.api_client.clone();

                    tokio::spawn(async move {
                        let payload = LogOut {
                            server_token: clone_auth_token.clone(),
                        };

                        let result = clone_api_client
                            .post(format!("{}/sessions/logout", url))
                            .json(&payload)
                            .bearer_auth(clone_auth_token)
                            .send()
                            .await;
                        match result {
                            Ok(response) => {
                                if response.status().is_success() {
                                    let _ = clone_sender.send(ApiMessage::LogOutSuccess);
                                } else {
                                    let _ = clone_sender.send(ApiMessage::LogOutError(format!(
                                        "Erreur lors de la déconnexion: {}",
                                        response.status()
                                    )));
                                }
                            }
                            Err(e) => {
                                let _ = clone_sender.send(ApiMessage::LogOutError(format!(
                                    "Erreur lors de la communication avec le serveur: {}",
                                    e
                                )));
                            }
                        }
                    });
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
            Page::Volume => {
                pages::volumes::show_volumes_page(self, ui);
            }
        });
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Client BindKey",
        native_options,
        Box::new(|cc| Ok(Box::new(BindKeyApp::new(cc)))),
    )
}
