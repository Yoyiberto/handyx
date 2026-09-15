use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::debug;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut copied = false;

    // Method 1: xclip (X11 / XWayland) - Copy to CLIPBOARD
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

    // Also copy to PRIMARY selection for terminal Shift+Insert / Middle-click
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "primary"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.flush();
        }
        let _ = child.wait();
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
        if let Ok(mut child) = Command::new("xsel")
            .args(["-p", "-i"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
            }
            let _ = child.wait();
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
    if let Ok(mut child) = Command::new("wl-copy")
        .arg("--primary")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.flush();
        }
        let _ = child.wait();
    }

    if copied {
        Ok(())
    } else {
        Err("Failed to copy to clipboard (tried xclip, xsel, wl-copy)".into())
    }
}

/// Detects if the current active focused window is a Terminal (GNOME Terminal, Alacritty, Kitty, etc.)
pub fn is_terminal_window() -> bool {
    // Method 1: Check X11 _NET_ACTIVE_WINDOW WM_CLASS
    if let Ok(out) = Command::new("sh")
        .arg("-c")
        .arg("xprop -root _NET_ACTIVE_WINDOW 2>/dev/null | awk '{print $NF}' | xargs -I {} xprop -id {} WM_CLASS 2>/dev/null")
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            let terms = [
                "terminal", "gnome-terminal", "alacritty", "kitty", "konsole",
                "xterm", "tilix", "terminator", "wezterm", "urxvt", "foot",
                "guake", "tilda", "pty", "console"
            ];
            for t in terms {
                if s.contains(t) {
                    debug!("Detected active terminal window: {}", s);
                    return true;
                }
            }
        }
    }
    false
}

pub fn simulate_paste(delay_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Wait for clipboard synchronization and modifier key release
    thread::sleep(Duration::from_millis(delay_ms.max(50)));

    let is_terminal = is_terminal_window();

    // Strategy 1: Enigo (native X11 / Wayland input simulation in Rust)
    if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
        let _ = enigo.key(Key::Control, Direction::Press);
        if is_terminal {
            let _ = enigo.key(Key::Shift, Direction::Press);
        }
        thread::sleep(Duration::from_millis(15));
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        thread::sleep(Duration::from_millis(15));
        if is_terminal {
            let _ = enigo.key(Key::Shift, Direction::Release);
        }
        let _ = enigo.key(Key::Control, Direction::Release);
        return Ok(());
    }

    // Strategy 2: wtype (Wayland virtual keyboard)
    let wtype_args = if is_terminal {
        vec!["-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl"]
    } else {
        vec!["-M", "ctrl", "-k", "v", "-m", "ctrl"]
    };
    if let Ok(status) = Command::new("wtype")
        .args(&wtype_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if status.success() {
            return Ok(());
        }
    }

    // Strategy 3: xdotool (X11)
    let xdotool_key = if is_terminal { "ctrl+shift+v" } else { "ctrl+v" };
    if let Ok(status) = Command::new("xdotool")
        .args(["key", "--clearmodifiers", xdotool_key])
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

/// Reads the currently selected/highlighted text from the X11/Wayland primary selection buffer (e.g. from a double-click)
pub fn get_primary_selection() -> String {
    // 1. Try xclip (primary selection)
    if let Ok(out) = Command::new("xclip")
        .args(["-o", "-selection", "primary"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }

    // 2. Try xsel (primary selection)
    if let Ok(out) = Command::new("xsel").args(["-p", "-o"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }

    // 3. Try wl-paste (Wayland primary)
    if let Ok(out) = Command::new("wl-paste").arg("--primary").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }

    String::new()
}
