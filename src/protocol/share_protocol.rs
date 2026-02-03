use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    Command = 0x01,
    Response = 0x02,
    Error = 0xEE,
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
    },
    VolumeCreated {
        encrypted_key: String,
        volume_id: String,
    },

    Ack,
}
