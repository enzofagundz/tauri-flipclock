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

## Packaging

- [ ] `npm test`, both `cargo test` commands, and `npm run tauri build` pass.
- [ ] `sudo dnf install ./src-tauri/target/release/bundle/rpm/Flipclock-0.1.0-1.x86_64.rpm` adds **Flipclock** to the application menu.
- [ ] Launching from the menu starts the app with the saved settings.
- [ ] `sudo dnf remove flipclock` removes the app, desktop entry, icons, and license; `~/.config/io.github.enzofagundz.flipclock/` may remain.
- [ ] No AppImage, deb, or Flatpak artifact exists under `src-tauri/target/release/bundle/`.
