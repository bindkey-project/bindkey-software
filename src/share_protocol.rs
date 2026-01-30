use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    StartEnrollment,
    Modify,
    SignChallenge(String),
    CreateVolume(VolumeCreationPayload),
    GetVolume,
    GetInfo,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VolumeCreationPayload {
    pub volume_name: String,
    pub size_gb: u32,
    pub volume_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", content = "data")]
pub enum UsbResponse {
    #[serde(rename = "SUCCESS")]
    Success(SuccessData),

    #[serde(rename = "ERROR")]
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum SuccessData {
    EnrollmentInfo {
        uid: String,
        public_key: String,
    },

    Signature {
        signature: String,
    },

    DeviceInfo {
        device_name: String,
        device_size: String,
        device_available_size: u32,
    },
    VolumeCreated {
        encrypted_key: String,
        volume_id: String,
    },

    Ack {},
}
