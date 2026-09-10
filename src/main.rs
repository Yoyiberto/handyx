mod audio;
mod config;
mod engines;
mod history;
mod hud;
mod ipc;
mod media;
mod paste;
mod polish;
mod shortcut;
mod tray;

use audio::AudioRecorder;
use clap::{Parser, Subcommand};
use config::{AppConfig, EngineType, ShortcutMode};
use engines::groq::GroqEngine;
use engines::moonshine::MoonshineEngine;
use engines::TranscriptionEngine;
use history::HistoryManager;
use hud::{HudController, HudState};
use media::MediaManager;
use polish::OpenRouterPolisher;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "handyx")]
#[command(about = "Minimalist, ultra-fast speech-to-text with Top Bar Tray Icon, Moonshine Base, Groq Whisper Turbo, OpenRouter AI Polish & Media Auto-Pause", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the background daemon with top bar Tray icon and floating HUD
    Daemon {
        /// Run detached in the background without keeping the terminal open
        #[arg(short, long)]
        detach: bool,
    },
    /// Restart the background daemon in the background without keeping a terminal open
    Restart,
    /// Stop the running background daemon cleanly
    StopDaemon,
    /// Toggle recording (start/stop) - default for global shortcut
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
    /// Toggle auto-pausing media (Spotify, Chrome, YouTube, VLC) while recording ON/OFF
    ToggleMediaPause,
    /// Enable or disable auto-pausing media while recording ('on', 'off', 'true', 'false')
    SetMediaPause {
        /// 'on', 'off', 'true', or 'false'
        state: String,
    },
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
    /// Configure autostart on system boot/login
    Autostart {
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        disable: bool,
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
    media_manager: Arc<MediaManager>,
    paused_players: Arc<Mutex<Vec<String>>>,
    is_busy: Arc<AtomicBool>,
    recording_started_at: Arc<Mutex<Option<Instant>>>,
    tray_handle: Option<ksni::Handle<tray::HandyXTray>>,
    tray_recording: Arc<std::sync::Mutex<bool>>,
    tray_engine: Arc<std::sync::Mutex<String>>,
    tray_polish: Arc<std::sync::Mutex<bool>>,
    tray_mode: Arc<std::sync::Mutex<String>>,
    tray_media_pause: Arc<std::sync::Mutex<bool>>,
}

