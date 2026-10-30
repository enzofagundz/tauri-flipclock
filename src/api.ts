import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Bootstrap, Reading, SaveResult, Settings } from "./types";

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
