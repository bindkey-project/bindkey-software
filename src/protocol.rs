use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    GetStatus,
    
    StartEnrollment { username: String },
    
    Unlock { token: String },
}