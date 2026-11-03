import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createNowPlayingCard, type NowPlayingCard, type NowPlayingStatus } from "./now-playing";
import type { MediaCommand, NowPlaying } from "./types";

let createdUrls: string[];
let revokedUrls: string[];

beforeEach(() => {
  createdUrls = [];
  revokedUrls = [];

  Object.defineProperty(URL, "createObjectURL", {
    configurable: true,
    writable: true,
    value: () => {
      const url = `blob:flipclock/${createdUrls.length + 1}`;
      createdUrls.push(url);
      return url;
    },
  });
  Object.defineProperty(URL, "revokeObjectURL", {
    configurable: true,
    writable: true,
    value: (url: string) => {
      revokedUrls.push(url);
    },
  });
});

afterEach(() => {
  document.body.innerHTML = "";
});

function mount(): HTMLElement {
  document.body.innerHTML = '<main id="clock" class="clock" data-scale="medium"></main>';
  const root = document.getElementById("clock");
  if (root === null) {
    throw new Error("clock root is missing");
  }
  return root;
}

function model(overrides: Partial<NowPlaying> = {}): NowPlaying {
  return {
    player: "chromium",
    label: "Chrome",
    title: "Everything In Its Right Place",
    artist: "Radiohead",
    album: "Kid A",
    artwork: null,
    playing: true,
    can_play_pause: true,
    can_go_previous: true,
    can_go_next: true,
    progress: null,
    ...overrides,
  };
}

function status(
  next: NowPlaying | null,
  overrides: Partial<Omit<NowPlayingStatus, "model">> = {},
): NowPlayingStatus {
  return {
    model: next,
    fullscreen: true,
    enabled: true,
    reducedMotion: false,
    ...overrides,
  };
}

