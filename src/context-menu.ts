export interface ContextMenuOptions {
  canPin: boolean;
  onSettings(): void;
  onTogglePin(): void;
  onToggleFullscreen(): void;
  onQuit(): void;
}

export interface ContextMenu {
  open(x: number, y: number): void;
  close(): void;
  isOpen(): boolean;
  setPinned(pinned: boolean): void;
  setFullscreen(fullscreen: boolean): void;
}

type Action = "settings" | "pin" | "fullscreen" | "quit";

const EDGE_MARGIN = 8;
const PIN_UNAVAILABLE_LABEL = "Fixar (indisponível)";

export function createContextMenu(options: ContextMenuOptions): ContextMenu {
  let visible = false;

  const element = document.createElement("div");
  element.className = "menu";
  element.hidden = true;
  element.setAttribute("role", "menu");

  const press = (event: PointerEvent): void => {
    if (event.target instanceof Node && element.contains(event.target)) {
      return;
    }
    close();
  };

  const key = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") {
      return;
    }
    event.preventDefault();
    close();
  };

  function dispatch(action: Action): void {
    switch (action) {
      case "settings":
        options.onSettings();
        break;
      case "pin":
        options.onTogglePin();
        break;
      case "fullscreen":
        options.onToggleFullscreen();
        break;
      case "quit":
        options.onQuit();
        break;
    }
  }

  function item(label: string, action: Action): HTMLButtonElement {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "menu__item";
    button.setAttribute("role", "menuitem");
    button.dataset.action = action;
    button.textContent = label;
    button.addEventListener("click", () => {
      close();
      dispatch(action);
    });
    return button;
  }

  const settingsItem = item("Configurações", "settings");
  const pinItem = item("Fixar", "pin");
  const fullscreenItem = item("Tela cheia", "fullscreen");
  const quitItem = item("Sair", "quit");

  if (!options.canPin) {
    pinItem.disabled = true;
    pinItem.textContent = PIN_UNAVAILABLE_LABEL;
    pinItem.title = "O modo fixado não está disponível neste ambiente.";
  }

  element.append(settingsItem, pinItem, fullscreenItem, quitItem);
  document.body.append(element);

  function close(): void {
    if (!visible) {
      return;
    }
    visible = false;
    element.hidden = true;
    document.removeEventListener("pointerdown", press);
    document.removeEventListener("keydown", key);
  }

  function open(x: number, y: number): void {
    element.hidden = false;
    visible = true;
    element.style.left = `${x}px`;
    element.style.top = `${y}px`;

    const rect = element.getBoundingClientRect();
    const width = rect.width === 0 ? 168 : rect.width;
    const height = rect.height === 0 ? 160 : rect.height;
    const maxLeft = Math.max(EDGE_MARGIN, window.innerWidth - width - EDGE_MARGIN);
    const maxTop = Math.max(EDGE_MARGIN, window.innerHeight - height - EDGE_MARGIN);

    element.style.left = `${Math.min(Math.max(EDGE_MARGIN, x), maxLeft)}px`;
    element.style.top = `${Math.min(Math.max(EDGE_MARGIN, y), maxTop)}px`;

    document.addEventListener("pointerdown", press);
    document.addEventListener("keydown", key);
    settingsItem.focus();
  }

  return {
    open,
    close,
    isOpen: () => visible,
    setPinned(pinned: boolean): void {
      if (!options.canPin) {
        return;
      }
      pinItem.textContent = pinned ? "Soltar" : "Fixar";
    },
    setFullscreen(fullscreen: boolean): void {
      fullscreenItem.textContent = fullscreen ? "Sair da tela cheia" : "Tela cheia";
    },
  };
}
