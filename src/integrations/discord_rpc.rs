//! Discord Rich Presence over the local Discord desktop-client IPC pipe.
//!
//! Privacy: only a generic activity is shared by default. The project name is sent only
//! when the user enables it, and then only the file stem (never the path). No pixels,
//! paths, user names or document contents leave Kagero.
//!
//! The connection logic ([`PresenceClient`]) is independent of the transport so it can be
//! tested without Discord. A background thread owns the real IPC connection; the UI only
//! sends the desired presence when it changes.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::config::DiscordSettings;

/// Minimum time between activity updates (Discord allows about 5 per 20 seconds).
pub const MIN_UPDATE_INTERVAL: Duration = Duration::from_secs(4);
const FIRST_RETRY: Duration = Duration::from_secs(5);
const MAX_RETRY: Duration = Duration::from_secs(120);
const MAX_PROJECT_NAME_CHARS: usize = 48;

/// What Kagero is doing right now, as far as Discord is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Start screen, no document open.
    Home,
    /// A single-frame document.
    StaticDocument,
    /// A document with several frames.
    AnimationDocument,
    /// Animation preview is playing.
    Previewing,
}

/// Activity text and asset keys sent to Discord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityData {
    pub details: String,
    pub state: String,
    /// Unix seconds the "elapsed" timer counts from.
    pub start: Option<i64>,
    pub large_image: &'static str,
    pub large_text: &'static str,
    pub small_image: &'static str,
    pub small_text: &'static str,
}

/// The presence Kagero wants Discord to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Desired {
    /// Show nothing (integration disabled).
    Clear,
    Show(ActivityData),
}

/// Presence images. Discord accepts HTTPS URLs as well as Developer Portal asset
/// keys, so the images are served straight from the repository
/// (`docs/images/discord/`) and need no upload (see
/// docs/developers/services.md).
pub mod assets {
    pub const LOGO: &str =
        "https://raw.githubusercontent.com/senzukisu/kagero/main/docs/images/discord/kagero-logo.png";
    pub const PIXEL_ART: &str =
        "https://raw.githubusercontent.com/senzukisu/kagero/main/docs/images/discord/pixel-art.png";
    pub const ANIMATION: &str =
        "https://raw.githubusercontent.com/senzukisu/kagero/main/docs/images/discord/animation.png";
    pub const WORKSHOP: &str =
        "https://raw.githubusercontent.com/senzukisu/kagero/main/docs/images/discord/workshop.png";
}

/// Keeps only a short, printable file stem.
fn safe_project_name(name: &str) -> Option<String> {
    let stem = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim_end_matches(".kgr")
        .trim();
    let cleaned: String = stem
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_PROJECT_NAME_CHARS)
        .collect();
    (!cleaned.trim().is_empty()).then(|| cleaned.trim().to_owned())
}

/// Builds the presence for a mode, applying the user's privacy settings.
pub fn build_presence(
    mode: AppMode,
    project_name: Option<&str>,
    session_start_unix: i64,
    settings: &DiscordSettings,
) -> Desired {
    if !settings.enabled {
        return Desired::Clear;
    }
    let (details, state, small_image, small_text) = match mode {
        AppMode::Home => (
            "Browsing the Workshop",
            "Ready to Create",
            assets::WORKSHOP,
            "Exploring Kagero",
        ),
        AppMode::StaticDocument => (
            "Creating Pixel Art",
            "Editing a Sprite",
            assets::PIXEL_ART,
            "Pixel art",
        ),
        AppMode::AnimationDocument => (
            "Animating Sprites",
            "Animating",
            assets::ANIMATION,
            "Animation",
        ),
        AppMode::Previewing => (
            "Previewing Animation",
            "Animating",
            assets::ANIMATION,
            "Animation preview",
        ),
    };
    let (details, mut state) = if settings.show_mode {
        (details.to_owned(), state.to_owned())
    } else {
        ("Using Kagero".to_owned(), "Pixel art editor".to_owned())
    };
    if settings.show_project_name && mode != AppMode::Home {
        if let Some(name) = project_name.and_then(safe_project_name) {
            state = format!("Project: {name}");
        }
    }
    Desired::Show(ActivityData {
        details,
        state,
        start: settings.show_timer.then_some(session_start_unix),
        large_image: assets::LOGO,
        large_text: "Kagero by Onibi Lab",
        small_image,
        small_text,
    })
}

// ---------------------------------------------------------------- connection logic

/// Something that can talk to Discord.
pub trait RpcTransport: Send {
    fn connect(&mut self) -> Result<(), String>;
    fn set_activity(&mut self, activity: &ActivityData) -> Result<(), String>;
    fn clear_activity(&mut self) -> Result<(), String>;
    fn close(&mut self);
}