function flush(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

interface Harness {
  root: HTMLElement;
  card: NowPlayingCard;
  commands: Array<[MediaCommand, string]>;
  loader: ReturnType<typeof vi.fn>;
  update(status: NowPlayingStatus): void;
  aside(): HTMLElement;
}

function harness(
  loadArtwork: (player: string, mime: string) => Promise<Blob> = async () =>
    new Blob(["art"], { type: "image/png" }),
): Harness {
  const root = mount();
  const commands: Array<[MediaCommand, string]> = [];
  const loader = vi.fn(loadArtwork);

  const card = createNowPlayingCard(root, {
    loadArtwork: loader,
    onCommand: (command, player) => {
      commands.push([command, player]);
    },
  });

  return {
    root,
    card,
    commands,
    loader,
    update(next: NowPlayingStatus): void {
      card.update(next);
    },
    aside(): HTMLElement {
      const node = root.querySelector<HTMLElement>(".now-playing");
      if (node === null) {
        throw new Error("now playing card is missing");
      }
      return node;
    },
  };
}

function query(parent: HTMLElement, selector: string): HTMLElement {
  const node = parent.querySelector<HTMLElement>(selector);
  if (node === null) {
    throw new Error(`${selector} is missing`);
  }
  return node;
}

function button(aside: HTMLElement, command: MediaCommand): HTMLButtonElement {
  return query(aside, `[data-command="${command}"]`) as HTMLButtonElement;
}

function art(aside: HTMLElement): HTMLImageElement {
  return query(aside, ".now-playing__art") as HTMLImageElement;
}

describe("createNowPlayingCard", () => {
  it("renders label, title and the artist with album line", () => {
    const view = harness();
    view.update(status(model()));

    const aside = view.aside();
    expect(query(aside, ".now-playing__label").textContent).toBe("Chrome");
    expect(query(aside, ".now-playing__title").textContent).toBe(
      "Everything In Its Right Place",
    );

    const meta = query(aside, ".now-playing__meta");
    expect(meta.textContent).toBe("Radiohead • Kid A");
    expect(meta.hidden).toBe(false);
  });

  it("omits the missing half of the meta line and drops the line when both are empty", () => {
    const view = harness();

    view.update(status(model({ artist: null, album: "Kid A" })));
    expect(query(view.aside(), ".now-playing__meta").textContent).toBe("Kid A");

    view.update(status(model({ artist: "Radiohead", album: null })));
    expect(query(view.aside(), ".now-playing__meta").textContent).toBe("Radiohead");

    view.update(status(model({ artist: "", album: null })));
    const meta = query(view.aside(), ".now-playing__meta");
    expect(meta.textContent).toBe("");
    expect(meta.hidden).toBe(true);
  });

  it("stays hidden unless fullscreen, enabled and holding a model", () => {
    const view = harness();
    const aside = view.aside();

    view.update(status(null));
    expect(aside.hidden).toBe(true);

    view.update(status(model(), { fullscreen: false }));
    expect(aside.hidden).toBe(true);

    view.update(status(model(), { enabled: false }));
    expect(aside.hidden).toBe(true);

    view.update(status(model(), { fullscreen: false, enabled: false }));
    expect(aside.hidden).toBe(true);

    view.update(status(model()));
    expect(aside.hidden).toBe(false);

    view.update(status(null, { fullscreen: true, enabled: true }));
    expect(aside.hidden).toBe(true);
  });

  it("shows the local cover from the injected loader and the placeholder without one", async () => {
    const view = harness();
    view.update(status(model({ artwork: { kind: "local", mime: "image/png", token: "track-1" } })));
    await flush();

    expect(view.loader).toHaveBeenCalledWith("chromium", "image/png");
    expect(createdUrls).toHaveLength(1);
    expect(art(view.aside()).getAttribute("src")).toBe(createdUrls[0]);
    expect(art(view.aside()).hidden).toBe(false);

    view.update(status(model({ artwork: null })));
    expect(art(view.aside()).hidden).toBe(true);
    expect(art(view.aside()).getAttribute("src")).toBeNull();
    expect(revokedUrls).toContain(createdUrls[0]);
  });

  it("keeps the placeholder when the loader fails", async () => {
    const view = harness(async () => {
      throw new Error("no artwork for this player");
    });
    view.update(status(model({ artwork: { kind: "local", mime: "image/png", token: "track-1" } })));
    await flush();

    const cover = query(view.aside(), ".now-playing__cover");
    expect(cover.hidden).toBe(false);
    expect(art(view.aside()).hidden).toBe(true);
    expect(art(view.aside()).hasAttribute("src")).toBe(false);
    expect(createdUrls).toHaveLength(0);
  });

  it("renders the remote cover URL directly", async () => {
    const view = harness();
    view.update(
      status(model({ artwork: { kind: "remote", url: "https://music.test/cover.jpg" } })),
    );
    await flush();

    expect(view.loader).not.toHaveBeenCalled();
    expect(art(view.aside()).getAttribute("src")).toBe("https://music.test/cover.jpg");
    expect(art(view.aside()).hidden).toBe(false);
  });

  it("hides the progress row without progress and fills the bar when present", () => {
    const view = harness();
    const aside = view.aside();

    view.update(status(model({ progress: null })));
    expect(query(aside, ".now-playing__progress").hidden).toBe(true);

    view.update(
      status(
        model({
          progress: { elapsed_ms: 30000, duration_ms: 120000, elapsed: "0:30", total: "2:00" },
        }),
      ),
    );
    const progress = query(aside, ".now-playing__progress");
    const bar = query(aside, ".now-playing__bar");
    const elapsed = aside.querySelectorAll<HTMLElement>(".now-playing__time")[0];
    const total = aside.querySelectorAll<HTMLElement>(".now-playing__time")[1];

    expect(progress.hidden).toBe(false);
    expect(elapsed?.textContent).toBe("0:30");
    expect(total?.textContent).toBe("2:00");
    expect(bar.getAttribute("role")).toBe("progressbar");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");
    expect(bar.getAttribute("aria-valuenow")).toBe("25");
    expect(bar.getAttribute("aria-label")).toBe("Progresso da faixa");
    expect(query(aside, ".now-playing__fill").style.width).toBe("25%");

    view.update(
      status(
        model({
          progress: { elapsed_ms: 240000, duration_ms: 120000, elapsed: "4:00", total: "2:00" },
        }),
      ),
    );
    expect(bar.getAttribute("aria-valuenow")).toBe("100");
  });

  it("keeps the same progress text while paused", () => {
    const view = harness();
    const progress = {
      elapsed_ms: 45000,
      duration_ms: 180000,
      elapsed: "0:45",
      total: "3:00",
    };

    view.update(status(model({ progress, playing: true })));
    const before = query(view.aside(), ".now-playing__progress").textContent;

    view.update(status(model({ progress, playing: false })));
    const aside = view.aside();
    const after = query(aside, ".now-playing__progress").textContent;

    expect(after).toBe(before);
    expect(query(aside, ".now-playing__bar").getAttribute("aria-valuenow")).toBe("25");
    expect(aside.querySelectorAll<HTMLElement>(".now-playing__time")[0]?.textContent).toBe("0:45");
  });

  it("hides controls the model cannot drive and follows the playing state", () => {
    const view = harness();
    const aside = view.aside();

    view.update(
      status(
        model({
          can_play_pause: false,
          can_go_previous: false,
          can_go_next: false,
        }),
      ),
    );
    expect(button(aside, "previous").hidden).toBe(true);
    expect(button(aside, "play_pause").hidden).toBe(true);
    expect(button(aside, "next").hidden).toBe(true);

    view.update(status(model({ can_go_previous: false, playing: false })));
    expect(button(aside, "previous").hidden).toBe(true);
    expect(button(aside, "play_pause").hidden).toBe(false);
    expect(button(aside, "next").hidden).toBe(false);

    expect(button(aside, "play_pause").getAttribute("aria-label")).toBe("Reproduzir");
    expect(button(aside, "play_pause").querySelectorAll("path")).toHaveLength(1);

    view.update(status(model({ playing: true })));
    expect(button(aside, "play_pause").getAttribute("aria-label")).toBe("Pausar");
    expect(button(aside, "play_pause").querySelectorAll("path")).toHaveLength(2);

    expect(button(aside, "previous").getAttribute("aria-label")).toBe("Faixa anterior");
    expect(button(aside, "next").getAttribute("aria-label")).toBe("Próxima faixa");
  });

  it("dispatches the command with the player id when a button is clicked", () => {
    const view = harness();
    view.update(status(model({ player: "spotify" })));

    const aside = view.aside();
    button(aside, "previous").click();
    button(aside, "play_pause").click();
    button(aside, "next").click();

    expect(view.commands).toEqual([
      ["previous", "spotify"],
      ["play_pause", "spotify"],
      ["next", "spotify"],
    ]);
  });

  it("revokes the previous blob URL when the artwork changes and on destroy", async () => {
    const view = harness();

    view.update(status(model({ artwork: { kind: "local", mime: "image/png", token: "track-1" } })));
    await flush();
    expect(createdUrls).toHaveLength(1);

    view.update(status(model({ artwork: { kind: "local", mime: "image/jpeg", token: "track-2" } })));
    await flush();
    expect(createdUrls).toHaveLength(2);
    expect(revokedUrls).toEqual([createdUrls[0]]);
    expect(art(view.aside()).getAttribute("src")).toBe(createdUrls[1]);

    view.card.destroy();
    expect(revokedUrls).toEqual([createdUrls[0], createdUrls[1]]);
    expect(view.root.querySelector(".now-playing")).toBeNull();
  });

  it("reloads the cover when the track changes but the mime type stays the same", async () => {
    const { card, loader, update } = harness(async () => new Blob(["art"], { type: "image/png" }));
    update(status(model({ artwork: { kind: "local", mime: "image/png", token: "track-1" } })));
    await flush();
    update(status(model({ title: "Idioteque", artwork: { kind: "local", mime: "image/png", token: "track-2" } })));
    await flush();

    expect(loader).toHaveBeenCalledTimes(2);
    expect(createdUrls).toHaveLength(2);
    expect(revokedUrls).toEqual([createdUrls[0]]);
    card.destroy();
  });

  it("does not refetch the same player artwork on every tick", async () => {
    const view = harness();
    const artwork = { kind: "local" as const, mime: "image/png", token: "track-1" };

    view.update(status(model({ artwork })));
    await flush();
    view.update(status(model({ artwork, title: "Kid A" })));
    view.update(status(model({ artwork, title: "Idioteque" })));
    await flush();

    expect(view.loader).toHaveBeenCalledTimes(1);
    expect(createdUrls).toHaveLength(1);
    expect(revokedUrls).toEqual([]);
  });

  it("ignores a stale artwork load that resolves after a newer model", async () => {
    let resolveFirst: (blob: Blob) => void = () => undefined;
    const loader = vi
      .fn<(player: string, mime: string) => Promise<Blob>>()
      .mockImplementationOnce(
        () =>
          new Promise<Blob>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(async () => new Blob(["second"], { type: "image/jpeg" }));

    const view = harness(loader);

    view.update(status(model({ player: "chromium", artwork: { kind: "local", mime: "image/png", token: "track-1" } })));
    view.update(status(model({ player: "spotify", artwork: { kind: "local", mime: "image/jpeg", token: "track-2" } })));
    await flush();

    expect(createdUrls).toHaveLength(1);
    const fresh = createdUrls[0];
    expect(art(view.aside()).getAttribute("src")).toBe(fresh);

    resolveFirst(new Blob(["first"], { type: "image/png" }));
    await flush();

    expect(createdUrls).toHaveLength(1);
    expect(art(view.aside()).getAttribute("src")).toBe(fresh);
    expect(revokedUrls).toEqual([]);
  });

  it("drops the progress transition when reduced motion is requested", () => {
    const view = harness();
    const aside = view.aside();

    view.update(status(model(), { reducedMotion: true }));
    expect(aside.classList.contains("now-playing--reduced-motion")).toBe(true);

    view.update(status(model(), { reducedMotion: false }));
    expect(aside.classList.contains("now-playing--reduced-motion")).toBe(false);
  });
});
