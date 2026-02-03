use serde::{Deserialize, Serialize};
pub const MAGIC_BYTES: [u8; 2] = [0x42, 0x4B];

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum Command {
    StartEnrollment,
    Modify,
    SignChallenge(String),
    CreateVolume(VolumeCreationPayload),
    GetVolume,
    GetInfo,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    Command = 0x01,
    Response = 0x02,
    Error = 0xEE,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct VolumeCreationPayload {
    pub volume_name: String,
    pub size_gb: u32,
    pub volume_id: String,
    pub mount_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum UsbResponse {
    Success(SuccessData),

    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(u8)]
#[serde(rename_all = "camelCase")]
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
        device_size: u32,
        device_available_size: u32,
        mount_id: i32,
    },
    VolumeCreated {
        encrypted_key: String,
        volume_id: String,
    },

    Ack,
}
