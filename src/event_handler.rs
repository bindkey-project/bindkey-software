use crate::protocol::protocol::{
    ApiMessage, LoginSuccessResponse, ModifyPayload, Page, RegisterPayload, Role,
    StatusBindkey::ACTIVE, User, VolumeCreatedInfo,
};
use crate::protocol::share_protocol::{SuccessData, UsbResponse};
use crate::usb_service::send_text_command;
use crate::{BindKeyApp, pages::enrollment::hash_password_with_salt};
use serde_json::json;
use std::time::Duration;
pub fn handke_api_message(app: &mut BindKeyApp, message: ApiMessage) {
    match message {
        ApiMessage::EnrollmentSuccess(texte) => {
            app.enroll_status = texte.to_string();
        }
        ApiMessage::EnrollmentUsbSuccess(data) => {
            match data {
                UsbResponse::Success(SuccessData::EnrollmentInfo { uid, public_key }) => {
                    let clone_sender = app.sender.clone();
                    let clone_firstname = app.enroll_firstname.clone();
                    let clone_lastname = app.enroll_lastname.clone();
                    let clone_email = app.enroll_email.clone();
                    let hash_password = hash_password_with_salt(&app.enroll_password);
                    let clone_user_role = app.enroll_role.clone();
                    let clone_auth_token = app.server_token.clone();
                    let clone_bk_pk = public_key;
                    let clone_bk_uid = uid;
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
                            public_key: clone_bk_pk,
                            bindkey_uid: clone_bk_uid,
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

        ApiMessage::ReceivedChallenge(le_challenge, session_id) => {
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
        ApiMessage::SignedChallenge(signature, session_id) => {
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
        ApiMessage::LoginSuccess(role, token, first_name, local_token) => {
            app.role_user = role;
            app.server_token = token;
            app.first_name_user = first_name;
            app.local_token = local_token;

            app.login_status = String::new();
            app.login_password = String::new();
            app.is_loading = false;

            app.current_page = Page::Home;
        }
        ApiMessage::VolumeCreationSuccess(data) => {
            match data {
                UsbResponse::Success(SuccessData::VolumeCreated {
                    encrypted_key,
                    volume_id,
                }) => {
                    let clone_sender = app.sender.clone();
                    let clone_auth_token = app.server_token.clone();
                    let clone_volume_name = app.volume_created_name.clone();
                    let clone_volume_size = app.volume_created_size;
                    let clone_device_name = app.device_name.clone();
                    let clone_url = app.config.api_url.clone();
                    let clone_api_client = app.api_client.clone();

                    tokio::spawn(async move {
                        let payload = VolumeCreatedInfo {
                            disk_id: clone_device_name,
                            name: clone_volume_name,
                            size_bytes: clone_volume_size,
                            encrypted_key: encrypted_key,
                            id: volume_id,
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
                                        " Refus serveur (API KO)".to_string(),
                                    ));
                                }
                            }
                            Err(e) => {
                                let _ = clone_sender.send(ApiMessage::VolumeCreationStatus(
                                    format!(" Erreur Réseau : {}", e),
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
            let url = format!("{}/users/{}", app.config.api_url, user_id);
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
    }
}
