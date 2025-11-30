use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollmentRequest {
   pub username :String,
    pub fingerprint_hash: String,
    pub device_id: String,
}

pub fn register_user(username: String, hash: String) -> Result<String, String> {
    
    let payload = EnrollmentRequest {
        username: username.clone(),
        fingerprint_hash: hash,
        device_id: "BK-SIMU-001".to_string(),
    };

    println!(" [API] Tentative d'envoi vers le serveur : {:?}", payload);

    let client = Client::new();

    let response = client.post("http://localhost:8080/api/users")
        .timeout(std::time::Duration::from_secs(2)) 
        .json(&payload)
        .send();

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok("Serveur : Succès (200 OK)".to_string())
            } else {
                Err(format!("Serveur : Erreur HTTP {}", resp.status()))
            }
        }
        Err(e) => {
            println!(" [API ERROR] Le serveur ne répond pas : {}", e);
            Err(format!("Echec connexion serveur : {}", e))
        }
    }
}