use crate::pages::volumes::rollback_physical_volume;
use crate::protocol::protocol::{
    ApiMessage, LoginSuccessResponse, ModifyPayload, Page, RegisterPayload, Role,
    StatusBindkey::ACTIVE, User, VolumeCreatedInfo,
};
use crate::protocol::protocol::{StatusBindkey, UserWithBindKey};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use crate::usb_service::send_text_command;
use crate::{BindKeyApp, pages::enrollment::hash_password_with_salt};
use serde_json::json;
use std::time::Duration;
pub fn handle_api_message(app: &mut BindKeyApp, message: ApiMessage) {
    match message {
        ApiMessage::EnrollmentSuccess(texte) => {
            app.enroll_status = texte.to_string();
        }
        ApiMessage::EnrollmentUsbSuccess(data) => {
            match data {
                UsbResponse::Success(SuccessData::EnrollmentInfo {
                    sn,
                    pub_sign,
                    pub_ecdh,
                }) => {
                    let clone_sender = app.sender.clone();
                    let clone_firstname = app.enroll_firstname.clone();
                    let clone_lastname = app.enroll_lastname.clone();
                    let clone_email = app.enroll_email.clone();
                    let hash_password = hash_password_with_salt(&app.enroll_password);
                    let clone_user_role = app.enroll_role.clone();
                    let clone_auth_token = app.server_token.clone();
                    let clone_bk_pub_sign = pub_sign;
                    let clone_bk_pub_ecdh = pub_ecdh;
                    let clone_bk_sn = sn;
                    let clone_url = app.config.api_url.clone();
                    let clone_api_client = app.api_client.clone();

                    tokio::spawn(async move {
                        let payload = RegisterPayload {
                            first_name: clone_firstname,
                            last_name: clone_lastname,
                            email: clone_email,
                            password: hash_password,
                            user_role: clone_user_role,
                            bindkey_status: ACTIVE,
                            pub_sign: clone_bk_pub_sign,
                            sn: clone_bk_sn,
                            pub_ecdh: clone_bk_pub_ecdh,
                        };
                        println!("{:?}", payload);
                        let url = format!("{}/auth/register", clone_url);
                        let resultat = clone_api_client
                            .post(&url)
                            .json(&payload)
                            .bearer_auth(clone_auth_token)
                            .send()
                            .await;

                        match resultat {
                            Ok(response) => {
                                if response.status().is_success() {
                                    let _ = clone_sender.send(ApiMessage::EnrollmentSuccess(
                                        " Enrolé (API OK) !".to_string(),
                                    ));
                                } else {
                                    let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                        " Refus serveur (API KO)".to_string(),
                                    ));
                                }
                            }
                            Err(e) => {
                                let _ = clone_sender.send(ApiMessage::EnrollmentError(format!(
                                    " Erreur Réseau : {}",
                                    e
                                )));
                            }
                        }
                    });
                    app.enroll_password.clear();
                }
                UsbResponse::Error(msg) => {
                    app.enroll_status = format!(" Erreur Clé USB : {}", msg);
                }
                _ => {
                    app.enroll_status = "Erreur Protocole : Données inattendues reçues".to_string();
                }
            };
        }
        ApiMessage::ModificationUsbSuccess(data) => match data {
            UsbResponse::Success(SuccessData::Ack) => {
                let clone_sender = app.sender.clone();
                let clone_email = app.enroll_email.clone();
                let clone_user_role = app.enroll_role.clone();
                let clone_auth_token = app.server_token.clone();
                let clone_url = app.config.api_url.clone();
                let clone_api_client = app.api_client.clone();

                tokio::spawn(async move {
                    let payload = ModifyPayload {
                        email: clone_email,
                        user_role: clone_user_role,
                    };
                    let url = format!("{}/users/modify", clone_url);
                    let resultat = clone_api_client
                        .post(&url)
                        .json(&payload)
                        .bearer_auth(clone_auth_token)
                        .send()
                        .await;
                    match resultat {
                        Ok(response) => {
                            if response.status().is_success() {
                                let _ = clone_sender.send(ApiMessage::EnrollmentSuccess(
                                    " Modifié (API OK) !".to_string(),
                                ));
                            } else {
                                println!(
                                    "code: {}, {:?}",
                                    response.status(),
                                    response
                                        .text()
                                        .await
                                        .unwrap_or_else(|_| "Impossible".to_string())
                                );
                                let _ = clone_sender.send(ApiMessage::EnrollmentError(
                                    " Refus serveur (API KO)".to_string(),
                                ));
                            }
                        }
                        Err(e) => {
                            let _ = clone_sender.send(ApiMessage::EnrollmentError(format!(
                                " Erreur Réseau : {}",
                                e
                            )));
                        }
                    }
                });
            }
            UsbResponse::Error(msg) => {
                app.enroll_status = format!(" Erreur Clé USB : {}", msg);
            }
            _ => {
                app.enroll_status = "Erreur Protocole : Données inattendues reçues".to_string();
            }
        },
        ApiMessage::LoginError(texte) => {
            app.login_status = texte.to_string();
            app.is_loading = false;
        }
        ApiMessage::EnrollmentError(texte) => {
            app.enroll_status = texte.to_string();
            app.is_loading = false;
        }

        ApiMessage::ReceivedChallenge(le_challenge, session_id, bindkey_uid) => {
            app.login_status =
                "Challenge reçue, communication avec la bindkey en cours".to_string();
            app.is_loading = true;
            let clone_sender = app.sender.clone();
            let clone_port_name = app.current_port_name.clone();
            tokio::spawn(async move {
                if !clone_port_name.is_empty() {
                    match serialport::new(&clone_port_name, 115200)
                        .timeout(Duration::from_secs(15))
                        .open()
                    {
                        Ok(mut port) => {
                            let _ = port.write_data_terminal_ready(true);
                            std::thread::sleep(Duration::from_millis(100));

                            let cmd = format!("challenge={}", le_challenge);
                            let _ = clone_sender
                                .send(ApiMessage::LoginError("Scannez votre doigt".to_string()));
                            match send_text_command(&mut *port, &cmd) {
                                Ok(map) => {
                                    if let Some(sig) = map.get("SIG") {
                                        println!("🔍 DEBUG SIGNATURE: '{}, {}'", sig, sig.len());
                                        let _ = clone_sender.send(ApiMessage::SignedChallenge(
                                            sig.clone(),
                                            session_id,
                                            bindkey_uid,
                                        ));
                                    } else {
                                        let _ = clone_sender.send(ApiMessage::LoginError(
                                            "La clé a répondu mais sans SIG".to_string(),
                                        ));
                                    }
                                }
                                Err(e) => {
                                    let _ = clone_sender.send(ApiMessage::LoginError(format!(
                                        "Erreur Com USB: {}",
                                        e
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = clone_sender.send(ApiMessage::LoginError(format!(
                                "Impossible d'ouvrir le port: {}",
                                e
                            )));
                        }
                    }
                } else {
                    let _ =
                        clone_sender.send(ApiMessage::LoginError("Clé non détectée".to_string()));
                }
            });
        }
        ApiMessage::SignedChallenge(signature, session_id, bindkey_uid) => {
            app.login_status =
                "Signature générée. Vérification finale auprès du serveur".to_string();
            app.is_loading = true;
            let clone_session_id = session_id.clone();
            let clone_signature = signature.clone();
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_api_client = app.api_client.clone();

            tokio::spawn(async move {
                let payload = json!({
                    "session_id": clone_session_id,
                    "signature": clone_signature,
                });
                let resultat = clone_api_client
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
                                        response.server_token,
                                        response.first_name,
                                        response.local_token,
                                        bindkey_uid,
                                    ));
                                }
                                Err(e) => {
                                    let _ =
                                        clone_sender.send(ApiMessage::LoginError(e.to_string()));
                                }
                            }
                        } else {
                            let _ = clone_sender.send(ApiMessage::LoginError(
                                "Signature refusée par le serveur".to_string(),
                            ));
                            println!(
                                "code: {}, {:?}",
                                response.status(),
                                response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Impossible".to_string())
                            );
                            println!("session_id: {}", session_id);
                        }
                    }
                    Err(error) => {
                        let _ = clone_sender.send(ApiMessage::LoginError(error.to_string()));
                    }
                }
            });
        }
        ApiMessage::LoginSuccess(role, token, first_name, local_token, bindkey_uid) => {
            app.role_user = role;
            app.server_token = token;
            app.first_name_user = first_name;
            app.local_token = local_token;

            app.login_status = String::new();
            app.login_password = String::new();
            app.is_loading = false;
            app.local_bindkey_sn = Some(bindkey_uid);
            app.current_page = Page::Home;
        }
        ApiMessage::VolumeCreationSuccess(data) => {
            match data {
                UsbResponse::Success(SuccessData::VolumeCreated {
                    volume_id,
                    device_path,
                    partition_number,
                }) => {
                    let clone_sender = app.sender.clone();
                    let clone_auth_token = app.server_token.clone();
                    let clone_volume_name = app.volume_created_name.trim().to_uppercase();
                    let clone_volume_size = app.volume_created_size;
                    let clone_device_name = app.device_name.clone();
                    let clone_url = app.config.api_url.clone();
                    let clone_api_client = app.api_client.clone();
                    let clone_port = app.current_port_name.clone();

                    tokio::spawn(async move {
                        let payload = VolumeCreatedInfo {
                            name: clone_volume_name,
                            size_bytes: clone_volume_size,
                            id: volume_id.clone(),
                        };
                        let url = format!("{}/volumes", clone_url);
                        let resultat = clone_api_client
                            .post(&url)
                            .json(&payload)
                            .bearer_auth(clone_auth_token)
                            .send()
                            .await;
                        match resultat {
                            Ok(response) => {
                                if response.status().is_success() {
                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                        "Volume enregistré sur le serv !".to_string(),
                                    ));
                                } else {
                                    let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                        format!(
                                            "Refus serveur ({}). Suppression du volume en cours...",
                                            response.status()
                                        ),
                                    ));
                                    /*
                                                                            rollback_physical_volume(
                                                                                &device_path,
                                                                                &partition_number,
                                                                                &clone_port,
                                                                                &volume_id,
                                                                            );

                                        let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                            "Volume annulé proprement suite à l'erreur serveur."
                                                .to_string(),
                                        ));
                                    */
                                }
                            }
                            Err(e) => {
                                let _ =
                                    clone_sender.send(ApiMessage::VolumeCreationStatus(format!(
                                        "Erreur Réseau : {}. Suppression du volume en cours...",
                                        e
                                    )));
                                /*
                                                                rollback_physical_volume(
                                                                    &device_path,
                                                                    &partition_number,
                                                                    &clone_port,
                                                                    &volume_id,
                                                                );
                                */
                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                    "Volume annulé proprement suite à la coupure réseau."
                                        .to_string(),
                                ));
                            }
                        }
                    });
                }
                UsbResponse::Error(msg) => {
                    app.volume_status = format!(" Erreur Clé USB : {}", msg);
                }
                _ => {
                    app.volume_status = "Erreur Protocole : Données inattendues reçues".to_string();
                }
            };
        }
        ApiMessage::VolumeCreationStatus(texte) => {
            app.volume_status = texte.to_string();
        }
        ApiMessage::VolumeDashboardStatus(texte) => {
            app.dashboard_status = texte.to_string();
            if texte.contains("succès") || texte.contains("Erreur") || texte.contains("❌") {
                app.is_loading = false;
            }
        }
        ApiMessage::VolumeInfoReceived(data) => {
            match data {
                UsbResponse::Success(SuccessData::DeviceInfo {
                    device_name,
                    device_size,
                    device_available_size,
                }) => {
                    app.device_name = device_name;
                    app.device_size = device_size;
                    app.device_available_space = device_available_size;
                    app.volume_status = "Disque analysé avec succès.".to_string();
                }
                UsbResponse::Error(msg) => {
                    app.volume_status = format!(" Erreur Clé USB : {}", msg);
                }
                _ => {
                    app.volume_status = "Erreur Protocole : Données inattendues reçues".to_string();
                }
            };
        }
        ApiMessage::FetchUsers => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            tokio::spawn(async move {
                let url = format!("{}/admin/users", url);
                let resultat = clone_api_client
                    .get(url)
                    .bearer_auth(clone_auth_token)
                    .send()
                    .await;

                match resultat {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<Vec<User>>().await {
                                Ok(users) => {
                                    let _ = clone_sender.send(ApiMessage::UserFetched(users));
                                }
                                Err(e) => {
                                    let _ = clone_sender
                                        .send(ApiMessage::FetchUsersError(format!("{}", e)));
                                }
                            }
                        } else {
                            let _ = clone_sender.send(ApiMessage::FetchUsersError(format!(
                                "Erreur serveur: {}",
                                response.status()
                            )));
                            println!(
                                "code: {}, {:?}",
                                response.status(),
                                response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Impossible".to_string())
                            );
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender
                            .send(ApiMessage::FetchUsersError(format!("Erreur réseau: {}", e)));
                    }
                }
            });
        }
        ApiMessage::UserFetched(users) => {
            app.users_list = users;
            app.enroll_status = format!("Liste mise à jour : {}", app.users_list.len());
        }
        ApiMessage::FetchUsersError(e) => {
            app.enroll_status = format!("Erreur dans la mise à jour de la liste: {}", e);
        }
        ApiMessage::DeleteUser(user_id) => {
            let clone_sender = app.sender.clone();
            let url = format!("{}/admin/users/{}", app.config.api_url, user_id);
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            tokio::spawn(async move {
                let resultat = clone_api_client
                    .delete(url)
                    .bearer_auth(clone_auth_token)
                    .send()
                    .await;

                match resultat {
                    Ok(reponse) => {
                        if reponse.status().is_success() {
                            let _ = clone_sender.send(ApiMessage::UserDeleted);
                        } else {
                            let _ = clone_sender.send(ApiMessage::DeleteUserError(format!(
                                "Erreur lors de la suppression: {}",
                                reponse.status()
                            )));
                            println!(
                                "code: {}, {:?}",
                                reponse.status(),
                                reponse
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Impossible".to_string())
                            );
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender.send(ApiMessage::DeleteUserError(format!(
                            "Erreur serveur: {}",
                            e
                        )));
                    }
                }
            });
        }
        ApiMessage::UserDeleted => {
            app.enroll_status = "Utilisateur bien supprimé".to_string();
            let _ = app.sender.send(ApiMessage::FetchUsers);
        }
        ApiMessage::DeleteUserError(e) => {
            app.enroll_status = format!("Échec de la suppression: {}", e);
        }
        ApiMessage::SearchUserByEmail(email) => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            app.enroll_status = "Recherche en cours...".to_string();

            tokio::spawn(async move {
                // Option A : Ton API possède une route de recherche par email
                let url = format!("{}/admin/users/search?email={}", url, email);

                let resultat = clone_api_client
                    .get(&url)
                    .bearer_auth(clone_auth_token)
                    .send()
                    .await;

                match resultat {
                    Ok(response) => {
                        if response.status().is_success() {
                            // Attention: il faut adapter la désérialisation selon ce que renvoie ton backend !
                            // Ex: Une structure json { "user": {..}, "bindkey": {..} }
                            // Ici on simule que l'API renvoie juste l'User pour l'instant
                            if let Ok(user_data) = response.json::<UserWithBindKey>().await {
                                // TODO: Récupérer la BindKey (soit incluse dans user_data, soit via un 2ème appel API)
                                let _ = clone_sender.send(ApiMessage::UserFound(user_data));
                            }
                        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                            let _ = clone_sender.send(ApiMessage::SearchUserError(
                                "Utilisateur introuvable".to_string(),
                            ));
                        } else {
                            let _ = clone_sender.send(ApiMessage::SearchUserError(format!(
                                "Erreur serveur: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender
                            .send(ApiMessage::SearchUserError(format!("Erreur réseau: {}", e)));
                    }
                }
            });
        }
        ApiMessage::UserFound(user) => {
            app.search_result = Some(user);
            app.enroll_status = "Utilisateur trouvé.".to_string();
        }

        ApiMessage::SearchUserError(e) => {
            app.search_result = None;
            app.enroll_status = e;
        }
        ApiMessage::UpdateBindKeyStatus(serial, new_status) => {
            let clone_sender = app.sender.clone();
            let url = app.config.api_url.clone();
            let clone_auth_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            // On prévient l'utilisateur que ça charge
            app.enroll_status = "⏳ Mise à jour du statut de la clé en cours...".to_string();

            tokio::spawn(async move {
                // Adapte cette URL selon ce que ton ingénieur backend a défini
                let url = format!("{}/admin/bindkeys/{}/status", url, serial);

                // On convertit l'enum en String pour que le JSON soit propre
                let status_str = match new_status {
                    StatusBindkey::ACTIVE => "ACTIVE",
                    StatusBindkey::RESET => "RESET",
                    StatusBindkey::LOST => "LOST",
                    StatusBindkey::BROKEN => "BROKEN",
                };

                // Création du payload JSON à la volée avec serde_json
                let payload = serde_json::json!({
                    "status": status_str
                });

                // On utilise PATCH (ou PUT selon ton backend)
                let resultat = clone_api_client
                    .patch(&url)
                    .json(&payload)
                    .bearer_auth(clone_auth_token)
                    .send()
                    .await;

                match resultat {
                    Ok(response) => {
                        if response.status().is_success() {
                            let _ = clone_sender.send(ApiMessage::BindKeyStatusUpdated);
                        } else {
                            let _ = clone_sender.send(ApiMessage::UpdateBindKeyError(format!(
                                "Refus serveur: {}",
                                response.status()
                            )));
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender.send(ApiMessage::UpdateBindKeyError(format!(
                            "Erreur réseau: {}",
                            e
                        )));
                    }
                }
            });
        }
        ApiMessage::BindKeyStatusUpdated => {
            app.enroll_status =
                "Le statut de la BindKey a été mis à jour avec succès !".to_string();

            // 🔊 Si tu as mis en place rodio, tu peux mettre un play_success_sound() ici !
        }

        ApiMessage::UpdateBindKeyError(e) => {
            app.enroll_status = format!("Échec de la mise à jour : {}", e);

            // 🔊 play_error_sound();
        }
        ApiMessage::StartFormatBindKey {
            device_path,
            partitions,
            port_name,
            volume_names,
        } => {
            let clone_sender = app.sender.clone();
            app.formatage_status = "Nettoyage des volumes sur le serveur...".to_string();

            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();

            tokio::spawn(async move {
                for name in volume_names {
                    let url_find = format!("{}/volumes/find_id?name={}", clone_url, name);
                    if let Ok(resp) = clone_api_client
                        .get(&url_find)
                        .bearer_auth(&clone_token)
                        .send()
                        .await
                    {
                        if resp.status().is_success() {
                            if let Ok(data) = resp.json::<serde_json::Value>().await {
                                if let Some(id) = data.get("volume_id").and_then(|v| v.as_str()) {
                                    let url_del = format!("{}/volumes/{}", clone_url, id);
                                    let _ = clone_api_client
                                        .delete(&url_del)
                                        .bearer_auth(&clone_token)
                                        .send()
                                        .await;
                                }
                            }
                        }
                    }
                }
                // 1. Ouverture du port série
                match serialport::new(&port_name, 115200)
                    .timeout(std::time::Duration::from_secs(10))
                    .open()
                {
                    Ok(mut port) => {
                        let _ = port.write_data_terminal_ready(true);
                        let _ = port.write_request_to_send(true);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        let cmd_init_format = "action=init_format\n".to_string();
                        let _ = clone_sender.send(ApiMessage::FormatStatus(
                            "Avertissement de la BindKey en cours...".to_string(),
                        ));

                        // 2. Envoi de l'avertissement à la puce
                        match crate::usb_service::send_text_command(&mut *port, &cmd_init_format) {
                            Ok(map) => {
                                let is_ok = map
                                    .get("STATUS")
                                    .map(|val| val.contains("OK"))
                                    .unwrap_or(false);

                                if is_ok {
                                    let _ = clone_sender.send(ApiMessage::FormatStatus(
                                        "BindKey prête. Démarrage du formatage Linux..."
                                            .to_string(),
                                    ));

                                    // 3. Formatage de la clé USB (Le disque)
                                    let format_result = tokio::task::spawn_blocking(move || {
                                        crate::pages::volumes::force_format(
                                            &device_path,
                                            &partitions,
                                        )
                                    })
                                    .await;

                                    match format_result {
                                        Ok(Ok(_)) => {
                                            let _ = clone_sender.send(ApiMessage::FormatStatus(
                                                "Succès : La clé est vide et réinitialiséé."
                                                    .to_string(),
                                            ));
                                            let _ =
                                                clone_sender.send(ApiMessage::RequestVolumeRefresh);
                                        }
                                        Ok(Err(e)) => {
                                            let _ = clone_sender.send(ApiMessage::FormatStatus(
                                                format!("Erreur système lors du formatage : {}", e),
                                            ));
                                        }
                                        Err(e) => {
                                            let _ = clone_sender.send(ApiMessage::FormatStatus(
                                                format!("Erreur fatale du thread : {}", e),
                                            ));
                                        }
                                    }
                                } else {
                                    let _ = clone_sender.send(ApiMessage::FormatStatus(
                                        "Erreur: La puce a refusé de s'initialiser".to_string(),
                                    ));
                                }
                            }
                            Err(e) => {
                                let _ = clone_sender.send(ApiMessage::FormatStatus(format!(
                                    "Échec de communication USB : {}",
                                    e
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = clone_sender.send(ApiMessage::FormatStatus(format!(
                            "Impossible d'ouvrir le port USB : {}",
                            e
                        )));
                    }
                }
            });
        }
        // Mise à jour de l'affichage dans l'UI
        ApiMessage::FormatStatus(status) => {
            app.formatage_status = status.clone();
            if status.contains("Succès") || status.contains("Erreur") || status.contains("Échec")
            {
                app.is_loading = false;
            }
        }

        ApiMessage::UserSearchResult(result) => {
            app.is_searching_user = false;
            match result {
                Ok(info) => {
                    app.share_target_name = Some(info.name);
                    app.share_target_email = Some(info.email);
                    app.share_target_role = Some(info.role);
                    app.share_search_feedback = String::new(); // On efface les erreurs
                    app.show_volume_selection = false; // On reset l'étape suivante au cas où
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
        ApiMessage::RequestVolumeRefresh => {
            app.needs_volume_refresh = true;
        }
        ApiMessage::LogOutSuccess => {
            app.current_page = Page::Login;
            app.role_user = Role::NONE;

            app.server_token.clear();
            app.local_token.clear();
            app.login_password.clear();

            app.first_name_user.clear();
            app.enroll_firstname.clear();
            app.enroll_lastname.clear();
            app.enroll_email.clear();
            app.enroll_status.clear();

            app.device_available_space = 0.0;
            app.device_name.clear();
            app.device_size = 0.0;
            app.volume_created_name.clear();
            app.volume_created_size = 0;
            app.volume_status.clear();
            app.users_list.clear();

            app.login_status = " Déconnexion réussie.".to_string();
        }
        ApiMessage::LogOutError(e) => {
            println!("{}", e);
        }
        ApiMessage::UpdateStatus(texte) => {
            app.update_status = texte;
        }
        ApiMessage::SharePipelineStatus(text) => {
            app.share_pipeline_status = text;
        }
        ApiMessage::StartVolumeDeletion(name) => {
            app.dashboard_status = format!("Recherche de l'ID pour le volume {}...", name);
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            let clone_name = name.clone();

            tokio::spawn(async move {
                // On suppose qu'il y a une route pour récupérer l'ID par le nom
                let url = format!("{}/volumes/find_id?name={}", clone_url, clone_name);
                let res = clone_api_client
                    .get(&url)
                    .bearer_auth(&clone_token)
                    .send()
                    .await;

                match res {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            if let Some(id) = data.get("volume_id").and_then(|v| v.as_str()) {
                                let _ = clone_sender.send(ApiMessage::VolumeIdReceivedForDeletion(
                                    clone_name,
                                    id.to_string(),
                                ));
                            } else {
                                let _ = clone_sender.send(ApiMessage::VolumeDeletionError(
                                    "ID non trouvé dans la réponse".to_string(),
                                ));
                            }
                        }
                    }
                    _ => {
                        let _ = clone_sender.send(ApiMessage::VolumeDeletionError(
                            "Impossible de trouver l'ID du volume sur le serveur".to_string(),
                        ));
                    }
                }
            });
        }
        ApiMessage::VolumeIdReceivedForDeletion(name, id) => {
            app.dashboard_status =
                format!("Suppression du volume {} ({}) sur le serveur...", name, id);
            let clone_sender = app.sender.clone();
            let clone_url = app.config.api_url.clone();
            let clone_token = app.server_token.clone();
            let clone_api_client = app.api_client.clone();
            let clone_id = id.clone();

            tokio::spawn(async move {
                let url = format!("{}/volumes/{}", clone_url, clone_id);
                let res = clone_api_client
                    .delete(&url)
                    .bearer_auth(&clone_token)
                    .send()
                    .await;

                match res {
                    Ok(resp) if resp.status().is_success() => {
                        let _ = clone_sender.send(ApiMessage::VolumeDeletedOnServer(clone_id));
                    }
                    _ => {
                        let _ = clone_sender.send(ApiMessage::VolumeDeletionError(
                            "Échec de la suppression sur le serveur".to_string(),
                        ));
                    }
                }
            });
        }
        ApiMessage::VolumeDeletedOnServer(id) => {
            app.dashboard_status = format!("Suppression du volume {} sur la BindKey...", id);
            let clone_sender = app.sender.clone();
            let clone_port = app.current_port_name.clone();
            let clone_id = id.clone();

            tokio::spawn(async move {
                if !clone_port.is_empty() {
                    match serialport::new(&clone_port, 115200)
                        .timeout(Duration::from_secs(5))
                        .open()
                    {
                        Ok(mut port) => {
                            let _ = port.write_data_terminal_ready(true);
                            let cmd = format!("delete_volume={}", clone_id);
                            match send_text_command(&mut *port, &cmd) {
                                Ok(_) => {
                                    let _ = clone_sender.send(ApiMessage::VolumeDashboardStatus(
                                        "Volume supprimé avec succès !".to_string(),
                                    ));
                                    let _ = clone_sender.send(ApiMessage::RequestVolumeRefresh);
                                }
                                Err(e) => {
                                    let _ = clone_sender.send(ApiMessage::VolumeDeletionError(
                                        format!("Erreur USB : {}", e),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = clone_sender.send(ApiMessage::VolumeDeletionError(format!(
                                "Port USB indisponible : {}",
                                e
                            )));
                        }
                    }
                } else {
                    let _ = clone_sender.send(ApiMessage::VolumeDeletionError(
                        "BindKey non connectée pour la suppression finale".to_string(),
                    ));
                }
            });
        }
        ApiMessage::VolumeDeletionError(err) => {
            app.dashboard_status = format!("❌ Erreur : {}", err);
            app.is_loading = false;
        }
    }
}
