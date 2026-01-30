use serde::{Deserialize, Serialize};
use std::str;

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
    EnrollmentUsbSuccess(String),
    ModificationUsbSuccess(String),
    LoginError(String),
    EnrollmentError(String),
    ReceivedChallenge(String, String),
    SignedChallenge(String, String),
    LoginSuccess(Role, String, String),
    VolumeCreationSuccess(String),
    VolumeCreationStatus(String),
    VolumeInfoReceived(String),
    FetchUsers,
    FetchUsersError(String),
    UserFetched(Vec<User>),
    LogOutSuccess,
    LogOutError(String),
}

//--------------------------ÉNUMÉRATION (FIN)----------------------------

//--------------------------STRUCT LOGIN (DÉBUT)----------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginPayload {
    pub email: String,
    pub password_hash: String,
    pub bindkey_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub auth_challenge: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyPayload {
    pub session_id: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginSuccessResponse {
    pub server_token: String,
    pub role: Role,
    pub first_name: String,
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
    pub id: String,
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
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: Role,
}
