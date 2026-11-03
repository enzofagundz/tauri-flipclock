import { beforeEach, describe, expect, it, vi } from "vitest";
import { createSettingsPanel } from "./settings-panel";
import type { SaveResult, Settings } from "./types";

function settings(overrides: Partial<Settings> = {}): Settings {
  return {
    version: 1,
    format_24h: false,
    show_seconds: false,
    show_date: false,
    pinned: false,
    show_now_playing: true,
    corner: "top_right",
    scale: "medium",
    ...overrides,
  };
}

function query<T extends HTMLElement>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (found === null) {
    throw new Error(`${selector} is missing`);
  }
  return found;
}

function toggle(name: string): HTMLInputElement {
  return query<HTMLInputElement>(`[data-setting="${name}"][type="checkbox"]`);
}

function radio(name: string, value: string): HTMLInputElement {
  return query<HTMLInputElement>(`[data-setting="${name}"][type="radio"][value="${value}"]`);
}

function pick(input: HTMLInputElement): void {
  input.dispatchEvent(new Event("change", { bubbles: true }));
}

function flush(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

function save(restartRequired = false): ReturnType<typeof vi.fn> {
  return vi.fn(
    async (next: Settings): Promise<SaveResult> => ({
      settings: next,
      restart_required: restartRequired,
    }),
  );
}

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("createSettingsPanel", () => {
  it("opens over the clock with the current settings", () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });

    panel.open(settings({ format_24h: true, corner: "bottom_left", scale: "large" }));

    expect(panel.isOpen()).toBe(true);
    expect(query<HTMLElement>(".settings").hidden).toBe(false);
    expect(query<HTMLElement>(".settings__card").getAttribute("aria-modal")).toBe("true");
    expect(toggle("format_24h").checked).toBe(true);
    expect(radio("corner", "bottom_left").checked).toBe(true);
    expect(radio("scale", "large").checked).toBe(true);
    expect(document.activeElement).toBe(toggle("format_24h"));
  });

  it("saves the 12/24-hour choice immediately", async () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings({ format_24h: false }));

    const input = toggle("format_24h");
    input.checked = true;
    pick(input);
    await flush();

    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange.mock.calls[0]?.[0]).toMatchObject({ format_24h: true });
  });

  it("saves the seconds and date choices immediately", async () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings());

    const seconds = toggle("show_seconds");
    seconds.checked = true;
    pick(seconds);
    await flush();

    const date = toggle("show_date");
    date.checked = true;
    pick(date);
    await flush();

    expect(onChange).toHaveBeenCalledTimes(2);
    expect(onChange.mock.calls[0]?.[0]).toMatchObject({ show_seconds: true });
    expect(onChange.mock.calls[1]?.[0]).toMatchObject({ show_seconds: true, show_date: true });
  });

  it("reflects and saves the now playing toggle with its fullscreen note", async () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings({ show_now_playing: false }));

    const input = toggle("show_now_playing");
    expect(input.checked).toBe(false);

    const hint = Array.from(document.querySelectorAll<HTMLElement>(".field__hint")).find(
      (node) => node.textContent?.includes("tela cheia"),
    );
    expect(hint?.textContent).toBe("Só aparece em tela cheia.");
    expect(hint?.hidden).toBe(false);

    input.checked = true;
    pick(input);
    await flush();

    expect(onChange.mock.calls[0]?.[0]).toMatchObject({ show_now_playing: true });
  });

  it("opens with now playing enabled by default", () => {
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange: save() });
    panel.open(settings());

    expect(toggle("show_now_playing").checked).toBe(true);
  });

  it("saves the anchor corner and the scale preset", async () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings());

    const corner = radio("corner", "top_left");
    corner.checked = true;
    pick(corner);
    await flush();

    const scale = radio("scale", "small");
    scale.checked = true;
    pick(scale);
    await flush();

    expect(onChange.mock.calls[0]?.[0]).toMatchObject({ corner: "top_left" });
    expect(onChange.mock.calls[1]?.[0]).toMatchObject({ corner: "top_left", scale: "small" });
  });

  it("reports that a restart is required after pinning", async () => {
    const onChange = save(true);
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings({ pinned: false }));

    const pinned = toggle("pinned");
    pinned.checked = true;
    pick(pinned);
    await flush();

    expect(onChange.mock.calls[0]?.[0]).toMatchObject({ pinned: true });
    const notice = query<HTMLElement>('[role="status"]');
    expect(notice.hidden).toBe(false);
    expect(notice.textContent).toContain("Reinicie o Flipclock");
  });

  it("explains that pinned mode is unavailable without layer shell", () => {
    const onChange = save();
    const panel = createSettingsPanel({ layerShellAvailable: false, onChange });
    panel.open(settings());

    const notice = Array.from(document.querySelectorAll<HTMLElement>(".settings__notice")).find(
      (node) => node.textContent?.includes("modo fixado"),
    );
    expect(notice).toBeDefined();
    expect(notice?.hidden).toBe(false);
    expect(toggle("pinned").disabled).toBe(true);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("restores the last saved settings when a save is rejected", async () => {
    let calls = 0;
    const onChange = vi.fn(async (next: Settings): Promise<SaveResult> => {
      calls += 1;
      if (calls === 2) {
        throw new Error("rejected");
      }
      return { settings: next, restart_required: false };
    });
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange });
    panel.open(settings());

    const seconds = toggle("show_seconds");
    seconds.checked = true;
    pick(seconds);
    await flush();

    const date = toggle("show_date");
    date.checked = true;
    pick(date);
    await flush();

    expect(date.checked).toBe(false);
    expect(seconds.checked).toBe(true);

    const errorNotice = Array.from(document.querySelectorAll<HTMLElement>(".settings__notice")).find(
      (node) => node.textContent?.includes("Não foi possível salvar"),
    );
    expect(errorNotice?.hidden).toBe(false);

    const retry = toggle("format_24h");
    retry.checked = true;
    pick(retry);
    await flush();

    expect(onChange.mock.calls[2]?.[0]).toMatchObject({ show_seconds: true, show_date: false });
  });

  it("shows the restart notice on demand for changes made outside the panel", () => {
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange: save(true) });
    panel.open(settings());

    panel.showRestartNotice();

    const notice = Array.from(document.querySelectorAll<HTMLElement>(".settings__notice")).find(
      (node) => node.textContent?.includes("Reinicie o Flipclock"),
    );
    expect(notice?.hidden).toBe(false);
  });

  it("closes from the close button and the scrim", () => {
    const panel = createSettingsPanel({ layerShellAvailable: true, onChange: save() });
    panel.open(settings());

    query<HTMLButtonElement>(".settings__close").click();
    expect(panel.isOpen()).toBe(false);

    panel.open(settings());
    query<HTMLElement>(".settings").click();
    expect(panel.isOpen()).toBe(false);
  });
});
