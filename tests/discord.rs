//! Discord Rich Presence: privacy rules and connection handling, without Discord.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kagero::config::DiscordSettings;
use kagero::integrations::discord_rpc::{
    assets, build_presence, ActivityData, AppMode, ConnectionState, Desired, DiscordPresence,
    Flags, PresenceClient, RpcTransport, MIN_UPDATE_INTERVAL,
};

#[derive(Default)]
struct Log {
    connects: usize,
    sets: Vec<ActivityData>,
    clears: usize,
    closes: usize,
}

/// Fake Discord: scripted connect results, optional failure of the next update.
struct FakeDiscord {
    log: Arc<Mutex<Log>>,
    running: bool,
    fail_next_set: bool,
}

impl FakeDiscord {
    fn new(running: bool) -> (Self, Arc<Mutex<Log>>) {
        let log = Arc::new(Mutex::new(Log::default()));
        (
            Self {
                log: log.clone(),
                running,
                fail_next_set: false,
            },
            log,
        )
    }
}

impl RpcTransport for FakeDiscord {
    fn connect(&mut self) -> Result<(), String> {
        self.log.lock().expect("lock").connects += 1;
        if self.running {
            Ok(())
        } else {
            Err("Couldn't connect to the Discord IPC pipe".into())
        }
    }
    fn set_activity(&mut self, a: &ActivityData) -> Result<(), String> {
        if self.fail_next_set {
            self.fail_next_set = false;
            return Err("broken pipe".into());
        }
        self.log.lock().expect("lock").sets.push(a.clone());
        Ok(())
    }
    fn clear_activity(&mut self) -> Result<(), String> {
        self.log.lock().expect("lock").clears += 1;
        Ok(())
    }
    fn close(&mut self) {
        self.log.lock().expect("lock").closes += 1;
    }
}

fn on() -> Flags {
    Flags {
        enabled: true,
        auto_reconnect: true,
        clear_on_disable: true,
    }
}

fn show(mode: AppMode) -> Desired {
    build_presence(
        mode,
        Some("walk"),
        1_700_000_000,
        &DiscordSettings::default(),
    )
}

#[test]
fn statuses_for_every_mode() {
    let cases = [
        (AppMode::Home, "Browsing the Workshop", "Ready to Create"),
        (
            AppMode::StaticDocument,
            "Creating Pixel Art",
            "Editing a Sprite",
        ),
        (AppMode::AnimationDocument, "Animating Sprites", "Animating"),
        (AppMode::Previewing, "Previewing Animation", "Animating"),
    ];
    for (mode, details, state) in cases {
        match show(mode) {
            Desired::Show(a) => {
                assert_eq!(a.details, details);
                assert_eq!(a.state, state);
                assert_eq!(a.large_image, assets::LOGO);
                assert!(a.large_image.starts_with("https://") && a.large_image.ends_with(".png"));
                assert_eq!(a.start, Some(1_700_000_000));
            }
            Desired::Clear => panic!("enabled by default"),
        }
    }
}

#[test]
fn privacy_settings_are_respected() {
    let defaults = DiscordSettings::default();
    assert!(!defaults.show_project_name, "project name is opt-in");
    let Desired::Show(a) = build_presence(
        AppMode::StaticDocument,
        Some("C:\\Users\\alice\\secret\\boss.kgr"),
        0,
        &defaults,
    ) else {
        panic!("shown");
    };
    assert!(!a.state.contains("boss") && !a.details.contains("alice"));

    let named = DiscordSettings {
        show_project_name: true,
        ..DiscordSettings::default()
    };
    let Desired::Show(a) = build_presence(
        AppMode::StaticDocument,
        Some("C:\\Users\\alice\\secret\\boss.kgr"),
        0,
        &named,
    ) else {
        panic!("shown");
    };
    assert_eq!(a.state, "Project: boss", "only the file stem is shared");
    assert!(!format!("{a:?}").contains("alice"));

    let quiet = DiscordSettings {
        show_mode: false,
        show_timer: false,
        ..DiscordSettings::default()
    };
    let Desired::Show(a) = build_presence(AppMode::Previewing, None, 5, &quiet) else {
        panic!("shown");
    };
    assert_eq!(a.details, "Using Kagero");
    assert_eq!(a.start, None);

    let off = DiscordSettings {
        enabled: false,
        ..DiscordSettings::default()
    };
    assert_eq!(build_presence(AppMode::Home, None, 0, &off), Desired::Clear);
}

#[test]
fn missing_application_id_is_a_quiet_no_op() {
    let mut presence = DiscordPresence::start("");
    assert!(!presence.is_configured());
    assert_eq!(presence.state(), ConnectionState::NotConfigured);
    presence.update(show(AppMode::Home), on());
    presence.retry();
    presence.disconnect(true);
}

