import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Bootstrap, MediaCommand, NowPlaying, Reading, SaveResult, Settings } from "./types";

export function getBootstrap(): Promise<Bootstrap> {
  return invoke<Bootstrap>("get_bootstrap");
}

export function saveSettings(settings: Settings): Promise<SaveResult> {
  return invoke<SaveResult>("save_settings", { settings });
}

export function setFullscreen(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_fullscreen", { enabled });
}

export function quit(): Promise<void> {
  return invoke<void>("quit");
}

export function onTick(handler: (reading: Reading) => void): Promise<UnlistenFn> {
  return listen<Reading>("clock:tick", (event) => {
    handler(event.payload);
  });
}

export function onNowPlaying(handler: (model: NowPlaying | null) => void): Promise<UnlistenFn> {
  return listen<NowPlaying | null>("now_playing:update", (event) => {
    handler(event.payload);
  });
}

export async function getArtwork(player: string, mime: string): Promise<Blob> {
  const payload = await invoke<ArrayBuffer | number[]>("get_artwork", { player });
  const bytes = payload instanceof ArrayBuffer ? payload : new Uint8Array(payload).buffer;
  return new Blob([bytes], { type: mime });
}

export function mediaCommand(player: string, command: MediaCommand): Promise<void> {
  return invoke<void>("media_command", { player, command });
}
