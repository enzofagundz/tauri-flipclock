import { getBootstrap, onTick, quit as quitApp, saveSettings, setFullscreen } from "./api";
import { createContextMenu } from "./context-menu";
import { prefersReducedMotion } from "./motion";
import { renderReading } from "./renderer";
import { createSettingsPanel } from "./settings-panel";
import type { Reading, SaveResult, Settings } from "./types";

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
  let fullscreen = false;

  clock.dataset.scale = settings.scale;

  const render = (): void => {
    renderReading(clock, reading, {
      showSeconds: settings.show_seconds,
      showDate: settings.show_date,
      reducedMotion: prefersReducedMotion(),
    });
  };

  let menu: ReturnType<typeof createContextMenu> | null = null;

  const applySettings = async (next: Settings): Promise<SaveResult> => {
    const result = await saveSettings(next);
    settings = result.settings;
    clock.dataset.scale = settings.scale;
    menu?.setPinned(settings.pinned);
    render();
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
  await onTick((next) => {
    reading = next;
    render();
  });
}

void start();
