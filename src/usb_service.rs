use serialport::SerialPort;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub fn send_text_command(
    port: &mut dyn SerialPort,
    command: &str,
) -> Result<HashMap<String, String>, String> {
    let _ = port.clear(serialport::ClearBuffer::All);

    let command_with_newline = format!("{}\n", command);

    if let Err(e) = port.write_all(command_with_newline.as_bytes()) {
        return Err(format!("Erreur écriture: {}", e));
    }
    let _ = port.flush();

    println!(">> USB ENVOI : {}", command);

    let mut results = HashMap::new();
    let start = Instant::now();
    let timeout = Duration::from_secs(3);

    let mut buffer: Vec<u8> = Vec::new();
    let mut byte_buf = [0u8; 1];

    while start.elapsed() < timeout {
        match port.read(&mut byte_buf) {
            Ok(1) => {
                let c = byte_buf[0];
                if c == b'\n' {
                    let line = String::from_utf8_lossy(&buffer).trim().to_string();
                    buffer.clear();

                    if line.is_empty() {
                        continue;
                    }
                    println!("<< USB REÇU : {}", line);

                    if line == "OK" {
                        return Ok(results);
                    } else if line.starts_with("ERR=") {
                        return Err(format!("Erreur Clé : {}", &line[4..]));
                    } else if let Some((key, value)) = line.split_once('=') {
                        results.insert(key.to_string(), value.to_string());
                    }
                } else {
                    buffer.push(c);
                }
            }
            Ok(_) => continue,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(format!("Erreur I/O: {}", e)),
        }
    }

    Err("Timeout: Pas de OK final".to_string())
}