impl AppState {
    fn update_tray(&self) {
        if let Some(handle) = &self.tray_handle {
            let handle = handle.clone();
            tokio::spawn(async move {
                handle.update(|_| {}).await;
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Daemon { detach: false }) {
        Commands::Daemon { detach } => {
            if detach {
                start_detached_daemon()?;
            } else {
                run_daemon().await?;
            }
        }
        Commands::Restart => {
            let _ = std::process::Command::new("pkill").args(["-f", "handyx daemon"]).status();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let sock = ipc::get_socket_path();
            if sock.exists() {
                let _ = fs::remove_file(&sock);
            }
            start_detached_daemon()?;
        }
        Commands::StopDaemon => {
            if let Ok(res) = ipc::send_command("QUIT") {
                println!("HandyX daemon stopped: {}", res);
            } else {
                let _ = std::process::Command::new("pkill").args(["-f", "handyx daemon"]).status();
                println!("HandyX daemon stopped.");
            }
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
        Commands::ToggleMediaPause => {
            let res = ipc::send_command("TOGGLE_MEDIA_PAUSE")?;
            println!("{}", res);
        }
        Commands::SetMediaPause { state } => {
            let res = ipc::send_command(&format!("SET_MEDIA_PAUSE {}", state))?;
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
        Commands::Autostart { enable, disable } => {
            if enable {
                setup_autostart(true)?;
                println!("HandyX autostart enabled (will launch on login in background).");
            } else if disable {
                setup_autostart(false)?;
                println!("HandyX autostart disabled.");
            } else {
                let status = is_autostart_enabled();
                println!("HandyX autostart status: {}", if status { "ENABLED" } else { "DISABLED" });
                println!("Run with --enable or --disable to change.");
            }
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

fn start_detached_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let handyx_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/home/jmendez/.local/bin/handyx"));
    let bin_str = handyx_bin.to_string_lossy();

    let cmd_str = format!("setsid {} daemon > /tmp/handyx.log 2>&1 &", bin_str);
    std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd_str)
        .spawn()?
        .wait()?;

    println!("HandyX daemon started in background.");
    println!("Icon is active in the top bar. You can safely close this terminal.");
    Ok(())
}

fn autostart_desktop_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(format!("{}/.config/autostart", home));
    let _ = fs::create_dir_all(&dir);
    dir.join("handyx.desktop")
}

fn is_autostart_enabled() -> bool {
    autostart_desktop_path().exists()
}

fn setup_autostart(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = autostart_desktop_path();
    if enable {
        let handyx_bin = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("/home/jmendez/.local/bin/handyx"))
            .to_string_lossy()
            .to_string();

        let desktop_content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=HandyX\n\
            Comment=HandyX Voice Dictation Daemon\n\
            Exec={} daemon\n\
            Icon=audio-input-microphone\n\
            Terminal=false\n\
            Categories=Utility;Audio;\n\
            X-GNOME-Autostart-enabled=true\n",
            handyx_bin
        );
        fs::write(path, desktop_content)?;

        // Also setup systemd user service
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let sys_dir = PathBuf::from(format!("{}/.config/systemd/user", home));
        let _ = fs::create_dir_all(&sys_dir);
        let service_content = format!(
            "[Unit]\n\
            Description=HandyX Voice Dictation Daemon\n\
            After=graphical-session.target\n\n\
            [Service]\n\
            Type=simple\n\
            ExecStart={} daemon\n\
            Restart=on-failure\n\
            RestartSec=2\n\
            Environment=PATH={}/.local/bin:/usr/local/bin:/usr/bin:/bin\n\n\
            [Install]\n\
            WantedBy=default.target\n",
            handyx_bin, home
        );
        let _ = fs::write(sys_dir.join("handyx.service"), service_content);
    } else {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Starting HandyX Daemon ===");
    let config = Arc::new(Mutex::new(AppConfig::load()));
    let cfg_guard = config.lock().await;

    println!("Active Engine: {}", cfg_guard.engine);
    println!("Shortcut Mode: {}", cfg_guard.shortcut_mode);
    println!("Global Shortcut: {}", cfg_guard.shortcut);
    println!("AI Polish: {} (Model: {})", if cfg_guard.enable_ai_polish { "ENABLED" } else { "DISABLED" }, cfg_guard.openrouter_model);
    println!("History Saving: {}", if cfg_guard.save_history { "ENABLED" } else { "DISABLED" });
    println!("Config Path: {:?}", AppConfig::config_path());

    let _ = shortcut::register_gnome_shortcut(&cfg_guard.shortcut);
    let _ = setup_autostart(true);

    let groq_engine = Arc::new(GroqEngine::new(
        cfg_guard.groq_api_key.clone(),
        cfg_guard.groq_model.clone(),
    ));
    let moonshine_en_engine = Arc::new(MoonshineEngine::new("en".to_string()));
    let moonshine_es_engine = Arc::new(MoonshineEngine::new("es".to_string()));
    let history_manager = Arc::new(HistoryManager::new(cfg_guard.history_dir.as_deref()));
    let media_manager = Arc::new(MediaManager::new().await);
    let paused_players = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::new(Mutex::new(AudioRecorder::new(cfg_guard.audio_sample_rate)));

    // Spawn Tray Icon
    let tray_rec = Arc::new(std::sync::Mutex::new(false));
    let tray_eng = Arc::new(std::sync::Mutex::new(cfg_guard.engine.to_string()));
    let tray_pol = Arc::new(std::sync::Mutex::new(cfg_guard.enable_ai_polish));
    let tray_mod = Arc::new(std::sync::Mutex::new(cfg_guard.shortcut_mode.to_string()));
    let tray_media = Arc::new(std::sync::Mutex::new(cfg_guard.pause_media_on_record));

    let tray_handle = match tray::spawn_tray(
        tray_rec.clone(),
        tray_eng.clone(),
        tray_pol.clone(),
        tray_mod.clone(),
        tray_media.clone(),
    ).await {
        Ok(handle) => {
            println!("System tray icon registered successfully in top bar.");
            Some(handle)
        }
        Err(e) => {
            eprintln!("Notice: System tray icon could not be registered ({}). Continuing in background.", e);
            None
        }
    };

    // Spawn Floating HUD
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
        media_manager,
        paused_players,
        is_busy: Arc::new(AtomicBool::new(false)),
        recording_started_at: Arc::new(Mutex::new(None)),
        tray_handle,
        tray_recording: tray_rec,
        tray_engine: tray_eng,
        tray_polish: tray_pol,
        tray_mode: tray_mod,
        tray_media_pause: tray_media,
    });

    let listener = ipc::create_listener()?;
    println!("IPC Socket ready at {:?}", ipc::get_socket_path());
    println!("HandyX is running in background with Tray Icon in top bar!");

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
                            if cmd == "QUIT" {
                                println!("HandyX daemon shutting down by user request.");
                                std::process::exit(0);
                            }
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

            *state.tray_engine.lock().unwrap() = cfg.engine.to_string();
            state.update_tray();

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

        *state.tray_mode.lock().unwrap() = cfg.shortcut_mode.to_string();
        state.update_tray();

        return format!("Shortcut mode updated to {}", cfg.shortcut_mode);
    }

    if cmd == "RELOAD_CONFIG" {
        let mut cfg = state.config.lock().await;
        *cfg = AppConfig::load();
        *state.tray_engine.lock().unwrap() = cfg.engine.to_string();
        *state.tray_polish.lock().unwrap() = cfg.enable_ai_polish;
        *state.tray_mode.lock().unwrap() = cfg.shortcut_mode.to_string();
        *state.tray_media_pause.lock().unwrap() = cfg.pause_media_on_record;
        state.update_tray();
        return "Config reloaded".to_string();
    }

    if cmd == "TOGGLE_POLISH" {
        let mut cfg = state.config.lock().await;
        cfg.enable_ai_polish = !cfg.enable_ai_polish;
        let _ = cfg.save();

        *state.tray_polish.lock().unwrap() = cfg.enable_ai_polish;
        state.update_tray();

        return format!("AI Polish: {}", if cfg.enable_ai_polish { "ENABLED" } else { "DISABLED" });
    }

    if cmd == "TOGGLE_MEDIA_PAUSE" {
        let mut cfg = state.config.lock().await;
        cfg.pause_media_on_record = !cfg.pause_media_on_record;
        let _ = cfg.save();

        *state.tray_media_pause.lock().unwrap() = cfg.pause_media_on_record;
        state.update_tray();

        return format!("Media Auto-Pause: {}", if cfg.pause_media_on_record { "ENABLED" } else { "DISABLED" });
    }

    if cmd.starts_with("SET_MEDIA_PAUSE ") {
        let arg = cmd.trim_start_matches("SET_MEDIA_PAUSE ").trim().to_lowercase();
        let val = match arg.as_str() {
            "true" | "1" | "on" | "enable" | "enabled" | "yes" | "si" => true,
            "false" | "0" | "off" | "disable" | "disabled" | "no" => false,
            _ => true,
        };
        let mut cfg = state.config.lock().await;
        cfg.pause_media_on_record = val;
        let _ = cfg.save();

        *state.tray_media_pause.lock().unwrap() = cfg.pause_media_on_record;
        state.update_tray();

        return format!("Media Auto-Pause set to: {}", if val { "ENABLED" } else { "DISABLED" });
    }

    if cmd == "OPEN_HISTORY" {
        let path = state.history_manager.get_recordings_dir();
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        return "Opening history folder".to_string();
    }

    if cmd == "OPEN_CONFIG" {
        let path = AppConfig::config_path();
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        return "Opening config file".to_string();
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

            *state.tray_engine.lock().unwrap() = cfg.engine.to_string();
            state.update_tray();

            format!("Switched to: {}", cfg.engine)
        }
        "STATUS" => {
            let is_rec = {
                let rec = state.recorder.lock().await;
                rec.is_recording()
            };
            let cfg = state.config.lock().await;
            format!(
                "Active Model: {} | Mode: {} | Polish: {} ({}) | Media Pause: {} | Recording: {}",
                cfg.engine,
                cfg.shortcut_mode,
                if cfg.enable_ai_polish { "ON" } else { "OFF" },
                cfg.openrouter_model,
                if cfg.pause_media_on_record { "ON" } else { "OFF" },
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

            *state.tray_recording.lock().unwrap() = true;
            state.update_tray();

            // Auto-pause playing media if enabled
            let pause_media = {
                let cfg = state.config.lock().await;
                cfg.pause_media_on_record
            };
            if pause_media {
                let media_mgr = state.media_manager.clone();
                let paused_list = state.paused_players.clone();
                tokio::spawn(async move {
                    let paused = media_mgr.pause_active_players().await;
                    let mut list = paused_list.lock().await;
                    *list = paused;
                });
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

    *state.tray_recording.lock().unwrap() = false;
    state.update_tray();

    // Auto-resume media that was paused by HandyX immediately
    {
        let media_mgr = state.media_manager.clone();
        let paused_list = state.paused_players.clone();
        tokio::spawn(async move {
            let players = {
                let mut list = paused_list.lock().await;
                std::mem::take(&mut *list)
            };
            if !players.is_empty() {
                media_mgr.resume_players(&players).await;
            }
        });
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
