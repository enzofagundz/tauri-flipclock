//! MPRIS adapter: reads what an allowlisted browser is playing over the D-Bus
//! session bus and publishes the normalized model to the frontend.
//!
//! The domain decisions (allowlist, player selection, metadata normalization)
//! live in `flipclock_core::now_playing`; this module only adapts D-Bus.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use flipclock_core::now_playing::{
    detect_image_mime, is_browser_player, micros_to_millis, parse_artwork_url, ArtistValue,
    Artwork, ArtworkRef, Capabilities, MediaEvent, NowPlayingState, PlaybackStatus, PlayerSnapshot,
    RawMetadata, TrackMetadata, MAX_ARTWORK_BYTES,
};
use tauri::{AppHandle, Manager, Runtime};

use crate::state::AppState;

/// Event carrying the current now-playing model (or `null`).
pub const UPDATE_EVENT: &str = "now_playing:update";

const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";
const ROOT_INTERFACE: &str = "org.mpris.MediaPlayer2";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";

const CALL_TIMEOUT: Duration = Duration::from_millis(500);
const PROCESS_TIMEOUT: Duration = Duration::from_millis(250);
const SCAN_INTERVAL: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_secs(1);

const PROPERTIES_CHANGED_MATCH: &str = "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2',arg0='org.mpris.MediaPlayer2.Player'";
const NAME_OWNER_CHANGED_MATCH: &str = "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',path='/org/freedesktop/DBus'";

/// Cached local artwork location for the selected player, kept so the
/// `get_artwork` command can read the bytes without touching D-Bus.
#[derive(Debug, Clone)]
pub struct ArtworkResource {
    pub player: String,
    pub path: PathBuf,
    pub mime: String,
}

/// What the worker lifecycle should do after a settings change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerAction {
    Start,
    Stop,
    Keep,
}

/// Pure lifecycle decision: start when enabled and idle, stop when disabled and
/// running, otherwise leave the worker alone.
pub fn worker_action(enabled: bool, running: bool) -> WorkerAction {
    match (enabled, running) {
        (true, false) => WorkerAction::Start,
        (false, true) => WorkerAction::Stop,
        _ => WorkerAction::Keep,
    }
}

/// Handle to the D-Bus reader thread, used to stop it deterministically.
pub struct MediaWorker {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl MediaWorker {
    /// Asks the thread to finish and waits for it to leave the D-Bus loop.
    pub fn stop(self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.handle.join();
    }
}

/// Spawns the named reader thread. The connection is opened inside the thread
/// because the blocking connection is not `Sync`.
pub fn start<R: Runtime>(app: AppHandle<R>) -> Option<MediaWorker> {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let handle = thread::Builder::new()
        .name("flipclock-media".to_string())
        .spawn(move || run(app, thread_stop))
        .ok()?;
    Some(MediaWorker { stop, handle })
}

/// Reads the players once, synchronously, so startup state exists before the
/// first frontend frame. Failures are logged once and otherwise ignored.
pub fn snapshot_once<R: Runtime>(app: &AppHandle<R>) {
    let connection = match Connection::new_session() {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("flipclock: cannot connect to the D-Bus session bus: {error}");
            return;
        }
    };

    let mut state = NowPlayingState::default();
    let mut known = HashSet::new();
    let mut failures = HashMap::new();
    let resources = scan(&connection, &mut state, &mut known, &mut failures).unwrap_or_default();

    let model = state.model();
    let artwork = model
        .as_ref()
        .and_then(|model| resources.get(&model.player).cloned());
    app.state::<AppState>()
        .publish_now_playing(app, model, artwork);
}

