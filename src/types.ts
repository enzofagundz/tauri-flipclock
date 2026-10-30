export type Corner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

export type Scale = "small" | "medium" | "large";

export type Meridiem = "AM" | "PM";

export interface Settings {
  version: number;
  format_24h: boolean;
  show_seconds: boolean;
  show_date: boolean;
  pinned: boolean;
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

export interface Bootstrap {
  settings: Settings;
  reading: Reading;
  layer_shell_available: boolean;
  window_mode: "pinned" | "windowed";
}

export interface SaveResult {
  settings: Settings;
  restart_required: boolean;
}
