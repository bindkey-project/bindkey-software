use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    StartEnrollment,
    SignChallenge(String),
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