#[test]
fn discord_not_running_schedules_retries_with_backoff() {
    let (fake, log) = FakeDiscord::new(false);
    let mut client = PresenceClient::new(fake);
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), on());
    let ConnectionState::NotConnected { reason, retry_in } = client.state().clone() else {
        panic!("not connected");
    };
    assert!(reason.contains("not running"), "{reason}");
    let first = retry_in.expect("auto retry");
    // No hammering: nothing happens before the retry time.
    client.step(t0 + first / 2, &show(AppMode::Home), on());
    assert_eq!(log.lock().expect("lock").connects, 1);
    client.step(t0 + first, &show(AppMode::Home), on());
    assert_eq!(log.lock().expect("lock").connects, 2);
    let ConnectionState::NotConnected {
        retry_in: Some(second),
        ..
    } = client.state().clone()
    else {
        panic!("still not connected");
    };
    assert!(second > first, "back-off grows");
}

#[test]
fn without_auto_reconnect_waits_for_manual_retry() {
    let (fake, log) = FakeDiscord::new(false);
    let mut client = PresenceClient::new(fake);
    let flags = Flags {
        auto_reconnect: false,
        ..on()
    };
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), flags);
    client.step(t0 + Duration::from_secs(600), &show(AppMode::Home), flags);
    assert_eq!(log.lock().expect("lock").connects, 1);
    client.retry(t0 + Duration::from_secs(601));
    client.step(t0 + Duration::from_secs(601), &show(AppMode::Home), flags);
    assert_eq!(log.lock().expect("lock").connects, 2);
}

#[test]
fn connects_sends_changes_only_and_rate_limits() {
    let (fake, log) = FakeDiscord::new(true);
    let mut client = PresenceClient::new(fake);
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), on());
    assert_eq!(client.state(), &ConnectionState::Connected);
    client.step(t0, &show(AppMode::Home), on());
    assert_eq!(log.lock().expect("lock").sets.len(), 1);
    // The same presence is not re-sent.
    client.step(t0 + Duration::from_secs(30), &show(AppMode::Home), on());
    assert_eq!(log.lock().expect("lock").sets.len(), 1);
    // A change within the rate limit waits.
    let t1 = t0 + Duration::from_secs(31);
    client.step(t1, &show(AppMode::StaticDocument), on());
    client.step(
        t1 + Duration::from_secs(1),
        &show(AppMode::Previewing),
        on(),
    );
    assert_eq!(log.lock().expect("lock").sets.len(), 2);
    client.step(t1 + MIN_UPDATE_INTERVAL, &show(AppMode::Previewing), on());
    let sets = &log.lock().expect("lock").sets;
    assert_eq!(sets.len(), 3);
    assert_eq!(sets[2].details, "Previewing Animation");
}

#[test]
fn reconnects_after_discord_closes() {
    let (mut fake, log) = FakeDiscord::new(true);
    fake.fail_next_set = true;
    let mut client = PresenceClient::new(fake);
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), on());
    client.step(t0, &show(AppMode::Home), on());
    assert!(matches!(
        client.state(),
        ConnectionState::NotConnected { .. }
    ));
    assert_eq!(log.lock().expect("lock").closes, 1);
    client.step(t0 + Duration::from_secs(5), &show(AppMode::Home), on());
    assert_eq!(client.state(), &ConnectionState::Connected);
    client.step(t0 + Duration::from_secs(5), &show(AppMode::Home), on());
    assert_eq!(log.lock().expect("lock").sets.len(), 1);
}

#[test]
fn disabling_clears_and_disconnects() {
    let (fake, log) = FakeDiscord::new(true);
    let mut client = PresenceClient::new(fake);
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), on());
    client.step(t0, &show(AppMode::Home), on());
    let off = Flags {
        enabled: false,
        ..on()
    };
    client.step(t0 + Duration::from_secs(10), &Desired::Clear, off);
    assert_eq!(client.state(), &ConnectionState::Disabled);
    let l = log.lock().expect("lock");
    assert_eq!(l.clears, 1);
    assert_eq!(l.closes, 1);
}

#[test]
fn user_disconnect_stays_off_until_retry() {
    let (fake, log) = FakeDiscord::new(true);
    let mut client = PresenceClient::new(fake);
    let t0 = Instant::now();
    client.step(t0, &show(AppMode::Home), on());
    client.disconnect(true);
    client.step(t0 + Duration::from_secs(60), &show(AppMode::Home), on());
    assert_eq!(client.state(), &ConnectionState::Disabled);
    assert_eq!(log.lock().expect("lock").connects, 1);
    client.retry(t0 + Duration::from_secs(61));
    client.step(t0 + Duration::from_secs(61), &show(AppMode::Home), on());
    assert_eq!(client.state(), &ConnectionState::Connected);
}
