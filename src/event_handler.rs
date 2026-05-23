use crate::BindKeyApp;
use crate::protocol::protocol::{
    ApiMessage, BindKeyInfo, Page, Role, StatusBindkey, User, UserWithBindKey,
    VolumeCreatedInfo, VolumeInfo, VolumeTab, RegisterPayload, LoginSuccessResponse,
};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use crate::usb_service::send_text_command;
use crate::pages::enrollment::hash_password_with_salt;
use eframe::egui;
use std::process::Command;
use std::time::Duration;
use serde_json::json;

pub fn handle_api_message(app: &mut BindKeyApp, message: ApiMessage) {
    match message {
        ApiMessage::LoginSuccess(role, first_name, server_token, local_token, sn) => {
            app.role_user = role;
            app.first_name_user = first_name;
            app.server_token = server_token;
            app.local_token = local_token;
            app.current_page = Page::Home;
            app.local_bindkey_sn = Some(sn);
            app.login_status = "Connexion réussie !".to_string();
            app.is_loading = false;
        }
        ApiMessage::LoginError(err) => {
            app.login_status = format!("❌ {}", err);
            app.is_loading = false;
        }
        ApiMessage::EnrollmentSuccess(email) => {
            app.enroll_status = format!("Utilisateur {} enrôlé avec succès !", email);
            app.is_loading = false;
        }
        ApiMessage::EnrollmentError(err) => {
            app.enroll_status = format!("❌ {}", err);
            app.is_loading = false;
        }
        ApiMessage::EnrollmentUsbSuccess(data) => {
            if let UsbResponse::Success(SuccessData::EnrollmentInfo { sn, pub_sign, pub_ecdh }) = data {
                app.enroll_status = "Données matérielles récupérées, envoi au serveur...".to_string();
                let clone_sender = app.sender.clone();
                let clone_firstname = app.enroll_firstname.clone();
                let clone_lastname = app.enroll_lastname.clone();
                let clone_email = app.enroll_email.clone();
                let hash_pw = hash_password_with_salt(&app.enroll_password);
                let clone_role = app.enroll_role.clone();
                let clone_token = app.server_token.clone();
                let clone_url = app.config.api_url.clone();
                let clone_client = app.api_client.clone();

                tokio::spawn(async move {
                    let payload = RegisterPayload {
                        first_name: clone_firstname,
                        last_name: clone_lastname,
                        email: clone_email,
                        password: hash_pw,
                        user_role: clone_role,
                        bindkey_status: StatusBindkey::ACTIVE,
                        pub_sign,
                        sn,
                        pub_ecdh,
                    };
                    let url = format!("{}/auth/register", clone_url);
                    let res = clone_client.post(&url).json(&payload).bearer_auth(clone_token).send().await;
                    match res {
                        Ok(resp) if resp.status().is_success() => {
                            let _ = clone_sender.send(ApiMessage::EnrollmentSuccess(payload.email));
                        }
                        _ => {
                            let _ = clone_sender.send(ApiMessage::EnrollmentError("Refus serveur".into()));
                        }
                    }
                });
                app.enroll_password.clear();
            }
        }
        ApiMessage::ModificationUsbSuccess(_) => {
            app.enroll_status = "BindKey mise à jour avec succès !".to_string();
            app.is_loading = false;
        }
        ApiMessage::ReceivedChallenge(challenge, session_id, sn) => {
            app.login_status = "Scannez votre doigt sur la BindKey...".to_string();
            let clone_sender = app.sender.clone();
            let clone_port = app.current_port_name.clone();
            let clone_challenge = challenge;
            let clone_session_id = session_id;
            let clone_sn = sn;

            tokio::spawn(async move {
                if !clone_port.is_empty() {
                    if let Ok(mut port) = serialport::new(&clone_port, 115200).timeout(Duration::from_secs(15)).open() {
                        let _ = port.write_data_terminal_ready(true);
                        let cmd = format!("challenge={}", clone_challenge);
                        match send_text_command(&mut *port, &cmd) {
                            Ok(map) => {
                                if let Some(sig) = map.get("SIG") {
                                    let _ = clone_sender.send(ApiMessage::SignedChallenge(sig.clone(), clone_session_id, clone_sn));
                                } else {
                                    let _ = clone_sender.send(ApiMessage::LoginError("Signature absente".into()));
                                }
                            }
                            Err(e) => { let _ = clone_sender.send(ApiMessage::LoginError(format!("Erreur USB: {}", e))); }
                        }
                    }
                }
            });
        }
        ApiMessage::SignedChallenge(signature, session_id, sn) => {
            app.login_status = "Vérification de la signature...".to_string();
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_client = app.api_client.clone();
            let clone_sn = sn;

            tokio::spawn(async move {
                let payload = json!({ "session_id": session_id, "signature": signature });
                let url = format!("{}/sessions/verify", clone_url);
                if let Ok(resp) = clone_client.post(&url).json(&payload).send().await {
                    if resp.status().is_success() {
                        if let Ok(data) = resp.json::<LoginSuccessResponse>().await {
                            let _ = clone_sender.send(ApiMessage::LoginSuccess(data.role, data.first_name, data.server_token, data.local_token, clone_sn));
                            return;
                        }
                    }
                }
                let _ = clone_sender.send(ApiMessage::LoginError("Authentification échouée".into()));
            });
        }
        ApiMessage::VolumeCreationSuccess(data) => {
            if let UsbResponse::Success(SuccessData::VolumeCreated { volume_id, .. }) = data {
                let clone_sender = app.sender.clone();
                let clone_auth_token = app.server_token.clone();
                let clone_volume_name = app.volume_created_name.trim().to_uppercase();
                let clone_volume_size = app.volume_created_size;
                let clone_url = app.config.api_url.clone();
                let clone_api_client = app.api_client.clone();

                tokio::spawn(async move {
                    let payload = VolumeCreatedInfo {
                        name: clone_volume_name,
                        size_bytes: clone_volume_size,
                        id: volume_id.clone(),
                    };
                    let url = format!("{}/volumes", clone_url);
                    let _ = clone_api_client.post(&url).json(&payload).bearer_auth(clone_auth_token).send().await;
                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus("Volume enregistré !".to_string()));
                    let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
                });
            }
        }
        ApiMessage::VolumeCreationStatus(texte) => { app.volume_status = texte.to_string(); }
        ApiMessage::VolumeDashboardStatus(texte) => {
            app.dashboard_status = texte.to_string();
            if texte.contains("succès") || texte.contains("Erreur") || texte.contains("❌") { app.is_loading = false; }
        }
        ApiMessage::VolumeInfoReceived(data) => {
            if let UsbResponse::Success(SuccessData::DeviceInfo { device_name, device_size, device_available_size }) = data {
                app.device_name = device_name;
                app.device_size = device_size;
                app.device_available_space = device_available_size;
                app.volume_status = "Disque analysé avec succès.".to_string();
            }
        }
        ApiMessage::FetchUsers => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            tokio::spawn(async move {
                if let Ok(resp) = clone_api_client.get(format!("{}/users", url)).bearer_auth(clone_auth_token).send().await {
                    if let Ok(users) = resp.json::<Vec<User>>().await {
                        let _ = clone_sender.send(ApiMessage::UserFetched(users));
                    }
                }
            });
        }
        ApiMessage::UserFetched(users) => app.users_list = users,
        ApiMessage::DeleteUser(id) => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            tokio::spawn(async move {
                let _ = clone_api_client.delete(format!("{}/users/{}", url, id)).bearer_auth(clone_auth_token).send().await;
                let _ = clone_sender.send(ApiMessage::UserDeleted);
            });
        }
        ApiMessage::UserDeleted => { let _ = app.sender.send(ApiMessage::FetchUsers); }
        ApiMessage::SearchUserByEmail(email) => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            tokio::spawn(async move {
                match clone_api_client.get(format!("{}/admin/users/search?email={}", url, email)).bearer_auth(clone_auth_token).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if status.is_success() {
                            if let Ok(user) = resp.json::<UserWithBindKey>().await {
                                let _ = clone_sender.send(ApiMessage::UserFound(user));
                            } else {
                                let _ = clone_sender.send(ApiMessage::SearchUserError("Format de réponse invalide".to_string()));
                            }
                        } else if status == 404 {
                            let _ = clone_sender.send(ApiMessage::SearchUserError("Utilisateur introuvable".to_string()));
                        } else if status == 401 {
                            let _ = clone_sender.send(ApiMessage::SearchUserError("Non autorisé (401) : Votre session a expiré. Veuillez vous déconnecter puis vous reconnecter.".to_string()));
                        } else {
                            let error_text = resp.text().await.unwrap_or_else(|_| "Détails indisponibles".to_string());
                            let _ = clone_sender.send(ApiMessage::SearchUserError(format!("Erreur serveur ({}): {}", status, error_text)));
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender.send(ApiMessage::SearchUserError(format!("Erreur réseau: {}", e)));
                    }
                }
            });
        }
        ApiMessage::UserFound(user) => { app.search_result = Some(user); app.is_loading = false; }
        ApiMessage::UpdateBindKeyStatus(sn, status) => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            tokio::spawn(async move {
                let _ = clone_api_client.put(format!("{}/bindkeys/status", url)).json(&serde_json::json!({"serial_number": sn, "status": status})).bearer_auth(clone_auth_token).send().await;
                let _ = clone_sender.send(ApiMessage::BindKeyStatusUpdated);
            });
        }
        ApiMessage::BindKeyStatusUpdated => app.enroll_status = "Statut mis à jour !".to_string(),
        ApiMessage::StartFormatBindKey { device_path, partitions, port_name, .. } => {
            let clone_sender = app.sender.clone();
            app.formatage_status = "Initialisation du formatage...".to_string();
            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            tokio::spawn(async move {
                if let Ok(mut port) = serialport::new(&port_name, 115200).timeout(Duration::from_secs(10)).open() {
                    let _ = port.write_data_terminal_ready(true);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if let Ok(map) = send_text_command(&mut *port, "action=init_format\n") {
                        if let Some(ids_str) = map.get("TO_DEL") {
                            for id in ids_str.split(';') {
                                if !id.trim().is_empty() {
                                    let _ = clone_api_client.delete(format!("{}/volumes/delete_id/{}", clone_url, id.trim())).bearer_auth(&clone_token).send().await;
                                }
                            }
                        }
                        if map.get("STATUS").map(|v| v.contains("OK")).unwrap_or(false) {
                            let _ = tokio::task::spawn_blocking(move || crate::pages::volumes::force_format(&device_path, &partitions)).await;
                            let _ = clone_sender.send(ApiMessage::FormatStatus("Succès : Clé réinitialisée.".to_string()));
                            let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
                        }
                    }
                }
            });
        }
        ApiMessage::RequestVolumeRefresh => {
            app.needs_volume_refresh = false;
            let clone_sender = app.sender.clone();
            tokio::spawn(async move {
                if let Ok(output) = Command::new("lsblk").args(&["-J", "-b", "-o", "NAME,MODEL,SIZE,TRAN,FSTYPE,PTTYPE,MOUNTPOINT,LABEL,FSUSED"]).output() {
                    let ouput_str = String::from_utf8_lossy(&output.stdout);
                    if let Ok(parsed) = serde_json::from_str::<crate::protocol::protocol::LsblkOutput>(&ouput_str) {
                        let mut extracted_volumes = Vec::new();
                        for disk in parsed.blockdevices {
                            if disk.tran.as_deref() == Some("usb") {
                                if let Some(children) = disk.children {
                                    for part in children {
                                        if part.size < 10_000_000 { continue; }
                                        if let Some(label) = part.label {
                                            extracted_volumes.push(VolumeInfo {
                                                name: label,
                                                device_path: format!("/dev/{}", part.name),
                                                total_space_gb: ((part.size as f64 / 1_073_741_824.0) * 10.0).round() / 10.0,
                                                used_space_gb: part.fsused.as_ref().and_then(|v| v.as_u64().map(|b| ((b as f64 / 1_073_741_824.0) * 10.0).round() / 10.0)),
                                                is_mounted: part.mountpoint.is_some(),
                                                mount_point: part.mountpoint,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        let _ = clone_sender.send(ApiMessage::VolumesUpdated(extracted_volumes));
                    }
                }
            });
        }
        ApiMessage::VolumesUpdated(volumes) => app.dashboard_volumes = volumes,
        ApiMessage::LogOutSuccess => {
            // 1. Redirection et Rôle
            app.current_page = Page::Login; 
            app.role_user = Role::NONE;

            // 2. Jetons et Sécurité
            app.server_token.clear(); 
            app.local_token.clear();
            app.login_password.clear();
            // Optionnel: vider l'email de login si tu veux forcer la resaisie, 
            // sinon on le laisse pour l'UX. On le garde par défaut pour le confort.
            
            // 3. Infos Utilisateur
            app.first_name_user.clear();
            app.local_bindkey_sn = None;

            // 4. Inputs Enrôlement
            app.enroll_firstname.clear();
            app.enroll_lastname.clear();
            app.enroll_email.clear();
            app.enroll_password.clear();

            // 5. Inputs Gestion Volume
            app.volume_created_name.clear();
            app.volume_created_size = 0;
            app.device_name.clear();
            app.device_size = 0.0;
            app.device_available_space = 0.0;

            // 6. Inputs & État Partage
            app.share_input_email.clear();
            app.search_email_input.clear();
            app.search_result = None;
            app.share_target_name = None;
            app.share_target_email = None;
            app.share_target_role = None;
            app.sharing_active_volume = None;
            app.show_volume_selection = false;
            app.is_sharing_in_progress = false;
            app.is_searching_user = false;

            // 7. Listes et Données
            app.dashboard_volumes.clear();
            app.users_list.clear();
            app.active_volume_id_hex = None;

            // 8. Messages de Statut API (Clear All)
            app.enroll_status.clear();
            app.volume_status.clear();
            app.formatage_status.clear();
            app.dashboard_status.clear();
            app.update_status.clear();
            app.share_search_feedback.clear();
            app.share_pipeline_status.clear();
            
            // 9. Feedback Final
            app.is_loading = false;
            app.login_status = "Déconnexion réussie.".to_string();
        }
        ApiMessage::StartVolumeDeletion(name, path) => {
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            let clone_name = name.clone();
            let clone_path = path.clone();

            tokio::spawn(async move {
                let url = format!("{}/volumes/find_id?name={}", clone_url, clone_name);
                if let Ok(resp) = clone_api_client.get(&url).bearer_auth(&clone_token).send().await {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(id) = data.get("volume_id").and_then(|v| v.as_str()) {
                            let _ = clone_sender.send(ApiMessage::VolumeIdReceivedForDeletion(clone_name, id.to_string(), clone_path));
                            return;
                        }
                    }
                }
                let _ = clone_sender.send(ApiMessage::VolumeDeletionError("ID non trouvé".to_string()));
            });
        }
        ApiMessage::VolumeIdReceivedForDeletion(name, id, path) => {
            app.dashboard_status = format!("Suppression de {}...", name);
            let clone_sender = app.sender.clone();
            let clone_port = app.current_port_name.clone();
            let clone_id = id.clone();
            let clone_path = path.clone();

            tokio::spawn(async move {
                if !clone_port.is_empty() {
                    if let Ok(mut port) = serialport::new(&clone_port, 115200).timeout(Duration::from_secs(5)).open() {
                        let _ = port.write_data_terminal_ready(true);
                        if send_text_command(&mut *port, &format!("delete_volume={}", clone_id)).is_ok() {
                            let _ = clone_sender.send(ApiMessage::UpdateStatus(
                                "Suppression physique de la partition...".to_string(),
                            ));

                            let script_delete = r#"#!/bin/bash
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
PART_PATH="$1"

if [[ ! -b "$PART_PATH" ]]; then
    exit 0
fi

PARENT_DEV=$(lsblk -no PKNAME "$PART_PATH" | tr -d ' ' | head -n 1)
if [ -z "$PARENT_DEV" ]; then
    exit 1
fi
DISK="/dev/$PARENT_DEV"

PART_NUM=$(echo "$PART_PATH" | grep -oE '[0-9]+$')

/usr/bin/udisksctl unmount -f -b "$PART_PATH" 2>/dev/null || true
/usr/sbin/wipefs -a "$PART_PATH" || true
sleep 1

/usr/sbin/parted -s "$DISK" rm "$PART_NUM"
/usr/sbin/partprobe "$DISK" || true
/usr/bin/udevadm settle
sleep 1
"#;
                            let output = Command::new("pkexec")
                                .arg("/bin/bash")
                                .arg("-c")
                                .arg(script_delete)
                                .arg("_")
                                .arg(&clone_path)
                                .output();

                            // Optionnel: Log du résultat pour le debug si besoin
                            if let Ok(out) = output {
                                println!("Delete Bash Output: {}", String::from_utf8_lossy(&out.stdout));
                                println!("Delete Bash Error: {}", String::from_utf8_lossy(&out.stderr));
                            }

                            let _ = clone_sender.send(ApiMessage::VolumeDeletedOnServer(clone_id));
                            return;
                        }
                    }
                }
                let _ = clone_sender.send(ApiMessage::VolumeDeletionError("Erreur matériel".to_string()));
            });
        }
        ApiMessage::VolumeDeletedOnServer(id) => {
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            let clone_id = id.clone();

            tokio::spawn(async move {
                let _ = clone_api_client.delete(format!("{}/volumes/{}", clone_url, clone_id)).bearer_auth(&clone_token).send().await;
                let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus("Volume supprimé !".to_string()));
                let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
            });
        }
        ApiMessage::VolumeDeletionError(err) => { app.dashboard_status = format!("❌ {}", err); app.is_loading = false; }
        ApiMessage::FormatStatus(texte) => app.formatage_status = texte,
        ApiMessage::UserSearchResult(result) => {
            app.is_searching_user = false;
            match result {
                Ok(info) => {
                    app.share_target_name = Some(info.name);
                    app.share_target_email = Some(info.email);
                    app.share_target_role = Some(info.role);
                    app.share_search_feedback = String::new();
                    app.show_volume_selection = false;
                }
                Err(err_msg) => {
                    app.share_target_name = None;
                    app.share_target_email = None;
                    app.share_target_role = None;
                    app.share_search_feedback = format!("❌ {}", err_msg);
                    app.show_volume_selection = false;
                }
            }
        }
        ApiMessage::SharePipelineStatus(texte) => {
            app.share_pipeline_status = texte.clone();
            if texte.contains("Réussi")
                || texte.contains("Erreur")
                || texte.contains("Échec")
                || texte.contains("Refus")
            {
                app.is_sharing_in_progress = false;
            }
        }
        ApiMessage::UpdateStatus(texte) => app.update_status = texte,
        ApiMessage::SearchUserError(err) => { app.search_result = None; app.enroll_status = err; app.is_loading = false; }
        ApiMessage::UpdateBindKeyError(err) => { app.enroll_status = format!("❌ {}", err); }
        ApiMessage::LogOutError(err) => println!("LogOut Error: {}", err),
        ApiMessage::DeleteUserError(err) => { app.enroll_status = format!("❌ {}", err); }
        ApiMessage::FetchUsersError(err) => { app.enroll_status = format!("❌ {}", err); }
        _ => {}
    }
}
