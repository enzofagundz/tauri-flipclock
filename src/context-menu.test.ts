import { beforeEach, describe, expect, it, vi } from "vitest";
import { createContextMenu } from "./context-menu";

interface Handlers {
  onSettings: ReturnType<typeof vi.fn>;
  onTogglePin: ReturnType<typeof vi.fn>;
  onToggleFullscreen: ReturnType<typeof vi.fn>;
  onQuit: ReturnType<typeof vi.fn>;
}

function setup(canPin = true): { menu: ReturnType<typeof createContextMenu>; handlers: Handlers } {
  const handlers: Handlers = {
    onSettings: vi.fn(),
    onTogglePin: vi.fn(),
    onToggleFullscreen: vi.fn(),
    onQuit: vi.fn(),
  };
  return { menu: createContextMenu({ canPin, ...handlers }), handlers };
}

function menuElement(): HTMLElement {
  const found = document.querySelector<HTMLElement>(".menu");
  if (found === null) {
    throw new Error("menu is missing");
  }
  return found;
}

function item(label: string): HTMLButtonElement {
  const found = Array.from(document.querySelectorAll<HTMLButtonElement>(".menu__item")).find(
    (node) => node.textContent === label,
  );
  if (found === undefined) {
    throw new Error(`menu item ${label} is missing`);
  }
  return found;
}

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("createContextMenu", () => {
  it("opens at the pointer position", () => {
    const { menu } = setup();

    menu.open(24, 32);

    expect(menu.isOpen()).toBe(true);
    expect(menuElement().hidden).toBe(false);
    expect(menuElement().style.left).toBe("24px");
    expect(menuElement().style.top).toBe("32px");
    expect(menuElement().querySelectorAll('[role="menuitem"]')).toHaveLength(4);
  });

  it("dispatches the settings action and closes", () => {
    const { menu, handlers } = setup();
    menu.open(10, 10);

    item("Configurações").click();

    expect(handlers.onSettings).toHaveBeenCalledOnce();
    expect(menu.isOpen()).toBe(false);
    expect(menuElement().hidden).toBe(true);
  });

  it("dispatches the quit action", () => {
    const { menu, handlers } = setup();
    menu.open(10, 10);

    item("Sair").click();

    expect(handlers.onQuit).toHaveBeenCalledOnce();
    expect(menu.isOpen()).toBe(false);
  });

  it("reflects pin and fullscreen state in the labels", () => {
    const { menu, handlers } = setup();
    menu.open(10, 10);

    menu.setPinned(true);
    expect(item("Soltar")).not.toBeUndefined();
    item("Soltar").click();
    expect(handlers.onTogglePin).toHaveBeenCalledOnce();

    menu.open(10, 10);
    menu.setFullscreen(true);
    item("Sair da tela cheia").click();
    expect(handlers.onToggleFullscreen).toHaveBeenCalledOnce();
  });

  it("does not offer pinning when layer shell is unavailable", () => {
    const { menu, handlers } = setup(false);
    menu.open(10, 10);

    const pin = item("Fixar (indisponível)");
    expect(pin.disabled).toBe(true);

    pin.click();
    expect(handlers.onTogglePin).not.toHaveBeenCalled();

    menu.setPinned(true);
    expect(pin.textContent).toBe("Fixar (indisponível)");
  });

  it("closes on an outside pointer press", () => {
    const { menu } = setup();
    menu.open(10, 10);

    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));

    expect(menu.isOpen()).toBe(false);
  });

  it("closes on Escape and consumes the key", () => {
    const { menu } = setup();
    menu.open(10, 10);

    const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    document.dispatchEvent(event);

    expect(menu.isOpen()).toBe(false);
    expect(event.defaultPrevented).toBe(true);
  });

  it("keeps the pointer press inside the menu open", () => {
    const { menu, handlers } = setup();
    menu.open(10, 10);

    item("Configurações").dispatchEvent(new Event("pointerdown", { bubbles: true }));

    expect(menu.isOpen()).toBe(true);
    expect(handlers.onSettings).not.toHaveBeenCalled();
  });
});