fn run<R: Runtime>(app: AppHandle<R>, stop: Arc<AtomicBool>) {
    let connection = match Connection::new_session() {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("flipclock: cannot connect to the D-Bus session bus: {error}");
            return;
        }
    };
    let _ = connection.add_match_no_cb(PROPERTIES_CHANGED_MATCH);
    let _ = connection.add_match_no_cb(NAME_OWNER_CHANGED_MATCH);

    let mut state = NowPlayingState::default();
    let mut known = HashSet::new();
    let mut failures = HashMap::new();
    let mut artwork: Option<ArtworkResource> = None;

    rescan(
        &connection,
        &mut state,
        &mut known,
        &mut failures,
        &app,
        &mut artwork,
    );

    let mut next_scan = Instant::now() + SCAN_INTERVAL;
    let mut next_poll = Instant::now() + POLL_INTERVAL;

    while !stop.load(Ordering::Acquire) {
        let signal = connection.process(PROCESS_TIMEOUT).unwrap_or(false);
        let now = Instant::now();

        if signal || now >= next_scan {
            rescan(
                &connection,
                &mut state,
                &mut known,
                &mut failures,
                &app,
                &mut artwork,
            );
            next_scan = now + SCAN_INTERVAL;
        } else if now >= next_poll {
            poll_position(&connection, &mut state, &app, &mut artwork);
            next_poll = now + POLL_INTERVAL;
        }
    }

    // The connection and thread are dropped together with this frame.
}

/// Reads every allowlisted player and returns the artwork resources found for
/// the players whose local artwork was accepted.
fn scan(
    connection: &Connection,
    state: &mut NowPlayingState,
    known: &mut HashSet<String>,
    failures: &mut HashMap<String, String>,
) -> Option<HashMap<String, ArtworkResource>> {
    let names = match list_names(connection) {
        Ok(names) => {
            failures.remove("dbus:list");
            names
        }
        Err(error) => {
            note_failure(
                failures,
                "dbus:list",
                format!("cannot list D-Bus names: {error}"),
            );
            return None;
        }
    };

    let mut seen = HashSet::new();
    let mut resources = HashMap::new();

    for name in names.into_iter().filter(|name| is_browser_player(name)) {
        seen.insert(name.clone());
        if let Some(read) = read_player(connection, &name, failures) {
            if let Some(resource) = read.resource {
                resources.insert(name, resource);
            }
            state.apply(MediaEvent::PlayerUpdated(read.snapshot));
        }
    }

    for name in known.difference(&seen) {
        state.apply(MediaEvent::PlayerGone {
            bus_name: name.clone(),
        });
    }
    *known = seen;

    Some(resources)
}

fn rescan<R: Runtime>(
    connection: &Connection,
    state: &mut NowPlayingState,
    known: &mut HashSet<String>,
    failures: &mut HashMap<String, String>,
    app: &AppHandle<R>,
    artwork: &mut Option<ArtworkResource>,
) {
    if let Some(mut resources) = scan(connection, state, known, failures) {
        *artwork = state
            .model()
            .and_then(|model| resources.remove(&model.player));
    }
    publish(app, state, artwork);
}

fn poll_position<R: Runtime>(
    connection: &Connection,
    state: &mut NowPlayingState,
    app: &AppHandle<R>,
    artwork: &mut Option<ArtworkResource>,
) {
    let Some(player) = state.playing_player().map(str::to_string) else {
        return;
    };

    let position = number_property(connection, &player, PLAYER_INTERFACE, "Position")
        .ok()
        .flatten();
    if let Some(micros) = position {
        state.apply(MediaEvent::PositionObserved {
            bus_name: player,
            position_ms: micros_to_millis(micros),
        });
    }

    publish(app, state, artwork);
}

fn publish<R: Runtime>(
    app: &AppHandle<R>,
    state: &NowPlayingState,
    artwork: &mut Option<ArtworkResource>,
) {
    let model = state.model();
    let resource = match &model {
        Some(model) => artwork
            .take()
            .filter(|resource| resource.player == model.player),
        None => {
            artwork.take();
            None
        }
    };
    *artwork = resource.clone();
    app.state::<AppState>()
        .publish_now_playing(app, model, resource);
}

struct PlayerRead {
    snapshot: PlayerSnapshot,
    resource: Option<ArtworkResource>,
}

