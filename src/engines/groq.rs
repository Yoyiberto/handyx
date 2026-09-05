use super::TranscriptionEngine;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct GroqResponse {
    text: String,
}

pub struct GroqEngine {
    api_key: String,
    model: String,
    client: Client,
}

impl GroqEngine {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            api_key,
            model,
            client,
        }
    }
}

#[async_trait]
impl TranscriptionEngine for GroqEngine {
    fn name(&self) -> &str {
        "Groq Whisper Turbo"
    }

    async fn transcribe(
        &self,
        wav_bytes: &[u8],
        _raw_samples: &[f32],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.api_key.is_empty() {
            return Err("Groq API key is missing. Set it with 'hadyx set-key <KEY>' or GROQ_API_KEY environment variable.".into());
        }

        let part = Part::bytes(wav_bytes.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        // NOTE: No 'language' param passed -> Groq Whisper automatically detects English, Spanish, etc.
        let form = Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        let resp = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            return Err(format!("Groq API error {}: {}", status, err_text).into());
        }

        let result: GroqResponse = resp.json().await?;
        Ok(result.text.trim().to_string())
    }
}
