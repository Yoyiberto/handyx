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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Secrets {
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub openrouter_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub engine: EngineType,
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default = "default_groq_model")]
    pub groq_model: String,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default)]
    pub shortcut_mode: ShortcutMode,
    #[serde(default = "default_paste_method")]
    pub paste_method: String,
    #[serde(default = "default_sample_rate")]
    pub audio_sample_rate: u32,
    #[serde(default = "default_true")]
    pub auto_paste: bool,
    #[serde(default = "default_paste_delay")]
    pub paste_delay_ms: u64,

    // AI Polish settings (OpenRouter)
    #[serde(default = "default_true")]
    pub enable_ai_polish: bool,
    #[serde(default)]
    pub openrouter_api_key: String,
    #[serde(default = "default_openrouter_model")]
    pub openrouter_model: String,

    // History and recording storage
    #[serde(default = "default_true")]
    pub save_history: bool,
    #[serde(default = "default_true")]
    pub save_audio: bool,
    #[serde(default)]
    pub history_dir: Option<String>,
}

fn default_groq_model() -> String { "whisper-large-v3-turbo".to_string() }
fn default_shortcut() -> String { "<Control><Shift>space".to_string() }
fn default_paste_method() -> String { "auto".to_string() }
fn default_sample_rate() -> u32 { 16000 }
fn default_paste_delay() -> u64 { 60 }
fn default_openrouter_model() -> String { "openai/gpt-5.6-luna".to_string() }
fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            engine: EngineType::GroqTurbo,
            groq_api_key: String::new(),
            groq_model: default_groq_model(),
            shortcut: default_shortcut(),
            shortcut_mode: ShortcutMode::Hybrid,
            paste_method: default_paste_method(),
            audio_sample_rate: default_sample_rate(),
            auto_paste: true,
            paste_delay_ms: default_paste_delay(),

            enable_ai_polish: true,
            openrouter_api_key: String::new(),
            openrouter_model: default_openrouter_model(),

            save_history: true,
            save_audio: true,
            history_dir: None,
        }
    }
}

impl AppConfig {
    pub fn base_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(format!("{}/.config/handyx", home));
        let _ = fs::create_dir_all(&dir);

        // Migrate old hadyx config if present
        let old_dir = PathBuf::from(format!("{}/.config/hadyx", home));
        if old_dir.exists() && !dir.join("config.toml").exists() {
            if let Ok(old_content) = fs::read_to_string(old_dir.join("config.toml")) {
                let _ = fs::write(dir.join("config.toml"), old_content);
            }
            if let Ok(old_secrets) = fs::read_to_string(old_dir.join("secrets.toml")) {
                let _ = fs::write(dir.join("secrets.toml"), old_secrets);
            }
        }

        dir
    }

    pub fn config_path() -> PathBuf {
        Self::base_dir().join("config.toml")
    }

    pub fn secrets_path() -> PathBuf {
        Self::base_dir().join("secrets.toml")
    }

    pub fn load_secrets() -> Secrets {
        let path = Self::secrets_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(sec) = toml::from_str::<Secrets>(&content) {
                    return sec;
                }
            }
        }
        Secrets::default()
    }

    pub fn save_secrets(secrets: &Secrets) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::secrets_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(secrets)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut cfg = if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                toml::from_str::<AppConfig>(&content).unwrap_or_default()
            } else {
                AppConfig::default()
            }
        } else {
            AppConfig::default()
        };

        let secrets = Self::load_secrets();

        if cfg.groq_api_key.trim().is_empty() {
            if !secrets.groq_api_key.trim().is_empty() {
                cfg.groq_api_key = secrets.groq_api_key.clone();
            } else if let Ok(env_key) = std::env::var("GROQ_API_KEY") {
                cfg.groq_api_key = env_key;
            }
        }

        if cfg.openrouter_api_key.trim().is_empty() {
            if !secrets.openrouter_api_key.trim().is_empty() {
                cfg.openrouter_api_key = secrets.openrouter_api_key.clone();
            } else if let Ok(env_key) = std::env::var("OPENROUTER_API_KEY") {
                cfg.openrouter_api_key = env_key;
            }
        }

        cfg
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut secrets = Self::load_secrets();
        if !self.groq_api_key.trim().is_empty() {
            secrets.groq_api_key = self.groq_api_key.clone();
        }
        if !self.openrouter_api_key.trim().is_empty() {
            secrets.openrouter_api_key = self.openrouter_api_key.clone();
        }
        let _ = Self::save_secrets(&secrets);

        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }
}
