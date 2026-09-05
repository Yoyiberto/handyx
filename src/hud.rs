use gdk::prelude::*;
use gtk::prelude::*;
use gtk::{Box as GtkBox, CssProvider, Label, Orientation, StyleContext, WindowPosition, WindowType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum HudState {
    Hidden,
    Recording { engine_name: String, level: f32 },
    Transcribing { engine_name: String },
    Polishing { model_name: String },
    Success { text: String },
    Error { message: String },
}

pub struct HudController {
    state: Arc<Mutex<HudState>>,
    app_running: Arc<AtomicBool>,
}

impl HudController {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HudState::Hidden)),
            app_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_state(&self, state: HudState) {
        if let Ok(mut s) = self.state.lock() {
            *s = state;
        }
    }

    pub fn start_ui_thread(&self) {
        if self.app_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let state_clone = self.state.clone();
        std::thread::spawn(move || {
            if gtk::init().is_err() {
                eprintln!("Failed to initialize GTK for HUD overlay");
                return;
            }

            let window = gtk::Window::new(WindowType::Toplevel);
            window.set_title("HadyX HUD");
            window.set_decorated(false);
            window.set_keep_above(true);
            window.set_skip_taskbar_hint(true);
            window.set_skip_pager_hint(true);
            window.set_resizable(false);
            window.set_position(WindowPosition::Center);

            // CRITICAL: Prevent HUD from stealing keyboard focus!
            window.set_accept_focus(false);
            window.set_focus_on_map(false);
            window.set_type_hint(gdk::WindowTypeHint::Notification);

            // Enable transparency
            if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&window) {
                if let Some(visual) = screen.rgba_visual() {
                    window.set_visual(Some(&visual));
                }
            }
            window.set_app_paintable(true);

            // Custom CSS for modern dark pill style
            let provider = CssProvider::new();
            let css_data = "
                window {
                    background-color: transparent;
                }
                .hud-pill {
                    background-color: rgba(18, 18, 24, 0.94);
                    border: 1px solid rgba(255, 255, 255, 0.15);
                    border-radius: 24px;
                    padding: 8px 20px;
                    box-shadow: 0px 8px 28px rgba(0, 0, 0, 0.7);
                }
                .hud-icon-rec {
                    color: #ff4757;
                    font-size: 16px;
                    font-weight: bold;
                }
                .hud-icon-trans {
                    color: #ffa502;
                    font-size: 16px;
                    font-weight: bold;
                }
                .hud-icon-polish {
                    color: #e056fd;
                    font-size: 16px;
                    font-weight: bold;
                }
                .hud-icon-success {
                    color: #2ed573;
                    font-size: 16px;
                    font-weight: bold;
                }
                .hud-icon-err {
                    color: #ff6b81;
                    font-size: 16px;
                    font-weight: bold;
                }
                .hud-text {
                    color: #f1f2f6;
                    font-family: 'Inter', 'Ubuntu', 'Sans', sans-serif;
                    font-size: 14px;
                    font-weight: 500;
                    margin-left: 8px;
                }
                .hud-badge {
                    color: #70a1ff;
                    background-color: rgba(112, 161, 255, 0.15);
                    border-radius: 12px;
                    padding: 2px 8px;
                    font-size: 11px;
                    font-weight: 600;
                    margin-left: 10px;
                }
                .hud-badge-polish {
                    color: #e056fd;
                    background-color: rgba(224, 86, 253, 0.18);
                    border-radius: 12px;
                    padding: 2px 8px;
                    font-size: 11px;
                    font-weight: 600;
                    margin-left: 10px;
                }
            ";
            let _ = provider.load_from_data(css_data.as_bytes());
            if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&window) {
                StyleContext::add_provider_for_screen(
                    &screen,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }

            let container = GtkBox::new(Orientation::Horizontal, 6);
            container.style_context().add_class("hud-pill");

            let icon_label = Label::new(Some("●"));
            icon_label.style_context().add_class("hud-icon-rec");

            let status_label = Label::new(Some("Escuchando..."));
            status_label.style_context().add_class("hud-text");

            let badge_label = Label::new(Some("Groq Turbo"));
            badge_label.style_context().add_class("hud-badge");

            container.pack_start(&icon_label, false, false, 0);
            container.pack_start(&status_label, false, false, 0);
            container.pack_start(&badge_label, false, false, 0);

            window.add(&container);

            // Move to bottom center of primary monitor
            if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&window) {
                let display = screen.display();
                if let Some(monitor) = display.primary_monitor() {
                    let geom = monitor.geometry();
                    let x = geom.x() + (geom.width() / 2) - 150;
                    let y = geom.y() + geom.height() - 95;
                    window.move_(x, y);
                }
            }

            // Periodic UI updater (40ms interval)
            let win_weak = window.downgrade();
            let icon_weak = icon_label.downgrade();
            let status_weak = status_label.downgrade();
            let badge_weak = badge_label.downgrade();
            let state_ref = state_clone;

            glib::timeout_add_local(Duration::from_millis(40), move || {
                let current_state = state_ref.lock().unwrap().clone();
                if let (Some(win), Some(icon), Some(status), Some(badge)) = (
                    win_weak.upgrade(),
                    icon_weak.upgrade(),
                    status_weak.upgrade(),
                    badge_weak.upgrade(),
                ) {
                    match current_state {
                        HudState::Hidden => {
                            if win.is_visible() {
                                win.hide();
                            }
                        }
                        HudState::Recording { engine_name, level } => {
                            let bars = if level > 0.6 {
                                "||||"
                            } else if level > 0.35 {
                                "|||"
                            } else if level > 0.15 {
                                "||"
                            } else {
                                "|"
                            };
                            icon.set_text(&format!("● {}", bars));
                            icon.style_context().remove_class("hud-icon-trans");
                            icon.style_context().remove_class("hud-icon-polish");
                            icon.style_context().remove_class("hud-icon-success");
                            icon.style_context().remove_class("hud-icon-err");
                            icon.style_context().add_class("hud-icon-rec");

                            badge.style_context().remove_class("hud-badge-polish");
                            badge.style_context().add_class("hud-badge");

                            status.set_text("Escuchando...");
                            badge.set_text(&engine_name);

                            if !win.is_visible() {
                                win.show_all();
                            }
                        }
                        HudState::Transcribing { engine_name } => {
                            icon.set_text("⚡");
                            icon.style_context().remove_class("hud-icon-rec");
                            icon.style_context().remove_class("hud-icon-polish");
                            icon.style_context().remove_class("hud-icon-success");
                            icon.style_context().remove_class("hud-icon-err");
                            icon.style_context().add_class("hud-icon-trans");

                            badge.style_context().remove_class("hud-badge-polish");
                            badge.style_context().add_class("hud-badge");

                            status.set_text("Transcribiendo...");
                            badge.set_text(&engine_name);

                            if !win.is_visible() {
                                win.show_all();
                            }
                        }
                        HudState::Polishing { model_name } => {
                            icon.set_text("✨");
                            icon.style_context().remove_class("hud-icon-rec");
                            icon.style_context().remove_class("hud-icon-trans");
                            icon.style_context().remove_class("hud-icon-success");
                            icon.style_context().remove_class("hud-icon-err");
                            icon.style_context().add_class("hud-icon-polish");

                            badge.style_context().remove_class("hud-badge");
                            badge.style_context().add_class("hud-badge-polish");

                            status.set_text("Puliendo texto...");
                            badge.set_text(&model_name);

                            if !win.is_visible() {
                                win.show_all();
                            }
                        }
                        HudState::Success { text } => {
                            icon.set_text("✓");
                            icon.style_context().remove_class("hud-icon-rec");
                            icon.style_context().remove_class("hud-icon-trans");
                            icon.style_context().remove_class("hud-icon-polish");
                            icon.style_context().remove_class("hud-icon-err");
                            icon.style_context().add_class("hud-icon-success");

                            badge.style_context().remove_class("hud-badge-polish");
                            badge.style_context().add_class("hud-badge");

                            let preview = if text.len() > 28 {
                                format!("{}...", &text[..28])
                            } else if text.is_empty() {
                                "Texto pegado".to_string()
                            } else {
                                text
                            };
                            status.set_text(&preview);
                            badge.set_text("OK");

                            if !win.is_visible() {
                                win.show_all();
                            }
                        }
                        HudState::Error { message } => {
                            icon.set_text("✗");
                            icon.style_context().remove_class("hud-icon-rec");
                            icon.style_context().remove_class("hud-icon-trans");
                            icon.style_context().remove_class("hud-icon-polish");
                            icon.style_context().remove_class("hud-icon-success");
                            icon.style_context().add_class("hud-icon-err");

                            badge.style_context().remove_class("hud-badge-polish");
                            badge.style_context().add_class("hud-badge");

                            status.set_text(&message);
                            badge.set_text("Error");

                            if !win.is_visible() {
                                win.show_all();
                            }
                        }
                    }
                }
                glib::ControlFlow::Continue
            });

            gtk::main();
        });
    }
}
