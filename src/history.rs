use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub engine: String,
    pub raw_transcript: String,
    pub polished_transcript: Option<String>,
    pub was_polished: bool,
    pub audio_file: Option<String>,
}

pub struct HistoryManager {
    base_dir: PathBuf,
}

impl HistoryManager {
    pub fn new(custom_dir: Option<&str>) -> Self {
        let base_dir = if let Some(dir) = custom_dir {
            PathBuf::from(dir)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let dir = PathBuf::from(format!("{}/.local/share/handyx", home));
            
            // Migrate old hadyx data if present
            let old_dir = PathBuf::from(format!("{}/.local/share/hadyx", home));
            if old_dir.exists() && !dir.exists() {
                let _ = fs::rename(&old_dir, &dir);
            }
            
            dir
        };

        let recordings_dir = base_dir.join("recordings");
        let _ = fs::create_dir_all(&recordings_dir);

        Self { base_dir }
    }

    pub fn get_base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn get_recordings_dir(&self) -> PathBuf {
        self.base_dir.join("recordings")
    }

    pub fn save_entry(
        &self,
        wav_bytes: Option<&[u8]>,
        engine: &str,
        raw_transcript: &str,
        polished_transcript: Option<&str>,
    ) -> Result<HistoryEntry, Box<dyn std::error::Error + Send + Sync>> {
        let now = Local::now();
        let id = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let timestamp = now.to_rfc3339();

        let recordings_dir = self.get_recordings_dir();
        let _ = fs::create_dir_all(&recordings_dir);

        let audio_filename = if let Some(bytes) = wav_bytes {
            let file_name = format!("{}.wav", id);
            let wav_path = recordings_dir.join(&file_name);
            fs::write(&wav_path, bytes)?;
            Some(file_name)
        } else {
            None
        };

        let entry = HistoryEntry {
            id: id.clone(),
            timestamp,
            engine: engine.to_string(),
            raw_transcript: raw_transcript.to_string(),
            polished_transcript: polished_transcript.map(|s| s.to_string()),
            was_polished: polished_transcript.is_some(),
            audio_file: audio_filename,
        };

        let json_path = recordings_dir.join(format!("{}.json", id));
        let json_content = serde_json::to_string_pretty(&entry)?;
        fs::write(&json_path, json_content)?;

        let jsonl_path = self.base_dir.join("history.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)?;

        let line = serde_json::to_string(&entry)?;
        writeln!(file, "{}", line)?;

        Ok(entry)
    }

    pub fn list_recent(&self, count: usize) -> Vec<HistoryEntry> {
        let jsonl_path = self.base_dir.join("history.jsonl");
        if !jsonl_path.exists() {
            return Vec::new();
        }

        if let Ok(content) = fs::read_to_string(jsonl_path) {
            let mut entries: Vec<HistoryEntry> = content
                .lines()
                .filter_map(|line| serde_json::from_str::<HistoryEntry>(line).ok())
                .collect();
            entries.reverse();
            entries.truncate(count);
            entries
        } else {
            Vec::new()
        }
    }
}
