use crate::{Role, protocol::RegisterPayload};


pub fn send_hash_server(nom: String, prénom: String, user_email: String, role: Role) {
    let payload = RegisterPayload {
        first_name : nom,
        last_name: prénom,
        email: user_email,
        user_role: role,
    };
}