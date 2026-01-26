use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    StartEnrollment,
    Modify,
    SignChallenge(String),
    CreateVolume(VolumeCreationPayload),
    GetVolume,
}

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

pub enum StatusBindkey {
    ACTIVE,
    RESET,
    LOST,
    BROKEN,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModifyPayload {
    pub email: String,
    pub user_role: Role,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct LoginPayload {
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VerifyPayload {
    pub email: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct LoginSuccessResponse {
    pub local_token: String,
    pub role: Role,
    pub first_name: String,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct ChallengeResponse {
    pub auth_challenge: String,
}

#[derive(PartialEq, Debug)]
pub enum Page {
    Login,
    Home,
    Enrollment,
    Unlock, // Page pour les volumes (à faire plus tard)
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
    LoginError(String),
    EnrollmentError(String),
    ReceivedChallenge(String),
    SignedChallenge(String),
    LoginSuccess(Role, String, String),
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VolumeCreationPayload {
    pub volume_name: String,
    pub size_gb: u32,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VolumeCreatedInfo {
    pub device_name: String,
    pub volume_name: String,
    pub volume_size_gb: u32,
}

#[derive(Debug)]
pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}
