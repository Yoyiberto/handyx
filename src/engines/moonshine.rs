use super::TranscriptionEngine;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::path::PathBuf;

#[derive(Serialize)]
struct TranscribeRequest {
    audio_b64: String,
    language: String,
}

#[derive(Deserialize)]
struct TranscribeResponse {
    status: String,
    text: Option<String>,
    error: Option<String>,
}

pub struct MoonshineEngine {
    language: String, // "en" or "es"
    worker: Mutex<Option<MoonshineWorker>>,
}

struct MoonshineWorker {
    _process: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl MoonshineEngine {
    pub fn new(language: String) -> Self {
        Self {
            language,
            worker: Mutex::new(None),
        }
    }

    fn ensure_worker(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut guard = self.worker.lock().unwrap();
        if guard.is_some() {
            return Ok(());
        }

        let venv_python = PathBuf::from("/home/jmendez/ramses/tmp/hadyX/.venv/bin/python3");
        let python_bin = if venv_python.exists() {
            venv_python
        } else {
            PathBuf::from("python3")
        };

        let worker_script = PathBuf::from("/home/jmendez/ramses/tmp/hadyX/scripts/moonshine_worker.py");

        let mut child = Command::new(python_bin)
            .arg(worker_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn Moonshine worker: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to open child stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open child stdout")?;
        let mut reader = BufReader::new(stdout);

        let mut line = String::new();
        reader.read_line(&mut line)?;
        if !line.contains("READY") {
            return Err(format!("Unexpected worker init response: {}", line).into());
        }

        *guard = Some(MoonshineWorker {
            _process: child,
            stdin,
            reader,
        });

        Ok(())
    }
}

#[async_trait]
impl TranscriptionEngine for MoonshineEngine {
    fn name(&self) -> &str {
        if self.language == "es" {
            "Moonshine Base (Español)"
        } else {
            "Moonshine Base (English)"
        }
    }

    async fn transcribe(
        &self,
        wav_bytes: &[u8],
        _raw_samples: &[f32],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.ensure_worker()?;

        let mut guard = self.worker.lock().unwrap();
        let worker = guard.as_mut().ok_or("Moonshine worker unavailable")?;

        use base64::Engine;
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(wav_bytes);
        let req = TranscribeRequest {
            audio_b64,
            language: self.language.clone(),
        };
        let req_json = serde_json::to_string(&req)?;

        writeln!(worker.stdin, "{}", req_json)?;
        worker.stdin.flush()?;

        let mut response_line = String::new();
        worker.reader.read_line(&mut response_line)?;

        let resp: TranscribeResponse = serde_json::from_str(&response_line)?;
        if resp.status == "ok" {
            Ok(resp.text.unwrap_or_default().trim().to_string())
        } else {
            Err(resp.error.unwrap_or_else(|| "Unknown Moonshine error".into()).into())
        }
    }
}
