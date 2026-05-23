<p align="center"> <img src="assets/logo-bindkey.png" alt="Logo BindKey" width="450"/> </p> 
<h1 align="center">BindKey</h1> 
<p align="center"><i>Security at your fingertip</i></p>

## Global Overview of the BindKey Project
BindKey is a hardware cybersecurity solution that resolves the compromise between offline data security and the need for enterprise collaboration. The device sits between the PC and a standard storage medium (USB key, SSD, SD reader) and acts as a legitimate Man-in-the-Middle: all data passing through it is sealed and encrypted on the fly in AES-256-GCM by a secure microcontroller, and is decrypted only for biometrically authenticated users whose BindKey holds the rights to the volume. Any modification made outside the BindKey environment makes the content unreadable. The entire system operates off-cloud, without host drivers, on Windows / Linux / macOS.

## The Three Pillars of the Project
*   **The BindKey hardware proxy** — the physical enclosure providing local biometric authentication, key derivation via an ATECC608A secure element, and on-the-fly data encryption. Composed of two ESP32-S3 microcontrollers:
    *   A master (USB MSC emulation + biometrics + AES-GCM crypto + secure element) — repository `bindkey-tinyesp`
    *   A slave (driving the actual physical media as USB Host) — repository `bindkey-esp`
*   **A backend server (API)** — repository `bindkey-server` — manages public user identities and orchestrates access delegation between BindKeys in Zero-Knowledge mode: only wrapped keys (via ECDH) circulate on the network, never the plaintext volume key.
*   **The desktop software** — repository `bindkey-software` **(← this repository)** — a Rust application that controls the BindKey via UART and provides the graphical interface: volume creation/deletion, sharing with a colleague, formatting, resetting, and any administration operation requiring the physical presence of the key.

## Main Features
*   **Transparent on-the-fly encryption** — no host-side driver, standard USB MSC key behavior.
*   **Local biometric fingerprint authentication**, hardware-gated and anti-replay.
*   **Provable integrity** — any modification outside BindKey makes the data unreadable (AES-GCM tag).
*   **Collaborative sharing** between BindKeys within the same organization via ECDH P-256 (Zero-Knowledge).
*   **Delegation of enrollment** — an administrator can grant Enroller privileges to a team leader.
*   **Lifecycle management** — remote revocation, restoration via recovery code, wipe & reassign.
*   **Centralized tamper-evident audit log** (GDPR compliance and forensic traceability).
*   **Air-gapped maintenance** — secure transport of payloads to isolated systems (OT, industrial).

---

## System Prerequisites
This software relies heavily on the **Linux** ecosystem for low-level disk management. 
The following utilities must be installed on your system:
* `rustc` & `cargo` (Edition 2024 recommended)
* `parted` (GPT Partitioning)
* `lsblk` (Disk enumeration)
* `udisksctl` (Secure unmounting)
* `wipefs` (Wiping filesystem signatures)
* `udevadm` & `partprobe` (Kernel cache updates)
* `pkexec` (Polkit) for privilege escalation during critical disk operations.

*Important: The current user must be part of the `dialout` or `uucp` group (depending on the Linux distribution) to have read/write permissions on the BindKey's Serial port without being root.*

## Installation and Execution

1. **Clone the repository:**
   ```bash
   git clone <your-repo>
   cd bindkey-software
   ```

2. **Compilation and Development Mode:**
   To quickly test the code during development:
   ```bash
   cargo run
   ```

3. **Build and Use the Executable (Production Mode):**
   For optimal performance and to use it without relying on Cargo, build the final executable:
   ```bash
   cargo build --release
   ```
   The compiled executable will be available in the `target/release/` folder. You can launch it directly:
   ```bash
   ./target/release/bindkey-software
   ```
   *Note: System authentication windows (pkexec) will appear during volume creation, deletion, or formatting operations.*

## Code Architecture
* **`src/main.rs`:** Application entry point (`egui` framework), global state management (`BindKeyApp`), and automatic USB connection detection.
* **`src/event_handler.rs`:** The asynchronous core of the software. Receives interface actions (via `ApiMessage`), orchestrates network API calls (`reqwest`) and hardware commands, and updates the interface.
* **`src/usb_service.rs`:** Serial communication protocol with the BindKey.
* **`src/pages/`:** Contains the different interface views (Login, Home, Enrollment, Volumes). Physical disk management (Linux system calls) is concentrated in `volumes.rs`.
* **`src/protocol/`:** Definition of shared data structures (JSON API) and the hardware sharing protocol.

## Troubleshooting
* **The key is not detected:** Ensure your user has been added to the `dialout` group (`sudo usermod -aG dialout $USER` then restart the session).
* **Volume deletion failed:** If Linux refuses to delete the partition, ensure no file explorer windows (Nautilus, Thunar) are currently reading the disk's folder.
* **Interface freezing:** The software is designed asynchronously (`tokio`). If the interface freezes, check the error console: it is often a sign that `pkexec` is blocked in the background waiting for a system password.