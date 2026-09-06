use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(format!("{}/handyx.sock", runtime_dir))
    } else {
        PathBuf::from("/tmp/handyx.sock")
    }
}

pub fn send_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    let sock_path = get_socket_path();
    let mut stream = UnixStream::connect(sock_path)
        .map_err(|e| format!("Could not connect to HandyX daemon (is 'handyx daemon' running?): {}", e))?;

    writeln!(stream, "{}", cmd)?;
    stream.flush()?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response.trim().to_string())
}

pub fn create_listener() -> Result<UnixListener, Box<dyn std::error::Error>> {
    let sock_path = get_socket_path();
    if sock_path.exists() {
        let _ = std::fs::remove_file(&sock_path);
    }
    let listener = UnixListener::bind(&sock_path)?;
    Ok(listener)
}
