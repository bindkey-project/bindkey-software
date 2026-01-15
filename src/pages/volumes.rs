use crate::{BindKeyApp, VolumeInfo, protocol::{self, VolumeCreationPayload}, usb_service::send_command_bindkey};
use eframe::egui;
use sysinfo::{Disks};

pub fn show_volumes_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.heading("Volumes et chiffrement");
    ui.label("Branchez une clé USB vierge pour créer un volume sécurisé.");

    ui.horizontal(|ui| {
        if ui.button("Actualiser la liste des disques").clicked() {
            refresh_disks(app);
        }

        if ui.button("Créer un volume chiffré").clicked() {
            refresh_disks(app);
            app.creation_state.is_open = true;
            app.creation_state.status = String::new();
        }
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);

    if app.detected_volumes.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("Aucun volume détecté. Cliquez sur Actualiser.");
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for vol in &app.detected_volumes {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let icon = if vol.is_removable { "🔌" } else { "💾" };
                        ui.label(egui::RichText::new(icon).size(24.0));
                        
                        ui.vertical(|ui| {
                            ui.strong(&vol.name);
                            ui.monospace(&vol.mount_point);
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Libre : {} Go", bytes_to_gb(vol.available_space)));
                        });
                    });
                });
                ui.add_space(5.0);
            }
        });
    }
    if app.creation_state.is_open {
        // On ouvre une petite fenêtre au dessus du reste
        egui::Window::new("Assistant de Création de Volume")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.set_width(400.0);
                
                ui.heading("1. Choisissez le périphérique cible");
                ui.colored_label(egui::Color32::RED, "⚠️ ATTENTION : Le disque sera entièrement effacé !");
                
                if !app.detected_volumes.is_empty() {
                    let selected_idx = app.creation_state.selected_disk_index;
                    let safe_idx = if selected_idx < app.detected_volumes.len() { selected_idx } else { 0 };
                    
                    let selected_vol = &app.detected_volumes[safe_idx];
                    
                    egui::ComboBox::from_label("Disque cible")
                        .selected_text(format!("{} ({})", selected_vol.name, selected_vol.mount_point))
                        .show_ui(ui, |ui| {
                            for (i, vol) in app.detected_volumes.iter().enumerate() {
                                let label = format!("{} ({}) - {} Go", vol.name, vol.mount_point, bytes_to_gb(vol.total_space));
                                ui.selectable_value(&mut app.creation_state.selected_disk_index, i, label);
                            }
                        });
                } else {
                    ui.label("Aucun disque disponible.");
                }
                ui.add_space(10.0);

                ui.heading("2. Configuration");

                ui.label("Nom du volume :");
                ui.text_edit_singleline(&mut app.creation_state.volume_name);

                ui.add_space(5.0);

                ui.label(format!("Taille du volume : {} Go", app.creation_state.volume_size_gb));
                ui.add(egui::Slider::new(&mut app.creation_state.volume_size_gb, 1..=64).text("Go"));

                ui.horizontal(|ui| {
                    if ui.button("Annuler").clicked() {
                        app.creation_state.is_open = false;
                    }

                    let btn = egui::Button::new(" Formater et Chiffrer avec BindKey").fill(egui::Color32::DARK_RED);
                    if ui.add(btn).clicked() {
                        send_create_order(app, ui.ctx().clone());
                    }
                });

                if !app.creation_state.status.is_empty() {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::BLUE,&app.creation_state.status);
                }
            });
    }
}

fn send_create_order(app: &mut BindKeyApp, ctx: egui::Context) {
    let idx = app.creation_state.selected_disk_index;
    if idx >= app.detected_volumes.len() {return; }

    let target_disk_name = app.detected_volumes[idx].name.clone();
    let vol_name = app.creation_state.volume_name.clone();
    let vol_size = app.creation_state.volume_size_gb;
    if vol_name.is_empty() {
        app.creation_state.status = "Erreur : Nom vide".to_string();
        return;
    }

    app.creation_state.status = "Envoi de l'ordre à la BK".to_string();

    if let Some(device) = app.devices.first(){
        let port_name = device.port_name.clone();

        let payload = VolumeCreationPayload {
            target_device_name: target_disk_name,
            volume_name: vol_name,
            size_gb: vol_size,
        };
        let cmd = protocol::Command::CreateVolume(payload);
        tokio::spawn(async move {
            match send_command_bindkey(&port_name, cmd) {
                Ok(response_str) => {
                    if response_str.contains("VolumeCreated") {
                        println!("Volume créé par la BindKey !");
                    }
                }
                Err(e) => {
                    println!("Erreur création : {}", e);
                }
            }
            ctx.request_repaint();
        });
    }
    else {
        app.creation_state.status = "❌ Erreur : BindKey non connectée !".to_string();
    }
}

fn refresh_disks(app: &mut BindKeyApp) {
    app.detected_volumes.clear();
    
    let disks = Disks::new_with_refreshed_list();
    
    for disk in &disks {
        if disk.is_removable() { 
            app.detected_volumes.push(VolumeInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                is_removable: true,
            });
        } 
    }
}

fn bytes_to_gb(bytes: u64) -> u64 {
    bytes / 1024 / 1024 / 1024
}
