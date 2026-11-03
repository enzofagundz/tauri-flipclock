import type { Reading } from "./types";

export interface RenderOptions {
  reducedMotion?: boolean;
  showSeconds?: boolean;
  showDate?: boolean;
  nowPlaying?: string;
}

const FLIP_DURATION_MS = 450;

type CardName =
  | "hour-tens"
  | "hour-ones"
  | "minute-tens"
  | "minute-ones"
  | "second-tens"
  | "second-ones";

interface Half {
  element: HTMLElement;
  digit: HTMLElement;
}

interface CardParts {
  root: HTMLElement;
  upperFaceDigit: HTMLElement;
  lowerFaceDigit: HTMLElement;
  upperLeafDigit: HTMLElement;
  lowerLeafDigit: HTMLElement;
}

interface ClockParts {
  seconds: HTMLElement;
  meridiem: HTMLElement;
  date: HTMLElement;
  cards: Record<CardName, CardParts>;
}

interface CardMemory {
  value: string;
  timer: number | undefined;
}

const partsByRoot = new WeakMap<HTMLElement, ClockParts>();
const memoryByCard = new WeakMap<HTMLElement, CardMemory>();

function element(className: string): HTMLElement {
  const node = document.createElement("div");
  node.className = className;
  return node;
}

function half(baseClass: string, modifierClass: string): Half {
  const node = element(`${baseClass} ${modifierClass}`);
  const digit = document.createElement("span");
  digit.className = "card__digit";
  node.append(digit);
  return { element: node, digit };
}

function createCard(name: CardName, modifier?: string): CardParts {
  const root = element(modifier === undefined ? "card" : `card ${modifier}`);
  root.dataset.card = name;
  root.setAttribute("aria-hidden", "true");

  const upperFace = half("card__face", "card__face--upper");
  const lowerFace = half("card__face", "card__face--lower");
  const upperLeaf = half("card__leaf", "card__leaf--upper");
  const lowerLeaf = half("card__leaf", "card__leaf--lower");

  root.append(upperFace.element, lowerFace.element, upperLeaf.element, lowerLeaf.element);

  return {
    root,
    upperFaceDigit: upperFace.digit,
    lowerFaceDigit: lowerFace.digit,
    upperLeafDigit: upperLeaf.digit,
    lowerLeafDigit: lowerLeaf.digit,
  };
}

function createPair(): HTMLElement {
  return element("clock__pair");
}

function createColon(): HTMLElement {
  const colon = element("clock__colon");
  colon.setAttribute("aria-hidden", "true");
  colon.append(element("clock__dot"), element("clock__dot"));
  return colon;
}

function buildClock(root: HTMLElement): ClockParts {
  const cards = {
    "hour-tens": createCard("hour-tens"),
    "hour-ones": createCard("hour-ones"),
    "minute-tens": createCard("minute-tens"),
    "minute-ones": createCard("minute-ones"),
    "second-tens": createCard("second-tens", "card--seconds"),
    "second-ones": createCard("second-ones", "card--seconds"),
  } satisfies Record<CardName, CardParts>;

  const hours = createPair();
  hours.append(cards["hour-tens"].root, cards["hour-ones"].root);

  const minutes = createPair();
  minutes.append(cards["minute-tens"].root, cards["minute-ones"].root);

  const secondsPair = createPair();
  secondsPair.className = "clock__pair clock__pair--seconds";
  secondsPair.append(cards["second-tens"].root, cards["second-ones"].root);

  const seconds = element("clock__seconds");
  seconds.append(createColon(), secondsPair);

  const display = element("clock__display");
  display.append(hours, createColon(), minutes, seconds);

  const meridiem = element("clock__meridiem");
  const date = element("clock__date");

  root.classList.add("clock");
  root.replaceChildren(display, meridiem, date);

  return { seconds, meridiem, date, cards };
}

function getParts(root: HTMLElement): ClockParts {
  const existing = partsByRoot.get(root);
  if (existing !== undefined) {
    return existing;
  }
  const parts = buildClock(root);
  partsByRoot.set(root, parts);
  return parts;
}

function setText(node: HTMLElement, value: string): void {
  if (node.textContent !== value) {
    node.textContent = value;
  }
}

function setHidden(node: HTMLElement, hidden: boolean): void {
  if (node.hidden !== hidden) {
    node.hidden = hidden;
  }
}

function settle(parts: CardParts, memory: CardMemory): void {
  if (memory.timer !== undefined) {
    window.clearTimeout(memory.timer);
    memory.timer = undefined;
  }
  setText(parts.lowerFaceDigit, memory.value);
  parts.root.classList.remove("card--flipping");
}

function updateCard(parts: CardParts, next: string, animate: boolean): void {
  const existing = memoryByCard.get(parts.root);
  const previous = existing?.value ?? "";
  if (previous === next) {
    return;
  }

  const memory: CardMemory = existing ?? { value: previous, timer: undefined };
  memoryByCard.set(parts.root, memory);
  if (existing !== undefined) {
    settle(parts, existing);
  }

  memory.value = next;
  setText(parts.upperFaceDigit, next);

  const canAnimate = animate && previous !== "";
  if (!canAnimate) {
    setText(parts.lowerFaceDigit, next);
    parts.root.classList.remove("card--flipping");
    return;
  }

  setText(parts.upperLeafDigit, previous);
  setText(parts.lowerLeafDigit, next);
  parts.root.classList.remove("card--flipping");
  void parts.root.getBoundingClientRect();
  parts.root.classList.add("card--flipping");

  memory.timer = window.setTimeout(() => {
    memory.timer = undefined;
    setText(parts.lowerFaceDigit, next);
    parts.root.classList.remove("card--flipping");
  }, FLIP_DURATION_MS);
}

export function renderReading(
  root: HTMLElement,
  reading: Reading,
  options: RenderOptions = {},
): void {
  const parts = getParts(root);
  const animate = reading.animate && options.reducedMotion !== true;
  const showSeconds = options.showSeconds === true;
  const showDate = options.showDate === true;

  updateCard(parts.cards["hour-tens"], reading.hour.charAt(0), animate);
  updateCard(parts.cards["hour-ones"], reading.hour.charAt(1), animate);
  updateCard(parts.cards["minute-tens"], reading.minute.charAt(0), animate);
  updateCard(parts.cards["minute-ones"], reading.minute.charAt(1), animate);
  updateCard(parts.cards["second-tens"], reading.second.charAt(0), animate);
  updateCard(parts.cards["second-ones"], reading.second.charAt(1), animate);

  setHidden(parts.seconds, !showSeconds);
  setHidden(parts.date, !showDate);
  root.dataset.seconds = showSeconds ? "true" : "false";

  if (showDate) {
    setText(parts.date, reading.date);
  }

  if (reading.meridiem === null) {
    setText(parts.meridiem, "");
    setHidden(parts.meridiem, true);
  } else {
    setText(parts.meridiem, reading.meridiem);
    setHidden(parts.meridiem, false);
  }

  const time = showSeconds
    ? `${reading.hour}:${reading.minute}:${reading.second}`
    : `${reading.hour}:${reading.minute}`;
  const suffix = reading.meridiem === null ? "" : ` ${reading.meridiem}`;
  const date = showDate ? `, ${reading.date}` : "";
  const media = options.nowPlaying?.trim() ?? "";
  const nowPlaying = media === "" ? "" : `, ${media}`;
  const label = `${time}${suffix}${date}${nowPlaying}`;
  if (root.getAttribute("aria-label") !== label) {
    root.setAttribute("aria-label", label);
  }
}
