use std::str;

use serde::{Deserialize, Serialize};
use crate::Role;

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