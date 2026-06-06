use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
fn main() -> anyhow::Result<()> {
    let port_name = "/dev/ttyACM0";
    println!("Opening port {}", port_name);
 
    let mut port = serialport::new(port_name, 115_200).timeout(Duration::from_secs(5)).open()?;
    println!("Port opened");
 
    let file_path="/home/ouiyam/TP/ESP32/test.txt";
    let mut file = File::open(file_path)?;
    println!("Sending file: {}", file_path);
 
    let mut buffer = [0u8; 512];
 
    loop{
        let n = file.read(&mut buffer)?;
        if n == 0{
            break;
        }
 
        port.write_all(&buffer[..n])?;
        println!("Sent {} bytes", n);
    }
 
    port.write_all(&[0x04])?;
    println!("Sent EOT");
 
    println!("Done");
 
    Ok(())
}