use eframe::egui;
use serialport::SerialPortType;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
mod config;
mod pages;
mod protocol;
use crate::protocol::protocol::{
    ApiMessage, LogOut, Page, Role, UsbDevice, User, VolumeTab, create_secure_client,
};
mod usb_service;
use crate::config::AppConfig;
use validator::Validate;
mod event_handler;
pub const UPDATE_PUBLIC_KEY: &str = "RWSJeF+oi2P6KH0F+FjnPr3NuWxaRv2DNisbPUBQpq2E6oB87JFQAqcX";

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
    pub formatage_status: String,
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
    pub available_devices: Vec<UsbDevice>,
    pub active_tab: VolumeTab,
    pub update_status: String,
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
            formatage_status: String::new(),
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
            available_devices: Vec::new(),
            active_tab: VolumeTab::Gestion,
            update_status: String::new(),
        }
    }
}

impl eframe::App for BindKeyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        configurer_theme_bindkey(ctx);

        if self.last_usb_check.elapsed() > Duration::from_secs(1) {
            self.last_usb_check = Instant::now();

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
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        if let Ok(message) = self.receiver.try_recv() {
            event_handler::handle_api_message(self, message);
        }

        if self.current_page != Page::Login {
            egui::SidePanel::left("menu").show(ctx, |ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if self.usb_connected {
                        ui.colored_label(egui::Color32::GREEN, "BindKey Connectée");
                        ui.label("BindKey Connectée");
                    } else {
                        ui.colored_label(egui::Color32::RED, "BindKey Déconnectée");
                        ui.label("BindKey Déconnectée");
                    }
                });
                ui.add_space(20.0);

                if ui.button("Accueil").clicked() {
                    self.current_page = Page::Home;
                };
                ui.add_space(10.0);
                if (self.role_user == Role::ENROLLER || self.role_user == Role::ADMIN)
                    && ui.button("Enrôlement").clicked()
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

pub fn configurer_theme_bindkey(ctx: &eframe::egui::Context) {
    let mut visuals = eframe::egui::Visuals::dark();

    visuals.window_fill = eframe::egui::Color32::from_rgb(42, 47, 56);
    visuals.panel_fill = eframe::egui::Color32::from_rgb(32, 36, 44);
    visuals.extreme_bg_color = eframe::egui::Color32::from_rgb(24, 28, 36);

    visuals.override_text_color = Some(eframe::egui::Color32::from_rgb(210, 215, 222));

    let accent_color = eframe::egui::Color32::from_rgb(0, 150, 255);
    visuals.selection.bg_fill = accent_color;
    visuals.hyperlink_color = accent_color;

    let rounding = eframe::egui::Rounding::same(8.0);
    visuals.window_rounding = eframe::egui::Rounding::same(12.0);

    visuals.widgets.noninteractive.rounding = rounding;
    visuals.widgets.inactive.rounding = rounding;
    visuals.widgets.hovered.rounding = rounding;
    visuals.widgets.active.rounding = rounding;

    visuals.widgets.noninteractive.bg_stroke =
        eframe::egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 60, 70));
    visuals.widgets.inactive.bg_stroke =
        eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(65, 70, 80));

    ctx.set_visuals(visuals);
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
#[test]
fn extraction_ultime_zipsign() {
    let pub_key_bytes = include_bytes!("../update.pub");
    let content = String::from_utf8_lossy(pub_key_bytes);
    
    // 1. On sépare tout le texte par les espaces, et on garde le plus long morceau.
    // Le Base64 sera obligatoirement le mot le plus long du fichier !
    let mut mot_le_plus_long = "";
    for mot in content.split_whitespace() {
        if mot.len() > mot_le_plus_long.len() {
            mot_le_plus_long = mot;
        }
    }
    
    // 2. Par sécurité, on enlève un éventuel point-virgule qui serait collé à la clé
    let b64_propre = mot_le_plus_long.trim_start_matches(';');
    
    // 3. On décode !
    use base64::{Engine as _, engine::general_purpose};
    let decoded = general_purpose::STANDARD.decode(b64_propre)
        .unwrap_or_else(|_| general_purpose::STANDARD_NO_PAD.decode(b64_propre).expect("C'est toujours pas du Base64 valide !"));
        
    // 4. On récupère les 32 derniers octets
    let vraie_cle = if decoded.len() >= 32 {
        &decoded[decoded.len() - 32..]
    } else {
        panic!("La clé est trop courte ({} octets) !", decoded.len());
    };
    
    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(vraie_cle);
    
    // 5. La libération
    println!("\n==================================================");
    println!("✅ VICTOIRE TOTALE ! Voici ton tableau à copier-coller :");
    println!("{:?}", raw_key);
    println!("==================================================\n");
}