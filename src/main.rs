mod audio;
mod config;
mod engines;
mod hud;
mod ipc;
mod paste;
mod shortcut;

use audio::AudioRecorder;
use clap::{Parser, Subcommand};
use config::{AppConfig, EngineType, ShortcutMode};
use engines::groq::GroqEngine;
use engines::moonshine::MoonshineEngine;
use engines::TranscriptionEngine;
use hud::{HudController, HudState};
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "hadyx")]
#[command(about = "Minimalist, ultra-fast speech-to-text with Moonshine Base & Groq Whisper Turbo", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the background daemon with floating HUD
    Daemon,
    /// Toggle recording (start/stop) - default for GNOME shortcut
    Toggle,
    /// Key-down trigger for Push-to-Talk (Hold mode)
    Start,
    /// Key-up trigger for Push-to-Talk (Hold mode)
    Stop,
    /// Cycle between the 3 models: Groq Whisper Turbo -> Moonshine English -> Moonshine Spanish
    SwitchEngine,
    /// Set active model directly ('groq', 'moonshine-en', 'moonshine-es')
    SetEngine {
        /// Engine name ('groq', 'moonshine-en', 'moonshine-es')
        engine: String,
    },
    /// Set shortcut mode ('toggle', 'hold', or 'hybrid')
    SetMode {
        /// Mode ('toggle', 'hold', 'hybrid')
        mode: String,
    },
    /// Show current status and active configuration
    Status,
    /// Set and register GNOME global shortcut (e.g. '<Control><Shift>space' or '<Control>space')
    SetShortcut {
        /// Keybinding expression (e.g. '<Control><Shift>space' or '<Control>space')
        binding: String,
    },
    /// Set Groq API key
    SetKey {
        /// Groq API key
        key: String,
    },
    /// Test clipboard and paste simulation
    PasteTest {
        /// Text to paste
        text: String,
    },
}

struct AppState {
    config: Arc<Mutex<AppConfig>>,
    recorder: Arc<Mutex<AudioRecorder>>,
    hud: Arc<HudController>,
    groq_engine: Arc<GroqEngine>,
    moonshine_en_engine: Arc<MoonshineEngine>,
    moonshine_es_engine: Arc<MoonshineEngine>,
    is_busy: Arc<AtomicBool>,
    recording_started_at: Arc<Mutex<Option<Instant>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Daemon) {
        Commands::Daemon => {
            run_daemon().await?;
        }
        Commands::Toggle => {
            let res = ipc::send_command("TOGGLE")?;
            println!("{}", res);
        }
        Commands::Start => {
            let res = ipc::send_command("START")?;
            println!("{}", res);
        }
        Commands::Stop => {
            let res = ipc::send_command("STOP")?;
            println!("{}", res);
        }
        Commands::SwitchEngine => {
            let res = ipc::send_command("SWITCH_ENGINE")?;
            println!("{}", res);
        }
        Commands::SetEngine { engine } => {
            if let Some(engine_type) = EngineType::from_str_loose(&engine) {
                let mut cfg = AppConfig::load();
                cfg.engine = engine_type;
                cfg.save()?;
                let _ = ipc::send_command(&format!("SET_ENGINE {:?}", cfg.engine));
                println!("Active engine set to: {}", cfg.engine);
            } else {
                eprintln!("Invalid engine. Choose from: 'groq' (Groq Whisper Turbo Auto), 'moonshine-en' (Moonshine Base English), 'moonshine-es' (Moonshine Base Español)");
            }
        }
        Commands::SetMode { mode } => {
            let mut cfg = AppConfig::load();
            match mode.to_lowercase().as_str() {
                "toggle" => cfg.shortcut_mode = ShortcutMode::Toggle,
                "hold" => cfg.shortcut_mode = ShortcutMode::Hold,
                "hybrid" => cfg.shortcut_mode = ShortcutMode::Hybrid,
                _ => {
                    eprintln!("Invalid mode. Choose from: 'toggle', 'hold', 'hybrid'");
                    return Ok(());
                }
            }
            cfg.save()?;
            let _ = ipc::send_command(&format!("SET_MODE {}", mode));
            println!("Shortcut mode set to '{}'", cfg.shortcut_mode);
        }
        Commands::Status => {
            let res = ipc::send_command("STATUS")?;
            println!("{}", res);
        }
        Commands::SetShortcut { binding } => {
            let mut cfg = AppConfig::load();
            cfg.shortcut = binding.clone();
            cfg.save()?;
            shortcut::register_gnome_shortcut(&binding)?;
            println!("Global shortcut updated to '{}'", binding);
        }
        Commands::SetKey { key } => {
            let mut cfg = AppConfig::load();
            cfg.groq_api_key = key;
            cfg.save()?;
            println!("Groq API key saved successfully in config.");
        }
        Commands::PasteTest { text } => {
            println!("Testing paste with text: '{}'...", text);
            paste::paste_text(&text, 60)?;
            println!("Paste completed.");
        }
    }

    Ok(())
}

