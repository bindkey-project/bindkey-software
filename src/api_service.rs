use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollmentRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: String,

    pub fingerprint_hash: String,
    pub device_id: String,
}

pub fn register_user(
    first_name: String,
    last_name: String,
    email: String,
    role: String,
    hash: String,
) -> Result<String, String> {
    let payload = EnrollmentRequest {
        first_name,
        last_name,
        email,
        role,
        fingerprint_hash: hash,
        device_id: "BK-SIMU-001".to_string(),
    };

    println!(" [API] Envoi vers PostgreSQL : {:?}", payload);

    let client = Client::new();

    let response = client
        .post("http://localhost:8080/api/users")
        .timeout(std::time::Duration::from_secs(2))
        .json(&payload)
        .send();

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok("Serveur : Utilisateur créé avec succès".to_string())
            } else {
                Err(format!("Serveur : Erreur {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Echec connexion : {}", e)),
    }
}
