use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    StartEnrollment,
    SignChallenge(String),
    CreateVolume(VolumeCreationPayload),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterPayload {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub user_role: Role,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VerifyPayload {
    pub email: String,
    pub signature: String,
}


#[derive(Serialize, Deserialize, Debug)]

pub struct LoginSuccessResponse {
    pub token: String,
    pub role: Role,
}


#[derive(Serialize, Deserialize, Debug)]

pub struct ChallengeResponse {
    pub challenge: String,
}

#[derive(PartialEq)]
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
    LoginSuccess(Role, String),
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VolumeCreationPayload {
    pub target_device_name: String,
    pub volume_name: String,
    pub size_gb: u32,
}

pub struct CreationState {
    pub is_open: bool,
    pub selected_disk_index: usize, 
    pub volume_size_gb: u32,
    pub volume_name: String,
    pub status: String,
}

pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}