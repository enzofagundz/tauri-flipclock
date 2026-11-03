# Use MPRIS over D-Bus for now playing

The now playing card only exists in fullscreen, and only for media played by a browser. Reading it through MPRIS 2 on the D-Bus session bus, with the `dbus` crate's blocking API in a dedicated thread, keeps the app's shape: no async runtime, no subprocess, and a thread that mirrors the existing one-second clock tick. A pure state machine in `flipclock-core` consumes player events and produces the display model, so allowlist filtering, player selection, metadata normalization and progress visibility are testable without a bus.

Only bus names under `org.mpris.MediaPlayer2.<token>` with a token in `firefox`, `chromium`, `chrome`, `zen` or `brave` count as browsers. Everything else — Rhythmbox, Spotify, mpv, `playerctld` — is ignored even when it is the only player running. A playing player wins over a paused one; when nothing plays, the most recently active allowlisted player stays on screen so a paused track does not vanish. The same allowlist validates every IPC command, so the webview cannot address an arbitrary bus name.

Chrome and Brave report an advancing `Position` and `mpris:length`, so they get the progress row. Firefox and Zen report neither; the row is omitted rather than shown stuck at zero, and no total is invented for live streams. Positions are polled once per second while the selected player is playing. Durations and positions are converted from MPRIS microseconds once, in the core.

Cover art stays split: a `file:` path captured in the shell's own metadata snapshot is read by the shell and served as raw bytes with a MIME type sniffed from the bytes, because Chrome's artwork file has no extension and the webview must never send a path. An `http(s)` URL is handed to the webview and loaded directly, so the Rust binary still has no HTTP client. Missing, unreadable, oversized, non-image or vanished files fall back to a placeholder.

Turning off "Mostrar o que está tocando" drops the D-Bus connection and stops the poll, rather than leaving a worker subscribed to media it will not show.

Rejected alternatives:

- `playerctl` as a subprocess: extra runtime dependency, output parsing, and it hides the track identity needed to keep a paused player selected.
- Fetching remote artwork in Rust: adds an HTTP client and its proxy/TLS surface for something the webview already does.
- Doing MPRIS work in JavaScript: no session-bus access from the webview without another bridge.
- Showing the card in windowed or pinned mode: the widget is meant to stay small and quiet.

Consequences: building from source needs `dbus-devel`; libdbus is already present at runtime through GTK. The feature depends on the browser's MPRIS support, so a browser without it is invisible. Progress and controls are limited to what each browser reports, and there is no per-player configuration or user-editable allowlist.
