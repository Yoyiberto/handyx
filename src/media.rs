use log::{debug, info, warn};
use zbus::proxy;

#[proxy(
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    interface = "org.freedesktop.DBus"
)]
trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer2Player {
    fn pause(&self) -> zbus::Result<()>;
    fn play(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

pub struct MediaManager {
    connection: Option<zbus::Connection>,
}

impl MediaManager {
    pub async fn new() -> Self {
        match zbus::Connection::session().await {
            Ok(conn) => Self {
                connection: Some(conn),
            },
            Err(e) => {
                warn!("Could not connect to session D-Bus for media control: {}", e);
                Self { connection: None }
            }
        }
    }

    /// Finds all MPRIS media players currently in "Playing" state, pauses them,
    /// and returns the list of player service names that were paused.
    pub async fn pause_active_players(&self) -> Vec<String> {
        let conn = match &self.connection {
            Some(c) => c,
            None => return Vec::new(),
        };

        let dbus_proxy = match DBusProxy::new(conn).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to create DBusProxy: {}", e);
                return Vec::new();
            }
        };

        let names = match dbus_proxy.list_names().await {
            Ok(n) => n,
            Err(e) => {
                warn!("Failed to list D-Bus names: {}", e);
                return Vec::new();
            }
        };

        let mut paused_players = Vec::new();

        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                if let Ok(builder) = MediaPlayer2PlayerProxy::builder(conn)
                    .destination(name.as_str())
                {
                    if let Ok(player) = builder.build().await {
                        if let Ok(status) = player.playback_status().await {
                            if status.eq_ignore_ascii_case("Playing") {
                                debug!("Pausing active media player: {}", name);
                                if player.pause().await.is_ok() {
                                    info!("Paused media player: {}", name);
                                    paused_players.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        paused_players
    }

    /// Resumes playback only for the specified players that were previously paused by HandyX.
    pub async fn resume_players(&self, players: &[String]) {
        let conn = match &self.connection {
            Some(c) => c,
            None => return,
        };

        for name in players {
            if let Ok(builder) = MediaPlayer2PlayerProxy::builder(conn)
                .destination(name.as_str())
            {
                if let Ok(player) = builder.build().await {
                    debug!("Resuming media player: {}", name);
                    if let Err(e) = player.play().await {
                        warn!("Failed to resume media player {}: {}", name, e);
                    } else {
                        info!("Resumed media player: {}", name);
                    }
                }
            }
        }
    }
}
