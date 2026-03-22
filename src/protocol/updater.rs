use self_update::cargo_crate_version;

// Ta fameuse clé publique générée précédemment
pub const UPDATE_PUBLIC_KEY: &str = "S2wJypLmrqEuvZs50/ZVXBwoLqyuUz5ff0fTJ7T3stA=";

pub fn update_application() -> Result<String, Box<dyn std::error::Error>> {
    println!("Recherche de mises à jour en cours...");

    // On configure le moteur de mise à jour pour regarder ton dépôt GitHub
    let status = self_update::backends::github::Update::configure()
        .repo_owner("bindkey-project") // Ton compte ou organisation GitHub
        .repo_name("bindkey-software") // Le nom exact de ton dépôt
        .bin_name("bindkey-client")    // Le nom de l'exécutable À L'INTÉRIEUR du .tar.gz
        .show_download_progress(true)
        .current_version(cargo_crate_version!()) // Lit la version actuelle dans ton Cargo.toml
        .build()?
        .update()?; // C'est ici que la magie opère !

    if status.updated() {
        Ok(format!("Mise à jour réussie ! L'application est passée à la version {}", status.version()))
    } else {
        Ok(format!("L'application est déjà à jour (Version {}).", status.version()))
    }
}