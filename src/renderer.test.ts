import { afterEach, describe, expect, it, vi } from "vitest";
import { renderReading } from "./renderer";
import type { Reading } from "./types";

function reading(overrides: Partial<Reading> = {}): Reading {
  return {
    hour: "09",
    minute: "41",
    second: "05",
    date: "30/10/2026",
    meridiem: null,
    animate: true,
    ...overrides,
  };
}

function mount(): HTMLElement {
  document.body.innerHTML = '<main id="clock" class="clock" data-scale="medium" role="group"></main>';
  const root = document.getElementById("clock");
  if (root === null) {
    throw new Error("clock root is missing");
  }
  return root;
}

function card(root: HTMLElement, name: string): HTMLElement {
  const found = root.querySelector<HTMLElement>(`[data-card="${name}"]`);
  if (found === null) {
    throw new Error(`card ${name} is missing`);
  }
  return found;
}

function face(root: HTMLElement, name: string, half: "upper" | "lower"): string {
  return card(root, name).querySelector(`.card__face--${half} .card__digit`)?.textContent ?? "";
}

function leaf(root: HTMLElement, name: string, half: "upper" | "lower"): string {
  return card(root, name).querySelector(`.card__leaf--${half} .card__digit`)?.textContent ?? "";
}

function flipping(root: HTMLElement, name: string): boolean {
  return card(root, name).classList.contains("card--flipping");
}

function flippedCards(root: HTMLElement): string[] {
  return Array.from(root.querySelectorAll<HTMLElement>(".card--flipping")).map(
    (node) => node.dataset.card ?? "",
  );
}

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

describe("renderReading", () => {
  it("renders the initial faces without a flip", () => {
    const root = mount();
    renderReading(root, reading());

    expect(face(root, "hour-tens", "upper")).toBe("0");
    expect(face(root, "hour-ones", "upper")).toBe("9");
    expect(face(root, "minute-tens", "upper")).toBe("4");
    expect(face(root, "minute-ones", "upper")).toBe("1");
    expect(face(root, "hour-tens", "lower")).toBe("0");
    expect(face(root, "minute-ones", "lower")).toBe("1");
    expect(flippedCards(root)).toEqual([]);
  });

  it("flips only the digits that change on a tick", () => {
    const root = mount();
    renderReading(root, reading({ minute: "41" }));
    const untouched = card(root, "hour-tens").outerHTML;

    renderReading(root, reading({ minute: "42" }));

    expect(flippedCards(root)).toEqual(["minute-ones"]);
    expect(face(root, "minute-ones", "upper")).toBe("2");
    expect(leaf(root, "minute-ones", "upper")).toBe("1");
    expect(leaf(root, "minute-ones", "lower")).toBe("2");
    expect(card(root, "hour-tens").outerHTML).toBe(untouched);
  });

  it("settles the lower face when the flip ends", () => {
    vi.useFakeTimers();
    const root = mount();
    renderReading(root, reading({ minute: "41" }));

    renderReading(root, reading({ minute: "42" }));
    expect(face(root, "minute-ones", "lower")).toBe("1");

    vi.advanceTimersByTime(450);

    expect(face(root, "minute-ones", "lower")).toBe("2");
    expect(flipping(root, "minute-ones")).toBe(false);
  });

  it("updates faces without a flip on a snap", () => {
    const root = mount();
    renderReading(root, reading({ minute: "41" }));

    renderReading(root, reading({ minute: "58", animate: false }));

    expect(flippedCards(root)).toEqual([]);
    expect(face(root, "minute-tens", "upper")).toBe("5");
    expect(face(root, "minute-ones", "upper")).toBe("8");
    expect(face(root, "minute-ones", "lower")).toBe("8");
  });

  it("shows and hides the seconds region", () => {
    const root = mount();
    renderReading(root, reading(), { showSeconds: true });
    const seconds = root.querySelector<HTMLElement>(".clock__seconds");
    expect(seconds?.hidden).toBe(false);
    expect(root.dataset.seconds).toBe("true");
    expect(face(root, "second-tens", "upper")).toBe("0");
    expect(face(root, "second-ones", "upper")).toBe("5");

    renderReading(root, reading(), { showSeconds: false });
    expect(seconds?.hidden).toBe(true);
    expect(root.dataset.seconds).toBe("false");
  });

  it("shows and hides the date line", () => {
    const root = mount();
    renderReading(root, reading(), { showDate: true });
    const date = root.querySelector<HTMLElement>(".clock__date");
    expect(date?.hidden).toBe(false);
    expect(date?.textContent).toBe("30/10/2026");

    renderReading(root, reading(), { showDate: false });
    expect(date?.hidden).toBe(true);
  });

  it("shows the AM/PM label in 12-hour time and hides it in 24-hour time", () => {
    const root = mount();
    renderReading(root, reading({ hour: "07", meridiem: "PM" }));
    const meridiem = root.querySelector<HTMLElement>(".clock__meridiem");
    expect(meridiem?.hidden).toBe(false);
    expect(meridiem?.textContent).toBe("PM");

    renderReading(root, reading({ hour: "19", meridiem: null }));
    expect(meridiem?.hidden).toBe(true);
    expect(meridiem?.textContent).toBe("");
  });

  it("never flips when reduced motion is requested", () => {
    const root = mount();
    renderReading(root, reading({ minute: "41" }));

    renderReading(root, reading({ minute: "42" }), { reducedMotion: true });

    expect(flippedCards(root)).toEqual([]);
    expect(face(root, "minute-ones", "upper")).toBe("2");
    expect(face(root, "minute-ones", "lower")).toBe("2");
    expect(leaf(root, "minute-ones", "upper")).toBe("");
  });

  it("appends the now playing summary to the clock label", () => {
    const root = mount();
    renderReading(root, reading(), { nowPlaying: "CHROME: Everything In Its Right Place" });

    expect(root.getAttribute("aria-label")).toBe(
      "09:41, CHROME: Everything In Its Right Place",
    );
  });

  it("rolls the minute over while the seconds region is hidden", () => {
    const root = mount();
    renderReading(root, reading({ hour: "09", minute: "59", second: "59" }), {
      showSeconds: false,
    });

    renderReading(root, reading({ hour: "10", minute: "00", second: "00" }), {
      showSeconds: false,
    });

    expect(root.querySelector<HTMLElement>(".clock__seconds")?.hidden).toBe(true);
    expect(face(root, "hour-tens", "upper")).toBe("1");
    expect(face(root, "hour-ones", "upper")).toBe("0");
    expect(face(root, "minute-tens", "upper")).toBe("0");
    expect(face(root, "minute-ones", "upper")).toBe("0");
    expect(flippedCards(root)).toEqual([
      "hour-tens",
      "hour-ones",
      "minute-tens",
      "minute-ones",
      "second-tens",
      "second-ones",
    ]);
  });
});