async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Starting HadyX Daemon ===");
    let config = Arc::new(Mutex::new(AppConfig::load()));
    let cfg_guard = config.lock().await;

    println!("Active Engine: {}", cfg_guard.engine);
    println!("Shortcut Mode: {}", cfg_guard.shortcut_mode);
    println!("Global Shortcut: {}", cfg_guard.shortcut);
    println!("Config Path: {:?}", AppConfig::config_path());

    let _ = shortcut::register_gnome_shortcut(&cfg_guard.shortcut);

    let groq_engine = Arc::new(GroqEngine::new(
        cfg_guard.groq_api_key.clone(),
        cfg_guard.groq_model.clone(),
    ));
    let moonshine_en_engine = Arc::new(MoonshineEngine::new("en".to_string()));
    let moonshine_es_engine = Arc::new(MoonshineEngine::new("es".to_string()));
    let recorder = Arc::new(Mutex::new(AudioRecorder::new(cfg_guard.audio_sample_rate)));
    let hud = Arc::new(HudController::new());
    hud.start_ui_thread();

    drop(cfg_guard);

    let app_state = Arc::new(AppState {
        config: config.clone(),
        recorder: recorder.clone(),
        hud: hud.clone(),
        groq_engine,
        moonshine_en_engine,
        moonshine_es_engine,
        is_busy: Arc::new(AtomicBool::new(false)),
        recording_started_at: Arc::new(Mutex::new(None)),
    });

    let listener = ipc::create_listener()?;
    println!("IPC Socket ready at {:?}", ipc::get_socket_path());
    println!("HadyX is listening for global triggers...");

    // Level updater task
    let state_for_level = app_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(40)).await;
            let is_rec = {
                let rec = state_for_level.recorder.lock().await;
                rec.is_recording()
            };
            if is_rec {
                let lvl = {
                    let rec = state_for_level.recorder.lock().await;
                    rec.get_current_level()
                };
                let engine_badge = {
                    let cfg = state_for_level.config.lock().await;
                    cfg.engine.badge_label().to_string()
                };
                state_for_level.hud.set_state(HudState::Recording {
                    engine_name: engine_badge,
                    level: lvl,
                });
            }
        }
    });

    // Accept IPC connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = app_state.clone();
                tokio::spawn(async move {
                    if let Ok(mut cloned_stream) = stream.try_clone() {
                        let mut reader = BufReader::new(stream);
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() {
                            let cmd = line.trim();
                            let response = handle_ipc_command(cmd, &state).await;
                            let _ = writeln!(cloned_stream, "{}", response);
                            let _ = cloned_stream.flush();
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("IPC connection error: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_ipc_command(cmd: &str, state: &Arc<AppState>) -> String {
    if cmd.starts_with("SET_ENGINE ") {
        let engine_str = cmd.trim_start_matches("SET_ENGINE ").trim();
        if let Some(engine_type) = EngineType::from_str_loose(engine_str) {
            let mut cfg = state.config.lock().await;
            cfg.engine = engine_type;
            let _ = cfg.save();
            return format!("Active engine set to: {}", cfg.engine);
        }
    }

    if cmd.starts_with("SET_MODE ") {
        let mode_str = cmd.trim_start_matches("SET_MODE ").trim();
        let mut cfg = state.config.lock().await;
        match mode_str {
            "toggle" => cfg.shortcut_mode = ShortcutMode::Toggle,
            "hold" => cfg.shortcut_mode = ShortcutMode::Hold,
            "hybrid" => cfg.shortcut_mode = ShortcutMode::Hybrid,
            _ => {}
        }
        let _ = cfg.save();
        return format!("Shortcut mode updated to {}", cfg.shortcut_mode);
    }

    match cmd {
        "TOGGLE" => {
            let is_rec = {
                let rec = state.recorder.lock().await;
                rec.is_recording()
            };
            if is_rec {
                stop_and_transcribe(state).await
            } else {
                start_recording(state).await
            }
        }
        "START" => start_recording(state).await,
        "STOP" => stop_and_transcribe(state).await,
        "SWITCH_ENGINE" => {
            let mut cfg = state.config.lock().await;
            cfg.engine = cfg.engine.next();
            let _ = cfg.save();
            format!("Switched to: {}", cfg.engine)
        }
        "STATUS" => {
            let is_rec = {
                let rec = state.recorder.lock().await;
                rec.is_recording()
            };
            let cfg = state.config.lock().await;
            format!(
                "Active Model: {} | Mode: {} | Recording: {}",
                cfg.engine, cfg.shortcut_mode, is_rec
            )
        }
        _ => "UNKNOWN_COMMAND".to_string(),
    }
}

async fn start_recording(state: &Arc<AppState>) -> String {
    if state.is_busy.load(Ordering::SeqCst) {
        return "BUSY".to_string();
    }

    let start_result = {
        let mut rec = state.recorder.lock().await;
        rec.start()
    };

    match start_result {
        Ok(_) => {
            {
                let mut start_time = state.recording_started_at.lock().await;
                *start_time = Some(Instant::now());
            }

            let engine_badge = {
                let cfg = state.config.lock().await;
                cfg.engine.badge_label().to_string()
            };
            state.hud.set_state(HudState::Recording {
                engine_name: engine_badge,
                level: 0.0,
            });
            "RECORDING_STARTED".to_string()
        }
        Err(err_msg) => {
            state.hud.set_state(HudState::Error {
                message: format!("Mic error: {}", err_msg),
            });
            let hud = state.hud.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(2000)).await;
                hud.set_state(HudState::Hidden);
            });
            format!("ERROR: {}", err_msg)
        }
    }
}

async fn stop_and_transcribe(state: &Arc<AppState>) -> String {
    state.is_busy.store(true, Ordering::SeqCst);

    {
        let mut start_time = state.recording_started_at.lock().await;
        *start_time = None;
    }

    let (wav_bytes, raw_samples) = {
        let mut rec = state.recorder.lock().await;
        let samples = rec.get_raw_samples();
        match rec.stop() {
            Ok(bytes) => (bytes, samples),
            Err(e) => {
                state.is_busy.store(false, Ordering::SeqCst);
                state.hud.set_state(HudState::Hidden);
                return format!("ERROR_STOP: {}", e);
            }
        }
    };

    let (engine_type, engine_badge, paste_delay, auto_paste) = {
        let cfg = state.config.lock().await;
        (
            cfg.engine.clone(),
            cfg.engine.badge_label().to_string(),
            cfg.paste_delay_ms,
            cfg.auto_paste,
        )
    };

    state.hud.set_state(HudState::Transcribing {
        engine_name: engine_badge,
    });

    let state_clone = state.clone();
    let trans_res = match engine_type {
        EngineType::GroqTurbo => {
            state_clone
                .groq_engine
                .transcribe(&wav_bytes, &raw_samples)
                .await
        }
        EngineType::MoonshineEn => {
            state_clone
                .moonshine_en_engine
                .transcribe(&wav_bytes, &raw_samples)
                .await
        }
        EngineType::MoonshineEs => {
            state_clone
                .moonshine_es_engine
                .transcribe(&wav_bytes, &raw_samples)
                .await
        }
    };

    state.is_busy.store(false, Ordering::SeqCst);

    match trans_res {
        Ok(text) => {
            if text.trim().is_empty() {
                state.hud.set_state(HudState::Hidden);
                return "EMPTY_TRANSCRIPTION".to_string();
            }

            println!("Transcribed: '{}'", text);
            state.hud.set_state(HudState::Success {
                text: text.clone(),
            });

            if auto_paste {
                if let Err(e) = paste::paste_text(&text, paste_delay) {
                    eprintln!("Paste error: {}", e);
                }
            }

            let hud = state.hud.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(800)).await;
                hud.set_state(HudState::Hidden);
            });

            format!("PASTED: {}", text)
        }
        Err(e) => {
            eprintln!("Transcription error: {}", e);
            state.hud.set_state(HudState::Error {
                message: format!("{}", e),
            });
            let hud = state.hud.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(2500)).await;
                hud.set_state(HudState::Hidden);
            });
            format!("ERROR: {}", e)
        }
    }
}