/// Connection state shown in Settings → Integrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// No Discord application ID is configured in this build.
    NotConfigured,
    /// Turned off in settings, or disconnected by the user.
    Disabled,
    Connecting,
    Connected,
    /// Discord is not running or refused; `retry_in` is set when a retry is scheduled.
    NotConnected {
        reason: String,
        retry_in: Option<Duration>,
    },
}

/// Flags from the settings that affect the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub enabled: bool,
    pub auto_reconnect: bool,
    pub clear_on_disable: bool,
}

impl From<&DiscordSettings> for Flags {
    fn from(s: &DiscordSettings) -> Self {
        Self {
            enabled: s.enabled,
            auto_reconnect: s.auto_reconnect,
            clear_on_disable: s.clear_on_disable,
        }
    }
}

/// Connection state machine. Each [`PresenceClient::step`] does at most one action.
pub struct PresenceClient<T: RpcTransport> {
    transport: T,
    connected: bool,
    paused: bool,
    sent: Option<Desired>,
    last_send: Option<Instant>,
    next_attempt: Option<Instant>,
    retry_delay: Duration,
    state: ConnectionState,
}

impl<T: RpcTransport> PresenceClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            connected: false,
            paused: false,
            sent: None,
            last_send: None,
            next_attempt: None,
            retry_delay: FIRST_RETRY,
            state: ConnectionState::Connecting,
        }
    }

    pub fn state(&self) -> &ConnectionState {
        &self.state
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// "Connect / Retry": try again now.
    pub fn retry(&mut self, now: Instant) {
        self.paused = false;
        self.retry_delay = FIRST_RETRY;
        self.next_attempt = Some(now);
    }

    /// "Disconnect": clear the presence and stay disconnected until `retry`.
    pub fn disconnect(&mut self, clear: bool) {
        self.paused = true;
        self.drop_connection(clear);
        self.state = ConnectionState::Disabled;
    }

    fn drop_connection(&mut self, clear: bool) {
        if self.connected {
            if clear {
                let _ = self.transport.clear_activity();
            }
            self.transport.close();
        }
        self.connected = false;
        self.sent = None;
    }

    fn schedule_retry(&mut self, now: Instant, reason: String, auto: bool) {
        let retry_in = auto.then_some(self.retry_delay);
        self.next_attempt = retry_in.map(|d| now + d);
        if auto {
            self.retry_delay = (self.retry_delay * 2).min(MAX_RETRY);
        }
        self.state = ConnectionState::NotConnected { reason, retry_in };
    }

    pub fn step(&mut self, now: Instant, desired: &Desired, flags: Flags) {
        if !flags.enabled || self.paused {
            if self.connected {
                self.drop_connection(flags.clear_on_disable);
            }
            self.state = ConnectionState::Disabled;
            return;
        }
        if !self.connected {
            if self.next_attempt.is_some_and(|t| now < t) {
                return;
            }
            if self.next_attempt.is_none()
                && matches!(
                    self.state,
                    ConnectionState::NotConnected { retry_in: None, .. }
                )
            {
                // Waiting for a manual retry.
                return;
            }
            self.state = ConnectionState::Connecting;
            match self.transport.connect() {
                Ok(()) => {
                    self.connected = true;
                    self.retry_delay = FIRST_RETRY;
                    self.next_attempt = None;
                    self.sent = None;
                    self.last_send = None;
                    self.state = ConnectionState::Connected;
                }
                Err(e) => {
                    self.schedule_retry(now, short_reason(&e), flags.auto_reconnect);
                }
            }
            return;
        }
        if self.sent.as_ref() == Some(desired) {
            return;
        }
        if self
            .last_send
            .is_some_and(|t| now.duration_since(t) < MIN_UPDATE_INTERVAL)
        {
            return;
        }
        let result = match desired {
            Desired::Clear => self.transport.clear_activity(),
            Desired::Show(activity) => self.transport.set_activity(activity),
        };
        match result {
            Ok(()) => {
                self.sent = Some(desired.clone());
                self.last_send = Some(now);
            }
            Err(e) => {
                tracing::info!("Discord connection lost: {e}");
                self.transport.close();
                self.connected = false;
                self.sent = None;
                self.schedule_retry(now, short_reason(&e), flags.auto_reconnect);
            }
        }
    }

    /// Clears the activity and closes the connection (app shutdown).
    pub fn shutdown(&mut self) {
        self.drop_connection(true);
    }
}

