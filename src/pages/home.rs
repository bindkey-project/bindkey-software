use crate::{
    BindKeyApp,
    protocol::protocol::{ApiMessage, Page, VolumeTab},
    protocol::updater::{UPDATE_PUBLIC_KEY, update_application}
};
use eframe::egui;

pub fn show_home_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    let card_frame = egui::Frame::none()
        .fill(ui.visuals().window_fill()) // <-- Correction ici : () ajoutées
        .rounding(15.0)
        .stroke(ui.visuals().window_stroke())
        .inner_margin(30.0);

    card_frame.show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("BindKey Security").size(32.0).strong());
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Votre coffre-fort numérique personnel")
                    .color(egui::Color32::GRAY),
            );
            ui.add(
                egui::Image::new(egui::include_image!("../../bindkey.png"))
                    .max_width(500.0)
                    .rounding(10.0),
            );
        });
    });

    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("Tableau de Bord").size(28.0).strong());
    });
    ui.add_space(30.0);

    // On crée 2 colonnes de même largeur pour mettre les cartes côte à côte
    ui.columns(2, |cols| {
        // ==========================================
        // COLONNE GAUCHE (Statuts)
        // ==========================================
        // LA CORRECTION RUST EST ICI : On utilise cols[0].visuals() au lieu de ui.visuals()
        let frame_style = egui::Frame::none()
            .fill(cols[0].visuals().window_fill())
            .rounding(10.0)
            .stroke(cols[0].visuals().window_stroke())
            .inner_margin(15.0);

        frame_style.show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.heading("🟢 Statut de la clé");
            ui.separator();
            if app.usb_connected {
                ui.label("BindKey détectée et prête.");
                // Ajouter ici une ProgressBar d'espace libre !
            } else {
                ui.colored_label(egui::Color32::RED, "Veuillez brancher votre BindKey.");
            }
        });

        cols[0].add_space(15.0); // Espace vertical entre deux cartes de la même colonne

        frame_style.show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.heading("Sécurité");
            ui.separator();
            ui.label("Volumes chiffrés actifs : 2");
            ui.label("Firmware : v1.0.4");
        });

        // ==========================================
        // COLONNE DROITE (Actions Rapides)
        // ==========================================
        frame_style.show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            ui.heading("Actions Rapides");
            ui.separator();

            ui.add_space(10.0);
            // Un gros bouton qui prend toute la largeur
            let btn_create = egui::Button::new("Créer un Volume")
                .min_size(egui::vec2(ui.available_width(), 40.0));
            if ui.add(btn_create).clicked() {
                app.current_page = Page::Volume;
                app.active_tab = VolumeTab::Gestion; // Ça téléporte l'utilisateur !
            }

            ui.add_space(10.0);
            let btn_enroll = egui::Button::new("Enrôler une nouvelle clé")
                .min_size(egui::vec2(ui.available_width(), 40.0));
            if ui.add(btn_enroll).clicked() {
                app.current_page = Page::Enrollment;
            }
        });
    });
    // =========================================================
    // NOUVELLE CARTE : SYSTÈME ET MISES À JOUR
    // =========================================================
    let card_frame = egui::Frame::none()
        .fill(ui.visuals().window_fill())
        .rounding(12.0)
        .inner_margin(20.0)
        .shadow(eframe::egui::epaint::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 10.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(40),
        });

    card_frame.show(ui, |ui| {
        ui.heading("⚙️ Système & Mises à jour");
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Version actuelle :").strong());
            ui.label(env!("CARGO_PKG_VERSION")); 
        });

        ui.add_space(15.0);

        // CORRECTION 1 : La bonne syntaxe pour dimensionner un bouton
        if ui.add(egui::Button::new("🔄 Rechercher une mise à jour").min_size(egui::vec2(200.0, 30.0))).clicked() {
            let sender_clone = app.sender.clone();
            
            std::thread::spawn(move || {
                let _ = sender_clone.send(ApiMessage::UpdateStatus("Recherche et vérification en cours...".to_string()));
                
                // 1. On aspire la VRAIE clé publique Zipsign directement depuis le fichier !
                // (Vérifie que le chemin pointe bien vers le update.pub de Zipsign)
                let pub_key_bytes = include_bytes!("../../update.pub"); 

                // 2. On la met dans le moule exact de 32 octets attendu par le compilateur
                let mut raw_key = [0u8; 32];
                // Par sécurité, on prend exactement les 32 premiers octets (au cas où il y aurait un saut de ligne)
                raw_key.copy_from_slice(&pub_key_bytes[..32]);

                // 3. On lance le moteur de mise à jour !
                let update_result = self_update::backends::github::Update::configure()
                    .repo_owner("bindkey-project")
                    .repo_name("bindkey-software")
                    .bin_name("bindkey-client")
                    .show_download_progress(true)
                    .current_version(env!("CARGO_PKG_VERSION"))
                    .verifying_keys([raw_key]) // <- Syntaxe parfaite et clé parfaite !
                    .build();

                match update_result {
                    Ok(updater) => {
                        match updater.update() {
                            Ok(status) => {
                                if status.updated() {
                                    let _ = sender_clone.send(ApiMessage::UpdateStatus(format!("✅ Succès ! Mise à jour installée (v{}). Veuillez relancer l'application.", status.version())));
                                } else {
                                    let _ = sender_clone.send(ApiMessage::UpdateStatus("L'application est déjà à jour !".to_string()));
                                }
                            },
                            Err(e) => {
                                let _ = sender_clone.send(ApiMessage::UpdateStatus(format!("❌ Échec du téléchargement : {}", e)));
                            }
                        }
                    },
                    Err(e) => {
                        let _ = sender_clone.send(ApiMessage::UpdateStatus(format!("❌ Erreur de configuration : {}", e)));
                    }
                }
            });
        }

        if !app.update_status.is_empty() {
            ui.add_space(10.0);
            let color = if app.update_status.contains("❌") { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, &app.update_status);
        }
    });
}

/*
ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "Bienvenue sur l'App BindKey {}",
                app.first_name_user
            ));

            ui.add_space(10.0);

            ui.label(format!("Votre rôle est : {:?}", app.role_user));

            ui.add_space(30.0);

            ui.add(
                egui::Image::new(egui::include_image!("../../bindkey.png"))
                    .max_width(500.0)
                    .rounding(10.0),
            );
        });
    });
*/
