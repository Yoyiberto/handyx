use std::process::Command;

const HANDYX_KEYBINDING_PATH: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/handyx/";
const HANDYX_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/handyx/";

pub fn register_gnome_shortcut(binding: &str) -> Result<(), Box<dyn std::error::Error>> {
    let handyx_bin = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("handyx"))
        .to_string_lossy()
        .to_string();

    let trigger_cmd = format!("{} toggle", handyx_bin);

    // 1. Get current custom keybinding list
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output()?;

    let current_list = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let mut paths: Vec<String> = if current_list.starts_with('[') && current_list.ends_with(']') {
        current_list[1..current_list.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    if !paths.contains(&HANDYX_KEYBINDING_PATH.to_string()) {
        paths.push(HANDYX_KEYBINDING_PATH.to_string());
    }

    let formatted_list = format!(
        "[{}]",
        paths
            .iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 2. Update custom-keybindings array
    Command::new("gsettings")
        .args([
            "set",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
            &formatted_list,
        ])
        .status()?;

    // 3. Set name, command, binding
    Command::new("gsettings")
        .args(["set", HANDYX_SCHEMA, "name", "'HandyX Toggle'"])
        .status()?;

    Command::new("gsettings")
        .args(["set", HANDYX_SCHEMA, "command", &format!("'{}'", trigger_cmd)])
        .status()?;

    Command::new("gsettings")
        .args(["set", HANDYX_SCHEMA, "binding", &format!("'{}'", binding)])
        .status()?;

    println!("Registered GNOME global shortcut: {} -> {}", binding, trigger_cmd);
    Ok(())
}

pub fn unregister_gnome_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output()?;

    let current_list = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let paths: Vec<String> = if current_list.starts_with('[') && current_list.ends_with(']') {
        current_list[1..current_list.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .filter(|s| !s.is_empty() && s != HANDYX_KEYBINDING_PATH)
            .collect()
    } else {
        Vec::new()
    };

    let formatted_list = format!(
        "[{}]",
        paths
            .iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Command::new("gsettings")
        .args([
            "set",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
            &formatted_list,
        ])
        .status()?;

    println!("Unregistered HandyX GNOME global shortcut");
    Ok(())
}
