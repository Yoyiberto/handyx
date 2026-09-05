use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut copied = false;

    // Method 1: xclip (X11 / XWayland)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.flush();
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                copied = true;
            }
        }
    }

    // Method 2: xsel (X11 fallback)
    if !copied {
        if let Ok(mut child) = Command::new("xsel")
            .args(["-b", "-i"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    copied = true;
                }
            }
        }
    }

    // Method 3: wl-copy (Wayland native)
    if let Ok(mut child) = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.flush();
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                copied = true;
            }
        }
    }

    if copied {
        Ok(())
    } else {
        Err("Failed to copy to clipboard (tried xclip, xsel, wl-copy)".into())
    }
}

pub fn simulate_paste(delay_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Wait for clipboard synchronization and modifier key release
    thread::sleep(Duration::from_millis(delay_ms.max(50)));

    // Strategy 1: Enigo (native X11 / Wayland input simulation in Rust)
    if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
        let _ = enigo.key(Key::Control, Direction::Press);
        thread::sleep(Duration::from_millis(15));
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        thread::sleep(Duration::from_millis(15));
        let _ = enigo.key(Key::Control, Direction::Release);
        return Ok(());
    }

    // Strategy 2: wtype (Wayland virtual keyboard)
    if let Ok(status) = Command::new("wtype")
        .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if status.success() {
            return Ok(());
        }
    }

    // Strategy 3: xdotool (X11)
    if let Ok(status) = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if status.success() {
            return Ok(());
        }
    }

    // Strategy 4: ydotool
    if let Ok(status) = Command::new("ydotool")
        .args(["key", "29:1", "47:1", "47:0", "29:0"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if status.success() {
            return Ok(());
        }
    }

    Ok(())
}

pub fn paste_text(text: &str, delay_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    copy_to_clipboard(text)?;
    simulate_paste(delay_ms)?;
    Ok(())
}
