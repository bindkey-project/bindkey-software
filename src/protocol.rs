use serde::{Deserialize, Serialize};
use crate::Role;

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    StartEnrollment,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterPayload {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub user_role: Role,
}