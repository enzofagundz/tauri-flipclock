//! Pure model of what an allowlisted browser is playing.
//!
//! The shell adapts MPRIS events into [`MediaEvent`] values; this module owns
//! the allowlist, player selection, metadata normalization and the display
//! model the frontend renders.

use std::path::PathBuf;

use serde::Serialize;

/// MPRIS bus-name tokens that belong to browsers.
pub const BROWSER_TOKENS: [&str; 5] = ["firefox", "chromium", "chrome", "zen", "brave"];

/// Largest artwork file the shell is willing to hand to the webview.
pub const MAX_ARTWORK_BYTES: u64 = 4 * 1024 * 1024;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// The bus-name token of an allowlisted browser, or `None` for anything else.
pub fn browser_token(bus_name: &str) -> Option<&str> {
    let rest = bus_name.strip_prefix(MPRIS_PREFIX)?;
    let token = rest.split('.').next()?;
    BROWSER_TOKENS.contains(&token).then_some(token)
}

/// Whether `bus_name` belongs to a browser the app is willing to read.
pub fn is_browser_player(bus_name: &str) -> bool {
    browser_token(bus_name).is_some()
}

/// Browser label for the card: the player identity without its `Mozilla ` prefix
/// and upper-cased, falling back to the bus-name token.
pub fn normalize_browser_label(identity: Option<&str>, bus_name: &str) -> String {
    let identity = identity.unwrap_or_default().trim();
    let base = identity
        .strip_prefix("Mozilla ")
        .or_else(|| identity.strip_prefix("mozilla "))
        .unwrap_or(identity)
        .trim();
    if base.is_empty() {
        return browser_token(bus_name).unwrap_or_default().to_uppercase();
    }
    base.to_uppercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtistValue {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for ArtistValue {
    fn default() -> Self {
        Self::Single(String::new())
    }
}

/// Metadata as delivered by MPRIS, before normalization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawMetadata {
    pub title: Option<String>,
    pub artists: ArtistValue,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Artwork the frontend can render without ever seeing a local path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Artwork {
    /// Bytes are fetched with the artwork command for the selected player.
    /// `token` changes per track so the webview can replace a cover that keeps
    /// the same MIME type.
    Local { mime: String, token: String },
    /// Loaded directly by the webview.
    Remote { url: String },
}

/// Artwork reference parsed from an MPRIS `mpris:artUrl` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtworkRef {
    Local(PathBuf),
    Remote(String),
}

/// Normalized track metadata used by the display model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork: Option<Artwork>,
    pub duration_ms: Option<u64>,
}

impl TrackMetadata {
    /// Trims values, joins array artists and omits empty artist/album parts.
    pub fn from_raw(raw: RawMetadata, artwork: Option<Artwork>) -> Self {
        Self {
            title: clean_text(raw.title).unwrap_or_default(),
            artist: clean_artists(raw.artists),
            album: clean_text(raw.album),
            artwork,
            duration_ms: raw.duration_ms,
        }
    }
}

fn clean_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_artists(artists: ArtistValue) -> Option<String> {
    let joined = match artists {
        ArtistValue::Single(artist) => clean_text(Some(artist)),
        ArtistValue::Multiple(artists) => {
            let parts: Vec<String> = artists
                .into_iter()
                .filter_map(|artist| clean_text(Some(artist)))
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        }
    };
    joined
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_go_previous: bool,
    pub can_go_next: bool,
}