fn short_reason(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("pipe") || lower.contains("not found") || lower.contains("connect") {
        "Discord desktop app is not running".to_owned()
    } else {
        error.chars().take(120).collect()
    }
}

// ---------------------------------------------------------------- real transport

/// The real IPC connection to the Discord desktop client.
pub struct IpcTransport {
    client: discord_rich_presence::DiscordIpcClient,
}

impl IpcTransport {
    pub fn new(application_id: &str) -> Self {
        Self {
            client: discord_rich_presence::DiscordIpcClient::new(application_id),
        }
    }
}

impl RpcTransport for IpcTransport {
    fn connect(&mut self) -> Result<(), String> {
        use discord_rich_presence::DiscordIpc;
        self.client.connect().map_err(|e| e.to_string())
    }

    fn set_activity(&mut self, a: &ActivityData) -> Result<(), String> {
        use discord_rich_presence::{activity, DiscordIpc};
        let assets = activity::Assets::new()
            .large_image(a.large_image)
            .large_text(a.large_text)
            .small_image(a.small_image)
            .small_text(a.small_text);
        let mut payload = activity::Activity::new()
            .details(a.details.as_str())
            .state(a.state.as_str())
            .assets(assets);
        if let Some(start) = a.start {
            payload = payload.timestamps(activity::Timestamps::new().start(start));
        }
        self.client.set_activity(payload).map_err(|e| e.to_string())
    }

    fn clear_activity(&mut self) -> Result<(), String> {
        use discord_rich_presence::DiscordIpc;
        self.client.clear_activity().map_err(|e| e.to_string())
    }

    fn close(&mut self) {
        use discord_rich_presence::DiscordIpc;
        let _ = self.client.close();
    }
}

// ---------------------------------------------------------------- background service

enum Message {
    Update(Desired, Flags),
    Retry,
    Disconnect(bool),
    Shutdown,
}

/// Owns the background thread that talks to Discord.
pub struct DiscordPresence {
    sender: Option<Sender<Message>>,
    state: Arc<Mutex<ConnectionState>>,
    worker: Option<JoinHandle<()>>,
    last: Option<(Desired, Flags)>,
}

impl DiscordPresence {
    /// Starts the service, or a no-op service when no application ID is configured.
    pub fn start(application_id: &str) -> Self {
        let state = Arc::new(Mutex::new(ConnectionState::NotConfigured));
        if application_id.trim().is_empty() {
            return Self {
                sender: None,
                state,
                worker: None,
                last: None,
            };
        }
        let (tx, rx) = mpsc::channel::<Message>();
        let shared = state.clone();
        let id = application_id.to_owned();
        let worker = std::thread::Builder::new()
            .name("kagero-discord".into())
            .spawn(move || {
                let mut client = PresenceClient::new(IpcTransport::new(&id));
                let mut desired = Desired::Clear;
                let mut flags = Flags {
                    enabled: false,
                    auto_reconnect: true,
                    clear_on_disable: true,
                };
                loop {
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(Message::Update(d, f)) => {
                            desired = d;
                            flags = f;
                        }
                        Ok(Message::Retry) => client.retry(Instant::now()),
                        Ok(Message::Disconnect(clear)) => client.disconnect(clear),
                        Ok(Message::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                            client.shutdown();
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    client.step(Instant::now(), &desired, flags);
                    if let Ok(mut s) = shared.lock() {
                        *s = client.state().clone();
                    }
                }
            });
        match worker {
            Ok(handle) => Self {
                sender: Some(tx),
                state,
                worker: Some(handle),
                last: None,
            },
            Err(e) => {
                tracing::warn!("Discord presence unavailable: {e}");
                Self {
                    sender: None,
                    state,
                    worker: None,
                    last: None,
                }
            }
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
            .lock()
            .map(|s| s.clone())
            .unwrap_or(ConnectionState::NotConfigured)
    }

    pub fn is_configured(&self) -> bool {
        self.sender.is_some()
    }

    /// Sends the desired presence if it changed since the last call.
    pub fn update(&mut self, desired: Desired, flags: Flags) {
        let next = (desired, flags);
        if self.last.as_ref() == Some(&next) {
            return;
        }
        if let Some(tx) = &self.sender {
            let _ = tx.send(Message::Update(next.0.clone(), next.1));
        }
        self.last = Some(next);
    }

    pub fn retry(&self) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(Message::Retry);
        }
    }

    pub fn disconnect(&self, clear: bool) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(Message::Disconnect(clear));
        }
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        if let Some(tx) = self.sender.take() {
            let _ = tx.send(Message::Shutdown);
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}
