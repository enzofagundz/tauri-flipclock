# Flipclock

A small desktop flip clock for Linux. Each time digit sits on a card and changes with a falling-leaf animation. It can be pinned to a screen corner as a layer-shell overlay on Wayland and installs as an RPM on Fedora.

Visible text is Brazilian Portuguese. The clock uses local time and bundles its font. The only network access is a remote cover URL supplied by a browser, loaded by the webview; the app itself fetches nothing.

## Requirements

- Fedora 44, x86_64, Wayland.
- COSMIC 1.10 for pinned mode. Pinned mode needs a compositor that advertises `zwlr_layer_shell_v1`; without it the app runs in Windowed mode.
- Rust 1.99 or newer and Node.js 22 for development.
- System packages:

```sh
sudo dnf install gtk3-devel webkit2gtk4.1-devel librsvg2-devel gtk-layer-shell-devel dbus-devel rpm-build patchelf
```

## Development

```sh
npm install
npm run tauri dev
```

The first launch opens a frameless window with the clock. `FLIPCLOCK_DISABLE_LAYER_SHELL=1` forces Windowed mode and is the way to verify the fallback path:

```sh
FLIPCLOCK_DISABLE_LAYER_SHELL=1 npm run tauri dev
```

## Tests

Frontend (Vitest + jsdom):

```sh
npm test
```

Rust domain crate (formatting, ticks, settings, geometry):

```sh
cargo test --manifest-path crates/flipclock-core/Cargo.toml
```

Tauri shell (IPC commands, state):

```sh
cargo test --manifest-path src-tauri/Cargo.toml
```

Formatting and lints:

```sh
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml
```

## Build

```sh
npm run tauri build
```

The RPM is written to `src-tauri/target/release/bundle/rpm/Flipclock-0.2.0-1.x86_64.rpm`, along with the release binary at `src-tauri/target/release/flipclock`. No AppImage, deb, or Flatpak is produced.

## Install and remove

```sh
sudo dnf install ./src-tauri/target/release/bundle/rpm/Flipclock-0.2.0-1.x86_64.rpm
```

This installs `/usr/bin/flipclock`, a desktop entry, the app icon, and the Roboto Condensed OFL license. Launch **Flipclock** from the application menu or run `flipclock`.

Settings live in `~/.config/io.github.enzofagundz.flipclock/settings.json` and survive removal:

```sh
sudo dnf remove flipclock
```

## Pinned mode

Pinned mode turns the window into a layer-shell overlay on the overlay layer, anchored to the chosen corner with a 24 px margin. It stays above normal windows and does not reserve screen space. Switching between Windowed and Pinned mode requires a restart; the settings panel reports it.

If layer-shell is unavailable, or the app cannot initialize it, Flipclock opens in Windowed mode, tries always-on-top, and shows a notice in the settings panel. Pinned mode is Wayland-only; Windowed mode on X11 is not supported.

## Now playing

While the clock is fullscreen and **Mostrar o que está tocando** is on, a card under the date shows the media playing in an allowlisted browser: Firefox, Zen, Chromium, Chrome or Brave. It carries the browser label, title, artist • album, cover art, a progress bar with elapsed and total time where the browser reports them, and play/pause, previous and next where the browser supports them.

Firefox and Zen report neither a duration nor an advancing position, so their card has no progress row. Non-browser players such as Spotify, Rhythmbox and mpv never show a card, even when they are the only player running. With several browsers open, the one playing wins; when none is playing, the most recently active one stays on screen.

The card appears only in fullscreen. Turn it off with **Mostrar o que está tocando** in the settings panel (`Ctrl+,`): this closes the app's D-Bus connection and stops position polling, and turning it back on restarts both without relaunching. The toggle defaults to on and persists across restarts.

A local cover supplied by the browser is read by the backend and served to the webview; a remote `http(s)` cover is loaded directly by the webview. No other network access is made.

## Controls

- Right-click: context menu with settings, pin/unpin, fullscreen, and quit.
- `Ctrl+,`: open the settings panel.
- `Esc`: close the settings panel, or leave fullscreen.
- `Ctrl+Q`: quit.

The settings panel changes 12/24-hour time, seconds, the date, now playing, the anchor corner, and the scale preset live. Only pin/unpin needs a restart.

## Font license

Roboto Condensed is bundled under the SIL Open Font License. The license text is in [`src/assets/fonts/OFL.txt`](src/assets/fonts/OFL.txt) and ships with the RPM as `/usr/lib/Flipclock/licenses/Roboto-Condensed-OFL.txt`.