/// One player as read from MPRIS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSnapshot {
    pub bus_name: String,
    pub identity: Option<String>,
    pub status: PlaybackStatus,
    pub metadata: TrackMetadata,
    pub capabilities: Capabilities,
    pub position_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaEvent {
    PlayerUpdated(PlayerSnapshot),
    PositionObserved { bus_name: String, position_ms: u64 },
    PlayerGone { bus_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub elapsed_ms: u64,
    pub duration_ms: u64,
    pub elapsed: String,
    pub total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NowPlaying {
    pub player: String,
    pub label: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork: Option<Artwork>,
    pub playing: bool,
    pub can_play_pause: bool,
    pub can_go_previous: bool,
    pub can_go_next: bool,
    pub progress: Option<Progress>,
}

/// Converts a microsecond duration or position into milliseconds.
pub fn micros_to_millis(micros: u64) -> u64 {
    micros / 1_000
}

/// `M:SS` below an hour and `H:MM:SS` from an hour onwards.
pub fn format_timestamp(millis: u64) -> String {
    let total_seconds = millis / 1_000;
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3_600;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Parses `mpris:artUrl` into a local file or a remote URL.
pub fn parse_artwork_url(url: &str) -> Option<ArtworkRef> {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("file://") {
        let path = percent_decode(rest);
        if path.is_empty() || !path.starts_with('/') {
            return None;
        }
        return Some(ArtworkRef::Local(PathBuf::from(path)));
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return Some(ArtworkRef::Remote(url.to_string()));
    }
    None
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Sniffs the image type from the bytes, because Chrome's artwork file has no
/// extension.
pub fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct PlayerEntry {
    bus_name: String,
    identity: Option<String>,
    status: PlaybackStatus,
    metadata: TrackMetadata,
    capabilities: Capabilities,
    position_ms: Option<u64>,
    last_position_ms: Option<u64>,
    position_advancing: bool,
    track_key: String,
    activity: u64,
}

impl PlayerEntry {
    fn from_snapshot(snapshot: PlayerSnapshot) -> Self {
        let track_key = track_key(&snapshot.metadata);
        let mut entry = Self {
            bus_name: snapshot.bus_name,
            identity: snapshot.identity,
            status: snapshot.status,
            metadata: snapshot.metadata,
            capabilities: snapshot.capabilities,
            position_ms: None,
            last_position_ms: None,
            position_advancing: false,
            track_key,
            activity: 0,
        };
        if let Some(position) = snapshot.position_ms {
            entry.observe_position(position);
        }
        entry
    }

    fn observe_position(&mut self, position_ms: u64) {
        if let Some(previous) = self.last_position_ms {
            if position_ms > previous {
                self.position_advancing = true;
            }
        }
        self.last_position_ms = Some(position_ms);
        self.position_ms = Some(position_ms);
    }
}

fn track_key(metadata: &TrackMetadata) -> String {
    format!(
        "{}\u{0}{}\u{0}{}",
        metadata.title,
        metadata.artist.as_deref().unwrap_or_default(),
        metadata.album.as_deref().unwrap_or_default()
    )
}

/// Pure state machine fed by the shell's D-Bus adapter.
#[derive(Debug, Default)]
pub struct NowPlayingState {
    players: Vec<PlayerEntry>,
    clock: u64,
}

impl NowPlayingState {
    /// Applies one event and reports whether the displayed model changed.
    pub fn apply(&mut self, event: MediaEvent) -> bool {
        let before = self.model();
        self.clock = self.clock.wrapping_add(1);
        let activity = self.clock;

        match event {
            MediaEvent::PlayerUpdated(snapshot) => self.upsert(snapshot, activity),
            MediaEvent::PositionObserved {
                bus_name,
                position_ms,
            } => {
                if let Some(entry) = self
                    .players
                    .iter_mut()
                    .find(|entry| entry.bus_name == bus_name)
                {
                    entry.activity = activity;
                    entry.observe_position(position_ms);
                }
            }
            MediaEvent::PlayerGone { bus_name } => {
                self.players.retain(|entry| entry.bus_name != bus_name);
            }
        }

        self.model() != before
    }

    fn upsert(&mut self, snapshot: PlayerSnapshot, activity: u64) {
        let key = track_key(&snapshot.metadata);
        if let Some(entry) = self
            .players
            .iter_mut()
            .find(|entry| entry.bus_name == snapshot.bus_name)
        {
            let before = entry.clone();
            let track_changed = entry.track_key != key;
            if track_changed {
                entry.track_key = key;
                entry.position_advancing = false;
                entry.last_position_ms = None;
                entry.position_ms = None;
            }
            entry.identity = snapshot.identity;
            entry.status = snapshot.status;
            entry.metadata = snapshot.metadata;
            entry.capabilities = snapshot.capabilities;
            match snapshot.position_ms {
                Some(position) => entry.observe_position(position),
                None if track_changed => entry.position_ms = None,
                None => {}
            }
            let changed = track_changed
                || before.identity != entry.identity
                || before.status != entry.status
                || before.metadata != entry.metadata
                || before.capabilities != entry.capabilities
                || before.position_ms != entry.position_ms;
            if changed {
                entry.activity = activity;
            }
            return;
        }

        let mut entry = PlayerEntry::from_snapshot(snapshot);
        entry.activity = activity;
        self.players.push(entry);
    }

    fn selected(&self) -> Option<&PlayerEntry> {
        self.players
            .iter()
            .filter(|entry| entry.status != PlaybackStatus::Stopped)
            .max_by_key(|entry| {
                let playing = entry.status == PlaybackStatus::Playing;
                (u8::from(playing), entry.activity)
            })
    }

    /// The player whose position should be polled, if any.
    pub fn playing_player(&self) -> Option<&str> {
        self.selected()
            .filter(|entry| entry.status == PlaybackStatus::Playing)
            .map(|entry| entry.bus_name.as_str())
    }

    pub fn model(&self) -> Option<NowPlaying> {
        let entry = self.selected()?;
        Some(NowPlaying {
            player: entry.bus_name.clone(),
            label: normalize_browser_label(entry.identity.as_deref(), &entry.bus_name),
            title: entry.metadata.title.clone(),
            artist: entry.metadata.artist.clone(),
            album: entry.metadata.album.clone(),
            artwork: entry.metadata.artwork.clone(),
            playing: entry.status == PlaybackStatus::Playing,
            can_play_pause: entry.capabilities.can_play || entry.capabilities.can_pause,
            can_go_previous: entry.capabilities.can_go_previous,
            can_go_next: entry.capabilities.can_go_next,
            progress: progress(entry),
        })
    }
}

fn progress(entry: &PlayerEntry) -> Option<Progress> {
    if !entry.position_advancing {
        return None;
    }
    let duration_ms = entry
        .metadata
        .duration_ms
        .filter(|duration| *duration > 0)?;
    let elapsed_ms = entry.position_ms.unwrap_or(0).min(duration_ms);
    Some(Progress {
        elapsed_ms,
        duration_ms,
        elapsed: format_timestamp(elapsed_ms),
        total: format_timestamp(duration_ms),
    })
}
