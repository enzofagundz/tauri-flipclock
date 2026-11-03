export type Corner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

export type Scale = "small" | "medium" | "large";

export type Meridiem = "AM" | "PM";

export interface Settings {
  version: number;
  format_24h: boolean;
  show_seconds: boolean;
  show_date: boolean;
  pinned: boolean;
  show_now_playing: boolean;
  corner: Corner;
  scale: Scale;
}

export interface Reading {
  hour: string;
  minute: string;
  second: string;
  date: string;
  meridiem: Meridiem | null;
  animate: boolean;
}

export type MediaCommand = "play_pause" | "previous" | "next";

export type NowPlayingArtwork =
  | { kind: "local"; mime: string; token: string }
  | { kind: "remote"; url: string };

export interface NowPlayingProgress {
  elapsed_ms: number;
  duration_ms: number;
  elapsed: string;
  total: string;
}

export interface NowPlaying {
  player: string;
  label: string;
  title: string;
  artist: string | null;
  album: string | null;
  artwork: NowPlayingArtwork | null;
  playing: boolean;
  can_play_pause: boolean;
  can_go_previous: boolean;
  can_go_next: boolean;
  progress: NowPlayingProgress | null;
}

export interface Bootstrap {
  settings: Settings;
  reading: Reading;
  layer_shell_available: boolean;
  window_mode: "pinned" | "windowed";
  now_playing: NowPlaying | null;
}

export interface SaveResult {
  settings: Settings;
  restart_required: boolean;
}
