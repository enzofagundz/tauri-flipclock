# Manual checklist

Run on Fedora 44 with COSMIC 1.10 on Wayland, after building the RPM or with `npm run tauri dev`. Automated tests do not cover compositor behavior.

## Windowed mode

- [ ] First launch opens frameless, in Windowed mode, showing HH:MM.
- [ ] `FLIPCLOCK_DISABLE_LAYER_SHELL=1` starts in Windowed mode and the settings panel shows the pinned-unavailable notice.
- [ ] The clock keeps the correct minute across a minute rollover, with seconds both hidden and shown.
- [ ] Suspend for more than 2 seconds and resume: values snap without a cascade of flips.
- [ ] `Ctrl+,` opens the settings panel; Tab and Shift+Tab reach every control; `Esc` closes it.
- [ ] 12-hour mode shows two-digit hours and AM/PM; 24-hour hides AM/PM.
- [ ] Seconds and date toggles add and remove their regions, and the window resizes without clipping.
- [ ] Date changes at midnight without a flip.
- [ ] `Ctrl+Q` and the context menu's **Sair** both quit.
- [ ] Right-click opens the menu; it closes on outside click and on `Esc`.

## Pinned mode

- [ ] Pin the clock, confirm the restart notice, restart, and confirm it opens pinned.
- [ ] Unpin, confirm the restart notice, restart, and confirm it opens windowed.
- [ ] For each anchor corner, the clock sits 24 px from both edges of that corner.
- [ ] For each scale preset (Pequeno, Médio, Grande), HH:MM fits; repeat with seconds, date, and 12-hour AM/PM on and confirm nothing clips.
- [ ] The clock stays above a focused terminal and a focused browser.
- [ ] Clicking the clock focuses it; clicks beside its rectangle reach the window underneath.
- [ ] Fullscreen covers the screen, keeps flipping, and `Esc` restores the previous pinned placement.
- [ ] The context menu and settings panel remain reachable in fullscreen.
- [ ] A second launch focuses the existing clock and leaves only one instance running.

## Now playing (fullscreen)

- [ ] Play a track in Chrome and enter fullscreen: the card appears under the date with `CHROME`, the title and an artist • album line. Leave fullscreen: it disappears immediately. Repeat in Windowed and Pinned mode and confirm it never appears.
- [ ] Play a track with no album, and one with no artist: no empty separator or blank line is left behind.
- [ ] Play in Firefox and Zen, then enter fullscreen: the card shows cover, label, title and artist, with no progress row at all.
- [ ] Play in Chrome and Brave: the bar advances once per second with elapsed and total timestamps. Pause: the bar and both timestamps freeze, and the card stays. Resume: it continues from the reported position.
- [ ] Play a live stream without a duration: no progress row and no invented total.
- [ ] On a playlist, use **Faixa anterior** and **Próxima faixa** from the card; they act on the browser. On a single tab, confirm **Próxima faixa** and **Faixa anterior** are absent.
- [ ] Use play/pause from the card: the icon follows the browser's reported state, not an optimistic toggle. Kill the browser mid-command: the failure is logged once and the card does not change.
- [ ] Tab through the card in fullscreen: all visible controls are reachable with a visible focus ring. `Esc` still leaves fullscreen; the context menu and settings panel still work.
- [ ] With two browsers open, start playback in one: it wins the card. Pause both: the most recently active one stays. Close the last browser: the card disappears.
- [ ] Play Spotify, Rhythmbox or mpv only, with every browser closed: no card appears, even in fullscreen.
- [ ] A track with a local cover shows it; a track without cover shows the placeholder, never a broken-image icon. Change tracks: the cover is replaced. A remote `http(s)` cover loads.
- [ ] Enter fullscreen while a track is already playing: the first frame is already populated, not blank.
- [ ] Turn **Mostrar o que está tocando** off: the card hides immediately. Run `busctl --user monitor` and confirm the app no longer reads MPRIS or polls position; no `Properties.Get`/`GetAll` calls for `org.mpris.MediaPlayer2` appear while it is off.
- [ ] Turn the toggle back on: the card repopulates without restarting the app.
- [ ] Restart with the toggle off: it stays off. Delete the key from `settings.json`, restart: the feature is on again.

## Packaging

- [ ] `npm test`, both `cargo test` commands, and `npm run tauri build` pass.
- [ ] `sudo dnf install ./src-tauri/target/release/bundle/rpm/Flipclock-0.2.0-1.x86_64.rpm` adds **Flipclock** to the application menu.
- [ ] Launching from the menu starts the app with the saved settings.
- [ ] `sudo dnf remove flipclock` removes the app, desktop entry, icons, and license; `~/.config/io.github.enzofagundz.flipclock/` may remain.
- [ ] No AppImage, deb, or Flatpak artifact exists under `src-tauri/target/release/bundle/`.
