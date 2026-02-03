use serde::{Deserialize, Serialize};
use std::str;
use uuid::Uuid;
use reqwest::{Client, Certificate};
use std::fs;
use std::path::Path;
use std::net::{SocketAddr, IpAddr};
use std::str::FromStr;
use std::time::Duration;
use crate::protocol::share_protocol::UsbResponse;

//----------------------------------------- ÉNUMÉRATION---------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub enum StatusBindkey {
    ACTIVE,
    RESET,
    LOST,
    BROKEN,
}

#[derive(PartialEq, Debug)]
pub enum Page {
    Login,
    Home,
    Enrollment,
    Volume,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Role {
    USER,
    ENROLLEUR,
    ADMIN,
    NONE,
}

pub enum ApiMessage {
    EnrollmentSuccess(String),
    EnrollmentUsbSuccess(UsbResponse),
    ModificationUsbSuccess(UsbResponse),
    LoginError(String),
    EnrollmentError(String),
    ReceivedChallenge(String, Uuid),
    SignedChallenge(String, Uuid),
    LoginSuccess(Role, String, String, String),
    VolumeCreationSuccess(UsbResponse),
    VolumeCreationStatus(String),
    VolumeInfoReceived(UsbResponse),
    FetchUsers,
    FetchUsersError(String),
    UserFetched(Vec<User>),
    LogOutSuccess,
    LogOutError(String),
    DeleteUserError(String),
    DeleteUser(Uuid),
    UserDeleted,
}

//--------------------------ÉNUMÉRATION (FIN)----------------------------

//--------------------------STRUCT LOGIN (DÉBUT)----------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub auth_challenge: String,
    pub session_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginSuccessResponse {
    pub server_token: String,
    pub role: Role,
    pub first_name: String,
    pub local_token: String,
}

//--------------------------STRUCT LOGIN (FIN)------------------------------

//--------------------------STRUCT ENRÔLEMENT (DÉBUT)------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterPayload {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub user_role: Role,
    pub bindkey_uid: String,
    pub bindkey_status: StatusBindkey,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModifyPayload {
    pub email: String,
    pub user_role: Role,
}
//--------------------------STRUCT ENRÔLEMENT (FIN)--------------------------------

//--------------------------STRUCT VOLUME (DÉBUT)--------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeInitInfo {
    pub name: String,
    pub disk_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeInitResponse {
    pub volume_id: String,
    pub exists: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeCreatedInfo {
    pub disk_id: String,
    pub name: String,
    pub size_bytes: u32,
    pub encrypted_key: String,
    pub id: String,
}
//--------------------------STRUCT VOLUME (FIN)--------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct LogOut {
    pub server_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: Role,
}

pub fn create_secure_client() -> Result<Client, String> {
    
    let ip_filename = "server_ip.txt";
    let default_ip = "172.16.253.17"; 

    let ip_str = if Path::new(ip_filename).exists() {
        fs::read_to_string(ip_filename)
            .map_err(|e| format!("Impossible de lire {}: {}", ip_filename, e))?
            .trim() 
            .to_string()
    } else {
        println!("⚠️ Fichier {} introuvable, utilisation de l'IP défaut : {}", ip_filename, default_ip);
        default_ip.to_string()
    };

    let ip_addr = IpAddr::from_str(&ip_str)
        .map_err(|e| format!("IP invalide '{}': {}", ip_str, e))?;
    
    let addr = SocketAddr::new(ip_addr, 8080);

    let cert_bytes = include_bytes!("../../bindkey_cert.pem");
    
    let cert = Certificate::from_pem(cert_bytes)
        .map_err(|e| format!("Certificat PEM invalide/corrompu : {}", e))?;

    let client = Client::builder()
        .add_root_certificate(cert) 
        .resolve("api.bindkey.local", addr) 
        .timeout(Duration::from_secs(10)) 
        .build()
        .map_err(|e| format!("Erreur construction client Reqwest : {}", e))?;

    Ok(client)
}
