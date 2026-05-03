# Projet BindKey - Software Client

## Présentation du Projet
BindKey est un logiciel en Rust conçu pour gérer l'interface entre un proxy USB matériel (basé sur ESP32) et un serveur backend sécurisé. Il permet l'enrôlement d'utilisateurs, l'authentification forte (challenge-response matériel) et la gestion sécurisée de volumes de stockage sur Linux.

### Technologies Clés
- **Langage :** Rust (Edition 2024)
- **Interface Graphique :** `egui` / `eframe`
- **Asynchronisme :** `tokio`
- **Communication Matérielle :** `serialport` (vitesse 115200)
- **Communication Réseau :** `reqwest` (HTTPS avec certificat embarqué)
- **Gestion Disque :** Commandes système Linux (`lsblk`, `parted`, `wipefs`, `udisksctl`, `udevadm`) via `pkexec`.

---

## Architecture et Conventions

### Structure des Fichiers
- `src/main.rs` : Point d'entrée, initialisation de l'App egui et détection de présence série.
- `src/event_handler.rs` : Cœur de la logique applicative. Gère les messages `ApiMessage` pour coordonner l'UI, le matériel et le réseau.
- `src/pages/` : Contient les différents écrans de l'application (Login, Home, Enrollment, Volumes).
- `src/protocol/` : Définition des structures de données, du protocole de partage et de l'updater.
- `src/usb_service.rs` : Utilitaires pour la communication série textuelle avec le firmware.

### Conventions de Développement
1. **Gestion des erreurs :** Utiliser `anyhow` pour les erreurs globales et des types explicites pour les erreurs de protocole.
2. **Asynchronisme :** Toute opération bloquante (I/O série, réseau, commandes système) doit être exécutée dans un `tokio::spawn`.
3. **Communication :** L'UI communique avec la logique via un canal `mpsc` envoyant des `ApiMessage`.
4. **Sécurité :** Les manipulations de disques utilisent `pkexec`. Ne jamais stocker de secrets en clair.

---

## Commandes Utiles

### Build et Run
- **Lancer l'application :** `cargo run`
- **Compiler en mode release :** `cargo build --release`

### Tests
- **Exécuter les tests :** `cargo test` (Note: Peu de tests automatisés actuellement, privilégier les tests d'intégration manuels avec le matériel).

---

## Guide d'Utilisation des Outils Système
Le logiciel s'appuie lourdement sur les utilitaires Linux. En cas de problème de détection ou de formatage, vérifier :
1. **lsblk :** Utilisé pour lister les périphériques USB. Le filtre `contains("BINDKEY")` est appliqué sur le modèle.
2. **Serial Port :** L'ESP32 est identifié par le VID `0x10c4` et le PID `0xea60`.
3. **Permissions :** L'utilisateur doit avoir les droits pour exécuter `pkexec` afin de modifier les partitions.

---

## TODO / Améliorations futures
- [ ] Finaliser la suppression de volume sur le firmware (commande `delete_volume`).
- [ ] Améliorer la couverture de tests unitaires sur la désérialisation du protocole.
- [ ] Optimiser la détection `lsblk` pour éviter les faux négatifs.
