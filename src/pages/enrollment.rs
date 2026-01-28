use crate::usb_service::send_command_bindkey;
use crate::{ApiMessage, BindKeyApp, Role, share_protocol};
use eframe::egui;
use serialport::SerialPortType;
use sha2::{Digest, Sha256};
use validator::{self, ValidateEmail, ValidateLength};

pub fn show_enrollment_page(app: &mut BindKeyApp, ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.label("Firstname :");
        ui.text_edit_singleline(&mut app.enroll_firstname);
        ui.add_space(20.0);

        ui.label("Lastname :");
        ui.text_edit_singleline(&mut app.enroll_lastname);
        ui.add_space(20.0);

        ui.label("Email :");
        ui.text_edit_singleline(&mut app.enroll_email);
        ui.add_space(20.0);

        ui.label("Password :");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.enroll_password).password(true));
            ui.label("Le mdp doit faire au minimum 14 caractères.");
        });

        ui.add_space(20.0);

        egui::ComboBox::from_label("Role")
            .selected_text(format!("{:?}", app.enroll_role))
            .show_ui(ui, |ui| {
                if app.role_user == Role::ADMIN {
                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                    ui.selectable_value(&mut app.enroll_role, Role::ENROLLEUR, "ENROLLEUR");
                } else if app.role_user == Role::ENROLLEUR {
                    ui.selectable_value(&mut app.enroll_role, Role::USER, "USER");
                }
            });
    });
    let formulaire_valide = !app.enroll_firstname.is_empty()
        && !app.enroll_lastname.is_empty()
        && !app.enroll_email.is_empty()
        && app.enroll_email.validate_email()
        && !app.enroll_password.is_empty()
        && app.enroll_password.validate_length(Some(14), None, None)
        && app.enroll_role != Role::NONE;

    ui.add_space(20.0);

    ui.add_enabled_ui(formulaire_valide, |ui| {
        if ui.button("Validé").clicked() {
            let bypass_usb = true;
          app.enroll_status = if bypass_usb {
        "🛠️ MODE SIMULATION : Bypass USB activé...".to_string()
    } else {
        "🔌 Recherche de la clé USB...".to_string()
    };

    let clone_sender = app.sender.clone();

    tokio::spawn(async move {
       let resultat_usb: Result<String, String>;

        if bypass_usb {
        println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
        resultat_usb = Ok(r#"{"status": "SUCCESS", "uid": "SIMULATED-BK-999", "public_key": "simulated-key-xyz"}"#
        .to_string());
    } else {
        let mut port_name = String::new();
        if let Ok(ports) = serialport::available_ports() {
            for p in ports {
                if let SerialPortType::UsbPort(_) = p.port_type {
                    port_name = p.port_name;
                    break;
                };
            }
        }

        if !port_name.is_empty() {
            resultat_usb = send_command_bindkey(&port_name, share_protocol::Command::StartEnrollment);
        } else {
            resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
        }
    }

    match resultat_usb {
        Ok(data) => {
            let _ = clone_sender.send(ApiMessage::EnrollmentUsbSuccess(data));
        }
        Err(e) => {
            let _ = clone_sender.send(ApiMessage::EnrollmentError(format!("Erreur USB: {}", e)));
        }
    }
    });
        };
    });

    let modif_valid = !app.enroll_email.is_empty()
        && app.enroll_role != Role::NONE
        && app.enroll_email.validate_email()
        && app.enroll_firstname.is_empty()
        && app.enroll_lastname.is_empty()
        && app.enroll_password.is_empty();

    ui.add_enabled_ui(modif_valid, |ui| {
        if ui.button("Modifié").clicked() {
             let bypass_usb = true;
            app.enroll_status = if bypass_usb {
        "🛠️ MODE SIMULATION : Bypass USB activé...".to_string()
    } else {
        "🔌 Recherche de la clé USB...".to_string()
    };

    let clone_sender = app.sender.clone();

    tokio::spawn(async move {
       let resultat_usb: Result<String, String>;

        if bypass_usb {
        println!(">> SIMULATION : On fait comme si la clé avait dit OUI");
        resultat_usb = Ok(r#"{"status": "SUCCESS", "uid": "SIMULATED-BK-999", "public_key": "simulated-key-xyz"}"#
        .to_string());
    } else {
        let mut port_name = String::new();
        if let Ok(ports) = serialport::available_ports() {
            for p in ports {
                if let SerialPortType::UsbPort(_) = p.port_type {
                    port_name = p.port_name;
                    break;
                };
            }
        }

        if !port_name.is_empty() {
            resultat_usb = send_command_bindkey(&port_name, share_protocol::Command::Modify);
        } else {
            resultat_usb = Err("Aucune Bindkey détectée. Branchez-là !".to_string());
        }
    }

    match resultat_usb {
        Ok(data) => {
            let _ = clone_sender.send(ApiMessage::ModificationUsbSuccess(data));
        }
        Err(e) => {
            let _ = clone_sender.send(ApiMessage::EnrollmentError(format!("Erreur USB: {}", e)));
        }
    }
    });
        };

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        if !app.enroll_status.is_empty() {
            ui.colored_label(egui::Color32::BLUE, &app.enroll_status);
        }
    });
});
}

pub fn hash_password_with_salt(password: &str) -> String {
    let salt = "bindkey.com";
    let combined = format!("{}{}", password, salt);
    let mut hasher = Sha256::new();
    hasher.update(combined);
    format!("{:x}", hasher.finalize())
}
