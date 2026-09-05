use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineType {
    #[serde(rename = "groq-turbo")]
    GroqTurbo,
    #[serde(rename = "moonshine-en")]
    MoonshineEn,
    #[serde(rename = "moonshine-es")]
    MoonshineEs,
}

impl Default for EngineType {
    fn default() -> Self {
        EngineType::GroqTurbo
    }
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineType::GroqTurbo => write!(f, "Groq Whisper Turbo (Auto)"),
            EngineType::MoonshineEn => write!(f, "Moonshine Base (English)"),
            EngineType::MoonshineEs => write!(f, "Moonshine Base (Español)"),
        }
    }
}

impl EngineType {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "groq" | "groq-turbo" | "whisper" | "groq_turbo" => Some(EngineType::GroqTurbo),
            "moonshine-en" | "moonshine_en" | "moonshine-english" | "en" => Some(EngineType::MoonshineEn),
            "moonshine-es" | "moonshine_es" | "moonshine-spanish" | "es" => Some(EngineType::MoonshineEs),
            _ => None,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            EngineType::GroqTurbo => EngineType::MoonshineEn,
            EngineType::MoonshineEn => EngineType::MoonshineEs,
            EngineType::MoonshineEs => EngineType::GroqTurbo,
        }
    }

    pub fn badge_label(&self) -> &'static str {
        match self {
            EngineType::GroqTurbo => "Groq Turbo (Auto)",
            EngineType::MoonshineEn => "Moonshine (EN)",
            EngineType::MoonshineEs => "Moonshine (ES)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShortcutMode {
    #[serde(rename = "toggle")]
    Toggle,
    #[serde(rename = "hold")]
    Hold,
    #[serde(rename = "hybrid")]
    Hybrid,
}

impl Default for ShortcutMode {
    fn default() -> Self {
        ShortcutMode::Hybrid
    }
}

impl std::fmt::Display for ShortcutMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortcutMode::Toggle => write!(f, "Toggle (Press to start, press to stop)"),
            ShortcutMode::Hold => write!(f, "Hold / Push-to-Talk (Hold to speak, release to paste)"),
            ShortcutMode::Hybrid => write!(f, "Hybrid (Tap for toggle, hold for push-to-talk)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub engine: EngineType,
    pub groq_api_key: String,
    pub groq_model: String,
    pub shortcut: String,
    pub shortcut_mode: ShortcutMode,
    pub paste_method: String,
    pub audio_sample_rate: u32,
    pub auto_paste: bool,
    pub paste_delay_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        let env_groq = std::env::var("GROQ_API_KEY").unwrap_or_default();

        Self {
            engine: EngineType::GroqTurbo,
            groq_api_key: env_groq,
            groq_model: "whisper-large-v3-turbo".to_string(),
            shortcut: "<Control><Shift>space".to_string(),
            shortcut_mode: ShortcutMode::Hybrid,
            paste_method: "auto".to_string(),
            audio_sample_rate: 16000,
            auto_paste: true,
            paste_delay_ms: 60,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "hadyx", "hadyx") {
            let dir = proj_dirs.config_dir();
            let _ = fs::create_dir_all(dir);
            dir.join("config.toml")
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let dir = PathBuf::from(format!("{}/.config/hadyx", home));
            let _ = fs::create_dir_all(&dir);
            dir.join("config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = toml::from_str::<AppConfig>(&content) {
                    if cfg.groq_api_key.is_empty() {
                        if let Ok(env_key) = std::env::var("GROQ_API_KEY") {
                            cfg.groq_api_key = env_key;
                        }
                    }
                    return cfg;
                }
            }
        }

        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }
}
