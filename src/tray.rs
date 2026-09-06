use ksni::{Handle, MenuItem, Status, ToolTip, Tray};
use std::sync::{Arc, Mutex};

pub struct HandyXTray {
    pub is_recording: Arc<Mutex<bool>>,
    pub active_engine: Arc<Mutex<String>>,
    pub is_polish_enabled: Arc<Mutex<bool>>,
    pub active_mode: Arc<Mutex<String>>,
}

impl Tray for HandyXTray {
    fn id(&self) -> String {
        "handyx".to_string()
    }

    fn title(&self) -> String {
        "HandyX Voice Dictation".to_string()
    }

    fn icon_name(&self) -> String {
        let is_rec = *self.is_recording.lock().unwrap();
        if is_rec {
            "media-record".to_string()
        } else {
            "audio-input-microphone".to_string()
        }
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn tool_tip(&self) -> ToolTip {
        let is_rec = *self.is_recording.lock().unwrap();
        let engine = self.active_engine.lock().unwrap().clone();
        ToolTip {
            title: "HandyX".to_string(),
            description: if is_rec {
                format!("● Grabando... ({})", engine)
            } else {
                format!("Listo ({})", engine)
            },
            icon_name: self.icon_name(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = crate::ipc::send_command("TOGGLE");
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;

        let is_rec = *self.is_recording.lock().unwrap();
        let current_engine = self.active_engine.lock().unwrap().clone();
        let polish_on = *self.is_polish_enabled.lock().unwrap();
        let current_mode = self.active_mode.lock().unwrap().clone();

        vec![
            StandardItem {
                label: if is_rec {
                    "⏹ Detener Grabación y Pegar".into()
                } else {
                    "🎙 Iniciar Grabación (Dictar)".into()
                },
                activate: Box::new(|_| {
                    let _ = crate::ipc::send_command("TOGGLE");
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            SubMenu {
                label: format!("🧠 Modelo Activo: {}", current_engine),
                submenu: vec![
                    StandardItem {
                        label: if current_engine.contains("Groq") {
                            "● Groq Whisper Turbo (Auto-Detect) [Activo]".into()
                        } else {
                            "  Groq Whisper Turbo (Auto-Detect)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_ENGINE groq");
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: if current_engine.contains("English") || current_engine.contains("EN") {
                            "● Moonshine Base (English - Local) [Activo]".into()
                        } else {
                            "  Moonshine Base (English - Local)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_ENGINE moonshine-en");
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: if current_engine.contains("Español") || current_engine.contains("ES") {
                            "● Moonshine Base (Español - Local) [Activo]".into()
                        } else {
                            "  Moonshine Base (Español - Local)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_ENGINE moonshine-es");
                        }),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "✨ Pulido con IA (OpenRouter / Luna)".into(),
                checked: polish_on,
                activate: Box::new(|_| {
                    let _ = crate::ipc::send_command("TOGGLE_POLISH");
                }),
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: format!("⚙️ Modo: {}", current_mode),
                submenu: vec![
                    StandardItem {
                        label: if current_mode.to_lowercase().contains("hybrid") {
                            "● Híbrido (Toque=Toggle, Mantener=Push-to-Talk) [Activo]".into()
                        } else {
                            "  Híbrido (Toque=Toggle, Mantener=Push-to-Talk)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_MODE hybrid");
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: if current_mode.to_lowercase().contains("toggle") {
                            "● Toggle (Un toque inicio, otro toque fin) [Activo]".into()
                        } else {
                            "  Toggle (Un toque inicio, otro toque fin)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_MODE toggle");
                        }),
                        ..Default::default()
                    }
                    .into(),
                    StandardItem {
                        label: if current_mode.to_lowercase().contains("hold") {
                            "● Hold (Mantener presionado para hablar) [Activo]".into()
                        } else {
                            "  Hold (Mantener presionado para hablar)".into()
                        },
                        activate: Box::new(|_| {
                            let _ = crate::ipc::send_command("SET_MODE hold");
                        }),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "📁 Abrir Carpeta de Grabaciones".into(),
                activate: Box::new(|_| {
                    let _ = crate::ipc::send_command("OPEN_HISTORY");
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "⚙️ Abrir Archivo de Configuración".into(),
                activate: Box::new(|_| {
                    let _ = crate::ipc::send_command("OPEN_CONFIG");
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "❌ Salir de HandyX".into(),
                activate: Box::new(|_| {
                    let _ = crate::ipc::send_command("QUIT");
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub async fn spawn_tray(
    is_recording: Arc<Mutex<bool>>,
    active_engine: Arc<Mutex<String>>,
    is_polish_enabled: Arc<Mutex<bool>>,
    active_mode: Arc<Mutex<String>>,
) -> Result<Handle<HandyXTray>, ksni::Error> {
    use ksni::TrayMethods;
    let tray = HandyXTray {
        is_recording,
        active_engine,
        is_polish_enabled,
        active_mode,
    };
    tray.spawn().await
}
