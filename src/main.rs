mod audio;
mod config;
mod engines;
mod history;
mod hud;
mod ipc;
mod paste;
mod polish;
mod shortcut;

use audio::AudioRecorder;
use clap::{Parser, Subcommand};
use config::{AppConfig, EngineType, ShortcutMode};
use engines::groq::GroqEngine;
use engines::moonshine::MoonshineEngine;
use engines::TranscriptionEngine;
use history::HistoryManager;
use hud::{HudController, HudState};
use polish::OpenRouterPolisher;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "hadyx")]
#[command(about = "Minimalist, ultra-fast speech-to-text with Moonshine Base, Groq Whisper Turbo & OpenRouter AI Polish", long_about = None)]
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
    /// Toggle AI polishing ON/OFF
    TogglePolish,
    /// Set OpenRouter API key for AI polishing
    SetOpenrouterKey {
        /// OpenRouter API key
        key: String,
    },
    /// Set OpenRouter AI polish model (default: 'openai/gpt-5.6-luna')
    SetPolishModel {
        /// Model name (e.g. 'openai/gpt-5.6-luna')
        model: String,
    },
    /// View recent transcription history
    History {
        /// Number of entries to display
        #[arg(default_value_t = 10)]
        count: usize,
    },
    /// Open the recordings and history directory in file manager
    OpenHistory,
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
    history_manager: Arc<HistoryManager>,
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
        Commands::TogglePolish => {
            let res = ipc::send_command("TOGGLE_POLISH")?;
            println!("{}", res);
        }
        Commands::SetOpenrouterKey { key } => {
            let mut cfg = AppConfig::load();
            cfg.openrouter_api_key = key;
            cfg.save()?;
            let _ = ipc::send_command("RELOAD_CONFIG");
            println!("OpenRouter API key saved successfully in config.");
        }
        Commands::SetPolishModel { model } => {
            let mut cfg = AppConfig::load();
            cfg.openrouter_model = model.clone();
            cfg.save()?;
            let _ = ipc::send_command("RELOAD_CONFIG");
            println!("AI Polish model updated to: '{}'", model);
        }
        Commands::History { count } => {
            let cfg = AppConfig::load();
            let history_mgr = HistoryManager::new(cfg.history_dir.as_deref());
            let entries = history_mgr.list_recent(count);
            if entries.is_empty() {
                println!("No history entries found in {:?}", history_mgr.get_base_dir());
            } else {
                println!("=== Recent Transcriptions ({}) ===", entries.len());
                for e in entries {
                    println!("--------------------------------------------------");
                    println!("ID:        {}", e.id);
                    println!("Time:      {}", e.timestamp);
                    println!("Engine:    {}", e.engine);
                    if let Some(audio) = &e.audio_file {
                        println!("Audio:     {}", audio);
                    }
                    println!("Raw:       {}", e.raw_transcript);
                    if let Some(pol) = &e.polished_transcript {
                        println!("✨ Polish:  {}", pol);
                    }
                }
                println!("--------------------------------------------------");
                println!("Full recordings & JSON stored in: {:?}", history_mgr.get_recordings_dir());
            }
        }
        Commands::OpenHistory => {
            let cfg = AppConfig::load();
            let history_mgr = HistoryManager::new(cfg.history_dir.as_deref());
            let path = history_mgr.get_recordings_dir();
            println!("Opening {:?} in file manager...", path);
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
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
            let _ = ipc::send_command("RELOAD_CONFIG");
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
    println!("AI Polish: {} (Model: {})", if cfg_guard.enable_ai_polish { "ENABLED" } else { "DISABLED" }, cfg_guard.openrouter_model);
    println!("History Saving: {}", if cfg_guard.save_history { "ENABLED" } else { "DISABLED" });
    println!("Config Path: {:?}", AppConfig::config_path());

    let _ = shortcut::register_gnome_shortcut(&cfg_guard.shortcut);

    let groq_engine = Arc::new(GroqEngine::new(
        cfg_guard.groq_api_key.clone(),
        cfg_guard.groq_model.clone(),
    ));
    let moonshine_en_engine = Arc::new(MoonshineEngine::new("en".to_string()));
    let moonshine_es_engine = Arc::new(MoonshineEngine::new("es".to_string()));
    let history_manager = Arc::new(HistoryManager::new(cfg_guard.history_dir.as_deref()));
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
        history_manager,
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

    if cmd == "RELOAD_CONFIG" {
        let mut cfg = state.config.lock().await;
        *cfg = AppConfig::load();
        return "Config reloaded".to_string();
    }

    if cmd == "TOGGLE_POLISH" {
        let mut cfg = state.config.lock().await;
        cfg.enable_ai_polish = !cfg.enable_ai_polish;
        let _ = cfg.save();
        return format!("AI Polish: {}", if cfg.enable_ai_polish { "ENABLED" } else { "DISABLED" });
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
                "Active Model: {} | Mode: {} | Polish: {} ({}) | Recording: {}",
                cfg.engine,
                cfg.shortcut_mode,
                if cfg.enable_ai_polish { "ON" } else { "OFF" },
                cfg.openrouter_model,
                is_rec
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

    let (engine_type, engine_badge, paste_delay, auto_paste, enable_polish, openrouter_key, openrouter_model, save_hist, save_aud) = {
        let cfg = state.config.lock().await;
        (
            cfg.engine.clone(),
            cfg.engine.badge_label().to_string(),
            cfg.paste_delay_ms,
            cfg.auto_paste,
            cfg.enable_ai_polish,
            cfg.openrouter_api_key.clone(),
            cfg.openrouter_model.clone(),
            cfg.save_history,
            cfg.save_audio,
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

    match trans_res {
        Ok(raw_text) => {
            if raw_text.trim().is_empty() {
                state.is_busy.store(false, Ordering::SeqCst);
                state.hud.set_state(HudState::Hidden);
                return "EMPTY_TRANSCRIPTION".to_string();
            }

            println!("Raw Transcript: '{}'", raw_text);

            let mut final_text = raw_text.clone();
            let mut polished_text_opt: Option<String> = None;

            // AI Polishing step via OpenRouter
            if enable_polish && !openrouter_key.trim().is_empty() {
                let model_short_name = if openrouter_model.contains("luna") {
                    "Luna".to_string()
                } else if openrouter_model.contains('/') {
                    openrouter_model.split('/').last().unwrap_or(&openrouter_model).to_string()
                } else {
                    openrouter_model.clone()
                };

                state.hud.set_state(HudState::Polishing {
                    model_name: model_short_name,
                });

                let polisher = OpenRouterPolisher::new(openrouter_key, openrouter_model);
                match polisher.polish(&raw_text).await {
                    Ok(polished) => {
                        if !polished.trim().is_empty() {
                            println!("✨ Polished: '{}'", polished);
                            final_text = polished.clone();
                            polished_text_opt = Some(polished);
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: AI Polish failed, using raw transcript: {}", e);
                    }
                }
            }

            // Save history & recordings
            if save_hist {
                let engine_name = format!("{:?}", engine_type);
                let audio_slice = if save_aud { Some(&wav_bytes[..]) } else { None };
                let _ = state.history_manager.save_entry(
                    audio_slice,
                    &engine_name,
                    &raw_text,
                    polished_text_opt.as_deref(),
                );
            }

            state.is_busy.store(false, Ordering::SeqCst);

            state.hud.set_state(HudState::Success {
                text: final_text.clone(),
            });

            if auto_paste {
                if let Err(e) = paste::paste_text(&final_text, paste_delay) {
                    eprintln!("Paste error: {}", e);
                }
            }

            let hud = state.hud.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(800)).await;
                hud.set_state(HudState::Hidden);
            });

            format!("PASTED: {}", final_text)
        }
        Err(e) => {
            state.is_busy.store(false, Ordering::SeqCst);
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
