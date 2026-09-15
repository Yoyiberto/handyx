use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VocabItem {
    pub original: String,
    pub corrected: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VocabularyData {
    pub items: Vec<VocabItem>,
}

pub struct VocabularyManager {
    path: PathBuf,
}

impl VocabularyManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(format!("{}/.config/handyx", home));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("vocabulary.json");
        Self { path }
    }

    pub fn load_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Ok(content) = fs::read_to_string(&self.path) {
            if let Ok(data) = serde_json::from_str::<VocabularyData>(&content) {
                for item in data.items {
                    map.insert(item.original.to_lowercase(), item.corrected);
                }
            }
        }
        map
    }

    pub fn list_items(&self) -> Vec<VocabItem> {
        if let Ok(content) = fs::read_to_string(&self.path) {
            if let Ok(data) = serde_json::from_str::<VocabularyData>(&content) {
                return data.items;
            }
        }
        Vec::new()
    }

    pub fn learn(&self, original: &str, corrected: &str) -> Result<(), Box<dyn std::error::Error>> {
        let orig = original.trim();
        let corr = corrected.trim();
        if orig.is_empty() || corr.is_empty() {
            return Ok(());
        }

        let mut data = if let Ok(content) = fs::read_to_string(&self.path) {
            serde_json::from_str::<VocabularyData>(&content).unwrap_or_default()
        } else {
            VocabularyData::default()
        };

        data.items.retain(|item| !item.original.eq_ignore_ascii_case(orig));

        data.items.push(VocabItem {
            original: orig.to_string(),
            corrected: corr.to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
        });

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let empty = VocabularyData::default();
        let json = serde_json::to_string_pretty(&empty)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn apply(&self, text: &str) -> String {
        let map = self.load_map();
        if map.is_empty() {
            return text.to_string();
        }

        let mut result = text.to_string();
        for (orig, corr) in map {
            let mut search_idx = 0;
            while let Some(pos) = result[search_idx..].to_lowercase().find(&orig) {
                let start = search_idx + pos;
                let end = start + orig.len();

                let is_start_boundary = start == 0 || !result.as_bytes()[start - 1].is_ascii_alphanumeric();
                let is_end_boundary = end == result.len() || !result.as_bytes()[end].is_ascii_alphanumeric();

                if is_start_boundary && is_end_boundary {
                    result.replace_range(start..end, &corr);
                    search_idx = start + corr.len();
                } else {
                    search_idx = end;
                }

                if search_idx >= result.len() {
                    break;
                }
            }
        }
        result
    }
}