fn read_player(
    connection: &Connection,
    player: &str,
    failures: &mut HashMap<String, String>,
) -> Option<PlayerRead> {
    let status = match string_property(connection, player, PLAYER_INTERFACE, "PlaybackStatus") {
        Ok(status) => match status.as_deref() {
            Some(status) => playback_status(status),
            None => PlaybackStatus::Stopped,
        },
        Err(error) => {
            note_failure(
                failures,
                player,
                format!("cannot read playback status: {error}"),
            );
            return None;
        }
    };
    failures.remove(player);

    let identity = string_property(connection, player, ROOT_INTERFACE, "Identity")
        .ok()
        .flatten();

    let raw = metadata_property(connection, player).unwrap_or_default();

    let capabilities = Capabilities {
        can_play: bool_property(connection, player, "CanPlay"),
        can_pause: bool_property(connection, player, "CanPause"),
        can_go_previous: bool_property(connection, player, "CanGoPrevious"),
        can_go_next: bool_property(connection, player, "CanGoNext"),
    };

    let position_ms = number_property(connection, player, PLAYER_INTERFACE, "Position")
        .ok()
        .flatten()
        .map(micros_to_millis);

    let (artwork, resource) = resolve_artwork(player, &raw, failures);
    let metadata = TrackMetadata::from_raw(raw, artwork);

    Some(PlayerRead {
        snapshot: PlayerSnapshot {
            bus_name: player.to_string(),
            identity,
            status,
            metadata,
            capabilities,
            position_ms,
        },
        resource,
    })
}

fn playback_status(status: &str) -> PlaybackStatus {
    match status {
        "Playing" => PlaybackStatus::Playing,
        "Paused" => PlaybackStatus::Paused,
        _ => PlaybackStatus::Stopped,
    }
}

fn resolve_artwork(
    player: &str,
    raw: &RawMetadata,
    failures: &mut HashMap<String, String>,
) -> (Option<Artwork>, Option<ArtworkResource>) {
    let Some(url) = raw.artwork_url.as_deref() else {
        return (None, None);
    };

    match parse_artwork_url(url) {
        Some(ArtworkRef::Remote(url)) => (Some(Artwork::Remote { url }), None),
        Some(ArtworkRef::Local(path)) => match read_artwork(&path) {
            Ok((mime, _)) => {
                failures.remove(player);
                let artwork = Artwork::Local {
                    mime: mime.to_string(),
                    token: artwork_token(url),
                };
                let resource = ArtworkResource {
                    player: player.to_string(),
                    path,
                    mime: mime.to_string(),
                };
                (Some(artwork), Some(resource))
            }
            Err(reason) => {
                note_failure(failures, player, format!("cannot read artwork: {reason}"));
                (None, None)
            }
        },
        None => (None, None),
    }
}

fn artwork_token(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Reads an artwork file, enforcing the size cap and sniffing its image type.
pub fn read_artwork(path: &Path) -> Result<(&'static str, Vec<u8>), String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_ARTWORK_BYTES {
        return Err(format!(
            "artwork is {} bytes, over the {MAX_ARTWORK_BYTES} byte limit",
            metadata.len()
        ));
    }

    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_ARTWORK_BYTES {
        return Err(format!(
            "artwork grew over the {MAX_ARTWORK_BYTES} byte limit"
        ));
    }

    let mime = detect_image_mime(&bytes).ok_or_else(|| "unrecognized image format".to_string())?;
    Ok((mime, bytes))
}

/// Sends an MPRIS transport command, refusing any player outside the allowlist
/// before opening a connection.
pub fn send_command(player: &str, member: &str) -> Result<(), String> {
    if !is_browser_player(player) {
        return Err(format!("`{player}` is not an allowlisted browser"));
    }

    let connection = Connection::new_session().map_err(|error| {
        eprintln!("flipclock: cannot connect to the D-Bus session bus: {error}");
        error.to_string()
    })?;
    let proxy = connection.with_proxy(player, MPRIS_PATH, CALL_TIMEOUT);
    let result: Result<(), dbus::Error> = proxy.method_call(PLAYER_INTERFACE, member, ());
    result.map_err(|error| {
        eprintln!("flipclock: media command failed for `{player}`: {error}");
        error.to_string()
    })
}

