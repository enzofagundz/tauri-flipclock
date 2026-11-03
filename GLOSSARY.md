# Glossary

## Now Playing

The card shown under the date while the clock is fullscreen, describing the
current track of the selected player. It carries the browser label, title,
artist and album, cover art, progress and controls when available. It never
appears in Windowed or Pinned mode, and it is absent whenever no allowlisted
browser has media.

## Player

An MPRIS 2 media player on the D-Bus session bus. Flipclock only treats the
allowlisted browsers as players; other MPRIS clients are ignored even when they
are the only ones running. The selected player is the one the card describes
and the one every control command addresses.

## MPRIS

The D-Bus specification browsers implement to publish their media session:
`org.mpris.MediaPlayer2` for identity, and
`org.mpris.MediaPlayer2.Player` for playback status, metadata, position,
capabilities and commands. Flipclock reads it through the `dbus` crate and never
uses `playerctl` or another subprocess.

## Browser

A player whose MPRIS bus name sits under one of the allowlisted tokens:
`firefox`, `chromium`, `chrome`, `zen` or `brave`. The label on the card is the
player's identity without the `Mozilla ` prefix, upper-cased, falling back to
the bus-name token.

## Cover art

The artwork of the current track. A local file is served by the shell as raw
bytes with a MIME type sniffed from the content; a remote URL is loaded by the
webview. A missing or unusable cover becomes the placeholder, never a broken
image.
