pub mod groq;
pub mod moonshine;

use async_trait::async_trait;

#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    fn name(&self) -> &str;
    async fn transcribe(
        &self,
        wav_bytes: &[u8],
        raw_samples: &[f32],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}