fn metadata_property(connection: &Connection, player: &str) -> Result<RawMetadata, dbus::Error> {
    let value = property(connection, player, PLAYER_INTERFACE, "Metadata")?;
    let mut raw = RawMetadata::default();

    let Some(mut items) = value.0.as_iter() else {
        return Ok(raw);
    };

    while let (Some(key), Some(item)) = (items.next(), items.next()) {
        let Some(key) = key.as_str() else {
            continue;
        };
        match key {
            "xesam:title" => raw.title = item.as_str().map(str::to_string),
            "xesam:artist" => raw.artists = parse_artists(item),
            "xesam:album" => raw.album = item.as_str().map(str::to_string),
            "mpris:artUrl" => raw.artwork_url = item.as_str().map(str::to_string),
            "mpris:length" => {
                raw.duration_ms = item.as_i64().and_then(micros).map(micros_to_millis)
            }
            _ => {}
        }
    }

    Ok(raw)
}

fn parse_artists(value: &dyn RefArg) -> ArtistValue {
    if let Some(artist) = value.as_str() {
        return ArtistValue::Single(artist.to_string());
    }

    let Some(items) = value.as_iter() else {
        return ArtistValue::default();
    };

    let mut artists = Vec::new();
    for item in items {
        if let Some(artist) = item.as_str() {
            artists.push(artist.to_string());
        } else if let Some(nested) = item.as_iter() {
            artists.extend(nested.filter_map(|artist| artist.as_str().map(str::to_string)));
        }
    }

    if artists.is_empty() {
        ArtistValue::default()
    } else {
        ArtistValue::Multiple(artists)
    }
}

fn micros(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn list_names(connection: &Connection) -> Result<Vec<String>, dbus::Error> {
    let proxy = connection.with_proxy(DBUS_INTERFACE, DBUS_PATH, CALL_TIMEOUT);
    let (names,): (Vec<String>,) = proxy.method_call(DBUS_INTERFACE, "ListNames", ())?;
    Ok(names)
}

fn with_proxy<'a>(
    connection: &'a Connection,
    player: &'a str,
    timeout: Duration,
) -> Proxy<'a, &'a Connection> {
    connection.with_proxy(player, MPRIS_PATH, timeout)
}

fn property(
    connection: &Connection,
    player: &str,
    interface: &str,
    name: &str,
) -> Result<Variant<Box<dyn RefArg>>, dbus::Error> {
    let proxy = with_proxy(connection, player, CALL_TIMEOUT);
    let (value,): (Variant<Box<dyn RefArg>>,) =
        proxy.method_call(PROPERTIES_INTERFACE, "Get", (interface, name))?;
    Ok(value)
}

fn string_property(
    connection: &Connection,
    player: &str,
    interface: &str,
    name: &str,
) -> Result<Option<String>, dbus::Error> {
    let value = property(connection, player, interface, name)?;
    Ok(value.0.as_str().map(str::to_string))
}

fn bool_property(connection: &Connection, player: &str, name: &str) -> bool {
    property(connection, player, PLAYER_INTERFACE, name)
        .ok()
        .and_then(|value| value.0.as_i64())
        .map(|value| value != 0)
        .unwrap_or(false)
}

fn number_property(
    connection: &Connection,
    player: &str,
    interface: &str,
    name: &str,
) -> Result<Option<u64>, dbus::Error> {
    let value = property(connection, player, interface, name)?;
    Ok(value.0.as_i64().and_then(micros))
}

fn note_failure(failures: &mut HashMap<String, String>, key: &str, message: String) {
    if failures.get(key) == Some(&message) {
        return;
    }
    eprintln!("flipclock: {message}");
    failures.insert(key.to_string(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_action_covers_the_four_lifecycle_cases() {
        assert_eq!(worker_action(true, false), WorkerAction::Start);
        assert_eq!(worker_action(false, true), WorkerAction::Stop);
        assert_eq!(worker_action(true, true), WorkerAction::Keep);
        assert_eq!(worker_action(false, false), WorkerAction::Keep);
    }
}
