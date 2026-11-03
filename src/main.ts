import {
  getArtwork,
  getBootstrap,
  mediaCommand,
  onNowPlaying,
  onTick,
  quit as quitApp,
  saveSettings,
  setFullscreen,
} from "./api";
import { createContextMenu } from "./context-menu";
import { prefersReducedMotion } from "./motion";
import { renderReading } from "./renderer";
import { createNowPlayingCard, type NowPlayingCard } from "./now-playing";
import { createSettingsPanel } from "./settings-panel";
import type { NowPlaying, Reading, SaveResult, Settings } from "./types";

function requireClock(): HTMLElement {
  const clock = document.querySelector<HTMLElement>("#clock");
  if (clock === null) {
    throw new Error("clock root is missing");
  }
  return clock;
}

async function start(): Promise<void> {
  const clock = requireClock();
  const bootstrap = await getBootstrap();

  let settings: Settings = bootstrap.settings;
  let reading: Reading = bootstrap.reading;
  let nowPlaying: NowPlaying | null = bootstrap.now_playing;
  let fullscreen = false;
  let card: NowPlayingCard | null = null;

  clock.dataset.scale = settings.scale;

  const nowPlayingLabel = (): string | undefined => {
    if (!fullscreen || !settings.show_now_playing || nowPlaying === null) {
      return undefined;
    }
    return `${nowPlaying.label}: ${nowPlaying.title}`;
  };

  const render = (): void => {
    renderReading(clock, reading, {
      showSeconds: settings.show_seconds,
      showDate: settings.show_date,
      reducedMotion: prefersReducedMotion(),
      nowPlaying: nowPlayingLabel(),
    });
  };

  const updateCard = (): void => {
    card?.update({
      model: nowPlaying,
      fullscreen,
      enabled: settings.show_now_playing,
      reducedMotion: prefersReducedMotion(),
    });
    render();
  };

  let menu: ReturnType<typeof createContextMenu> | null = null;

  const applySettings = async (next: Settings): Promise<SaveResult> => {
    const result = await saveSettings(next);
    settings = result.settings;
    clock.dataset.scale = settings.scale;
    menu?.setPinned(settings.pinned);
    render();
    updateCard();
    return result;
  };

  const panel = createSettingsPanel({
    layerShellAvailable: bootstrap.layer_shell_available,
    onChange: applySettings,
  });

  menu = createContextMenu({
    canPin: bootstrap.layer_shell_available,
    onSettings: () => {
      panel.open(settings);
    },
    onTogglePin: () => {
      void applySettings({ ...settings, pinned: !settings.pinned }).then(
        (result) => {
          if (result.restart_required) {
            panel.open(settings);
            panel.showRestartNotice();
          }
        },
        () => undefined,
      );
    },
    onToggleFullscreen: () => {
      void applyFullscreen(!fullscreen);
    },
    onQuit: () => {
      void quitApp();
    },
  });

  menu.setPinned(settings.pinned);

  async function applyFullscreen(enabled: boolean): Promise<void> {
    try {
      fullscreen = await setFullscreen(enabled);
      menu?.setFullscreen(fullscreen);
      updateCard();
    } catch (error) {
      console.error("flipclock: cannot change fullscreen", error);
    }
  }

  window.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    if (panel.isOpen()) {
      return;
    }
    menu?.open(event.clientX, event.clientY);
  });

  document.addEventListener("keydown", (event) => {
    if (event.defaultPrevented) {
      return;
    }

    if (event.key === "Escape") {
      if (menu?.isOpen() === true) {
        menu.close();
        event.preventDefault();
        return;
      }
      if (panel.isOpen()) {
        panel.close();
        event.preventDefault();
        return;
      }
      if (fullscreen) {
        void applyFullscreen(false);
        event.preventDefault();
      }
      return;
    }

    if (event.ctrlKey && !event.altKey && !event.metaKey && event.code === "Comma") {
      event.preventDefault();
      menu?.close();
      panel.open(settings);
      return;
    }

    if (event.ctrlKey && !event.altKey && !event.metaKey && event.key.toLowerCase() === "q") {
      event.preventDefault();
      void quitApp();
    }
  });

  render();

  let commandFailed = false;
  card = createNowPlayingCard(clock, {
    loadArtwork: getArtwork,
    onCommand: (command, player) => {
      void mediaCommand(player, command).then(
        () => {
          commandFailed = false;
        },
        (error: unknown) => {
          if (commandFailed) {
            return;
          }
          commandFailed = true;
          console.error("flipclock: media command failed", error);
        },
      );
    },
  });
  updateCard();

  await onNowPlaying((next) => {
    nowPlaying = next;
    updateCard();
  });

  await onTick((next) => {
    reading = next;
    render();
  });
}

void start();
