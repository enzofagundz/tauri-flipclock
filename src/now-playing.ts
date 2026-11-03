import type { MediaCommand, NowPlaying } from "./types";

export interface NowPlayingStatus {
  model: NowPlaying | null;
  fullscreen: boolean;
  enabled: boolean;
  reducedMotion: boolean;
}

export interface NowPlayingCardOptions {
  loadArtwork(player: string, mime: string): Promise<Blob>;
  onCommand(command: MediaCommand, player: string): void;
}

export interface NowPlayingCard {
  update(status: NowPlayingStatus): void;
  destroy(): void;
}

const SVG_NS = "http://www.w3.org/2000/svg";

type IconName = "play" | "pause" | "previous" | "next";

const ICON_PATHS: Record<IconName, readonly string[]> = {
  previous: ["M7 5h2.4v14H7z", "M19 5v14L9.5 12z"],
  play: ["M8.5 5 19 12 8.5 19z"],
  pause: ["M7 5h3.6v14H7z", "M13.4 5H17v14h-3.6z"],
  next: ["M14.6 5H17v14h-2.4z", "M5 5l9.5 7L5 19z"],
};

function element(className: string, tag = "div"): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

function createIcon(name: IconName): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", "18");
  svg.setAttribute("height", "18");
  svg.setAttribute("fill", "currentColor");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  for (const d of ICON_PATHS[name]) {
    const path = document.createElementNS(SVG_NS, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function setText(node: HTMLElement, value: string): void {
  if (node.textContent !== value) {
    node.textContent = value;
  }
}

function metaLine(model: NowPlaying): string {
  const parts = [model.artist, model.album]
    .map((part) => (part ?? "").trim())
    .filter((part) => part !== "");
  return parts.join(" • ");
}

function artworkKey(model: NowPlaying): string | null {
  const artwork = model.artwork;
  if (artwork === null) {
    return null;
  }
  return artwork.kind === "remote"
    ? `remote:${artwork.url}`
    : `local:${model.player}:${artwork.mime}:${artwork.token}`;
}

function controlButton(className: string, command: MediaCommand, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  button.dataset.command = command;
  button.setAttribute("aria-label", label);
  return button;
}

function safeRevoke(url: string): void {
  try {
    URL.revokeObjectURL(url);
  } catch {
    // a revoked or unsupported URL is not worth failing a render over
  }
}

export function createNowPlayingCard(
  container: HTMLElement,
  options: NowPlayingCardOptions,
): NowPlayingCard {
  const root = document.createElement("aside");
  root.className = "now-playing";
  root.hidden = true;
  root.setAttribute("aria-label", "Reprodução atual");

  const body = element("now-playing__body");

  const cover = element("now-playing__cover");
  cover.setAttribute("aria-hidden", "true");

  const artwork = document.createElement("img");
  artwork.className = "now-playing__art";
  artwork.alt = "";
  artwork.decoding = "async";
  artwork.hidden = true;
  cover.append(artwork);

  const text = element("now-playing__text");
  const labelNode = element("now-playing__label", "p");
  const titleNode = element("now-playing__title", "p");
  const metaNode = element("now-playing__meta", "p");
  text.append(labelNode, titleNode, metaNode);

  body.append(cover, text);

  const progress = element("now-playing__progress");
  progress.hidden = true;
  const elapsedNode = element("now-playing__time", "span");
  const totalNode = element("now-playing__time", "span");
  const bar = element("now-playing__bar");
  bar.setAttribute("role", "progressbar");
  bar.setAttribute("aria-valuemin", "0");
  bar.setAttribute("aria-valuemax", "100");
  bar.setAttribute("aria-label", "Progresso da faixa");
  const fill = element("now-playing__fill");
  bar.append(fill);
  progress.append(elapsedNode, bar, totalNode);

  const controls = element("now-playing__controls");
  const previousButton = controlButton(
    "now-playing__button",
    "previous",
    "Faixa anterior",
  );
  const playButton = controlButton("now-playing__button", "play_pause", "Reproduzir");
  const nextButton = controlButton("now-playing__button", "next", "Próxima faixa");
  previousButton.append(createIcon("previous"));
  playButton.append(createIcon("play"));
  nextButton.append(createIcon("next"));
  controls.append(previousButton, playButton, nextButton);

  root.append(body, progress, controls);
  container.append(root);

  let current: NowPlaying | null = null;
  let currentArtworkKey: string | null = null;
  let loadToken = 0;
  let objectUrl: string | null = null;
  let playIcon: IconName = "play";
  let destroyed = false;

  function releaseObjectUrl(): void {
    if (objectUrl !== null) {
      safeRevoke(objectUrl);
      objectUrl = null;
    }
  }

  function resetArtwork(): void {
    artwork.hidden = true;
    artwork.removeAttribute("src");
    releaseObjectUrl();
  }

  artwork.addEventListener("error", () => {
    artwork.hidden = true;
    artwork.removeAttribute("src");
  });

  function loadArtwork(model: NowPlaying): void {
    const token = (loadToken += 1);
    resetArtwork();

    const art = model.artwork;
    if (art === null) {
      return;
    }

    if (art.kind === "remote") {
      artwork.src = art.url;
      artwork.hidden = false;
      return;
    }

    const player = model.player;
    const mime = art.mime;
    void options.loadArtwork(player, mime).then(
      (blob) => {
        if (destroyed || token !== loadToken || !(blob instanceof Blob)) {
          return;
        }
        const url = URL.createObjectURL(blob);
        if (destroyed || token !== loadToken) {
          safeRevoke(url);
          return;
        }
        releaseObjectUrl();
        objectUrl = url;
        artwork.src = url;
        artwork.hidden = false;
      },
      () => {
        if (!destroyed && token === loadToken) {
          resetArtwork();
        }
      },
    );
  }

  function updateArtwork(model: NowPlaying): void {
    const key = artworkKey(model);
    if (key === currentArtworkKey) {
      return;
    }
    currentArtworkKey = key;
    loadArtwork(model);
  }

  function updateProgress(model: NowPlaying): void {
    const value = model.progress;
    if (value === null) {
      progress.hidden = true;
      return;
    }

    progress.hidden = false;
    setText(elapsedNode, value.elapsed);
    setText(totalNode, value.total);

    const ratio =
      value.duration_ms > 0
        ? Math.min(1, Math.max(0, value.elapsed_ms / value.duration_ms))
        : 0;
    const percent = Math.round(ratio * 100);

    fill.style.width = `${percent}%`;
    const now = String(percent);
    if (bar.getAttribute("aria-valuenow") !== now) {
      bar.setAttribute("aria-valuenow", now);
    }
  }

  function updateControls(model: NowPlaying): void {
    previousButton.hidden = !model.can_go_previous;
    playButton.hidden = !model.can_play_pause;
    nextButton.hidden = !model.can_go_next;

    const icon: IconName = model.playing ? "pause" : "play";
    if (icon !== playIcon) {
      playIcon = icon;
      playButton.replaceChildren(createIcon(icon));
    }

    const name = model.playing ? "Pausar" : "Reproduzir";
    if (playButton.getAttribute("aria-label") !== name) {
      playButton.setAttribute("aria-label", name);
    }
  }

  function dispatch(command: MediaCommand): void {
    if (current === null) {
      return;
    }
    options.onCommand(command, current.player);
  }

  previousButton.addEventListener("click", () => {
    dispatch("previous");
  });
  playButton.addEventListener("click", () => {
    dispatch("play_pause");
  });
  nextButton.addEventListener("click", () => {
    dispatch("next");
  });

  return {
    update(status: NowPlayingStatus): void {
      if (destroyed) {
        return;
      }

      const model = status.model;
      current = model;
      root.hidden = !(status.fullscreen && status.enabled && model !== null);
      root.classList.toggle("now-playing--reduced-motion", status.reducedMotion);

      if (model === null) {
        currentArtworkKey = null;
        loadToken += 1;
        resetArtwork();
        progress.hidden = true;
        return;
      }

      setText(labelNode, model.label);
      setText(titleNode, model.title);

      const meta = metaLine(model);
      setText(metaNode, meta);
      metaNode.hidden = meta === "";

      updateArtwork(model);
      updateProgress(model);
      updateControls(model);
    },

    destroy(): void {
      if (destroyed) {
        return;
      }
      destroyed = true;
      loadToken += 1;
      releaseObjectUrl();
      current = null;
      root.remove();
    },
  };
}
