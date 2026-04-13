use crate::protocol::share_protocol::UsbResponse;
use reqwest::{Certificate, Client};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

//----------------------------------------- ÉNUMÉRATION---------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    ENROLLER,
    ADMIN,
    NONE,
}

#[derive(PartialEq)]
pub enum VolumeTab {
    Dashboard,
    Gestion,
    Formatage,
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
    UpdateStatus(String),
    SearchUserByEmail(String),
    UserFound(UserWithBindKey),
    SearchUserError(String),
    UpdateBindKeyStatus(String, StatusBindkey),
    BindKeyStatusUpdated,
    UpdateBindKeyError(String),
    StartFormatBindKey {
        device_path: String,
        // Adapte le type de partition selon ta structure (ex: Vec<Partition>)
        partitions: Vec<String>,
        port_name: String,
    },
    FormatStatus(String),
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeInitResponse {
    pub volume_id: String,
    pub exists: bool,
}

#[derive(Clone, Debug)]
pub struct VolumeInfo {
    pub name: String,
    pub device_path: String,
    pub total_space_gb: f64,
    pub is_mounted: bool,
    pub mount_point: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeCreatedInfo {
    pub name: String,
    pub size_bytes: u32,
    pub id: String,
}

#[derive(Clone, PartialEq)]
pub struct UsbDevice {
    pub path: String,
    pub display_name: String,
    pub partitions: Vec<String>,
}

#[derive(Deserialize)]
pub struct LsblkOutput {
    pub blockdevices: Vec<BlockDeviceJson>,
}

#[derive(Deserialize, Debug)]
pub struct BlockDeviceJson {
    pub name: String,
    pub model: Option<String>,
    pub size: u64,
    pub tran: Option<String>,
    pub fstype: Option<String>,
    pub pttype: Option<String>,
    pub mountpoint: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub children: Option<Vec<BlockDeviceJson>>,
}

#[derive(Deserialize)]
pub struct PartitionJson {
    pub name: String,
}
//--------------------------STRUCT VOLUME (FIN)--------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct LogOut {
    pub server_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: Role,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BindKeyInfo {
    pub serial_number: String,
    pub status: StatusBindkey,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserWithBindKey {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: Role,
    pub bindkey: Option<BindKeyInfo>,
}

pub fn create_secure_client() -> Result<Client, String> {
    let ip_filename = "server_ip.txt";
    let default_ip = "10.10.10.187";

    let ip_str = if Path::new(ip_filename).exists() {
        fs::read_to_string(ip_filename)
            .map_err(|e| format!("Impossible de lire {}: {}", ip_filename, e))?
            .trim()
            .to_string()
    } else {
        println!(
            "⚠️ Fichier {} introuvable, utilisation de l'IP défaut : {}",
            ip_filename, default_ip
        );
        default_ip.to_string()
    };

    let ip_addr =
        IpAddr::from_str(&ip_str).map_err(|e| format!("IP invalide '{}': {}", ip_str, e))?;

    let addr = SocketAddr::new(ip_addr, 31278);

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
