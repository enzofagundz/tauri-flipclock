import type { Corner, SaveResult, Scale, Settings } from "./types";

export interface SettingsPanelOptions {
  layerShellAvailable: boolean;
  onChange: (settings: Settings) => Promise<SaveResult>;
}

export interface SettingsPanel {
  open(settings: Settings): void;
  close(): void;
  isOpen(): boolean;
  showRestartNotice(): void;
}

const CORNERS: ReadonlyArray<{ value: Corner; label: string }> = [
  { value: "top_left", label: "Superior esquerdo" },
  { value: "top_right", label: "Superior direito" },
  { value: "bottom_left", label: "Inferior esquerdo" },
  { value: "bottom_right", label: "Inferior direito" },
];

const SCALES: ReadonlyArray<{ value: Scale; label: string }> = [
  { value: "small", label: "Pequeno" },
  { value: "medium", label: "Médio" },
  { value: "large", label: "Grande" },
];

const RESTART_NOTICE = "Reinicie o Flipclock para aplicar esta alteração.";
const SAVE_ERROR_NOTICE = "Não foi possível salvar as configurações.";
const LAYER_SHELL_NOTICE =
  "O modo fixado não está disponível neste ambiente. O relógio abre em modo janela.";

type ToggleSetting = "format_24h" | "show_seconds" | "show_date" | "pinned";

interface ToggleField {
  field: HTMLLabelElement;
  input: HTMLInputElement;
}

function notice(): HTMLParagraphElement {
  const node = document.createElement("p");
  node.className = "settings__notice";
  node.hidden = true;
  return node;
}

function toggleField(label: string, setting: ToggleSetting): ToggleField {
  const field = document.createElement("label");
  field.className = "field field--toggle";

  const text = document.createElement("span");
  text.className = "field__label";
  text.textContent = label;

  const input = document.createElement("input");
  input.type = "checkbox";
  input.dataset.setting = setting;

  field.append(text, input);
  return { field, input };
}

interface RadioGroup {
  legend: string;
  name: string;
  setting: "corner" | "scale";
  entries: ReadonlyArray<{ value: Corner | Scale; label: string }>;
}

function radioGroup(group: RadioGroup): { fieldset: HTMLFieldSetElement; inputs: HTMLInputElement[] } {
  const fieldset = document.createElement("fieldset");
  fieldset.className = "field-group";

  const legend = document.createElement("legend");
  legend.textContent = group.legend;
  fieldset.append(legend);

  const inputs: HTMLInputElement[] = [];
  for (const entry of group.entries) {
    const label = document.createElement("label");
    label.className = "field field--radio";

    const input = document.createElement("input");
    input.type = "radio";
    input.name = group.name;
    input.value = entry.value;
    input.dataset.setting = group.setting;
    inputs.push(input);

    const text = document.createElement("span");
    text.className = "field__label";
    text.textContent = entry.label;

    label.append(input, text);
    fieldset.append(label);
  }

  return { fieldset, inputs };
}

export function createSettingsPanel(options: SettingsPanelOptions): SettingsPanel {
  let current: Settings | null = null;
  let saved: Settings | null = null;

  const overlay = document.createElement("div");
  overlay.className = "settings";
  overlay.hidden = true;

  const dialog = document.createElement("div");
  dialog.className = "settings__card";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-labelledby", "settings-title");

  const head = document.createElement("div");
  head.className = "settings__head";

  const title = document.createElement("h2");
  title.className = "settings__title";
  title.id = "settings-title";
  title.textContent = "Configurações";

  const closeButton = document.createElement("button");
  closeButton.type = "button";
  closeButton.className = "settings__close";
  closeButton.textContent = "Fechar";
  closeButton.setAttribute("aria-label", "Fechar configurações");

  head.append(title, closeButton);

  const fields = document.createElement("div");
  fields.className = "settings__fields";

  const formatField = toggleField("Formato de 24 horas", "format_24h");
  const secondsField = toggleField("Mostrar segundos", "show_seconds");
  const dateField = toggleField("Mostrar data", "show_date");
  const pinnedField = toggleField("Fixar na tela", "pinned");

  const layerNotice = notice();
  layerNotice.textContent = LAYER_SHELL_NOTICE;
  layerNotice.hidden = options.layerShellAvailable;

  const pinnedFieldGroup = document.createElement("div");
  pinnedFieldGroup.className = "settings__group";
  pinnedFieldGroup.append(pinnedField.field, layerNotice);

  const cornerGroup = radioGroup({
    legend: "Canto de ancoragem",
    name: "corner",
    setting: "corner",
    entries: CORNERS,
  });

  const scaleGroup = radioGroup({
    legend: "Tamanho",
    name: "scale",
    setting: "scale",
    entries: SCALES,
  });

  fields.append(
    formatField.field,
    secondsField.field,
    dateField.field,
    pinnedFieldGroup,
    cornerGroup.fieldset,
    scaleGroup.fieldset,
  );

  const restartNotice = notice();
  restartNotice.setAttribute("role", "status");
  restartNotice.textContent = RESTART_NOTICE;

  const saveErrorNotice = notice();
  saveErrorNotice.setAttribute("role", "status");
  saveErrorNotice.textContent = SAVE_ERROR_NOTICE;

  const notices = document.createElement("div");
  notices.className = "settings__notices";
  notices.append(restartNotice, saveErrorNotice);

  dialog.append(head, fields, notices);
  overlay.append(dialog);
  document.body.append(overlay);

  if (!options.layerShellAvailable) {
    pinnedField.input.disabled = true;
  }

  function syncControls(settings: Settings): void {
    formatField.input.checked = settings.format_24h;
    secondsField.input.checked = settings.show_seconds;
    dateField.input.checked = settings.show_date;
    pinnedField.input.checked = settings.pinned;

    for (const input of cornerGroup.inputs) {
      input.checked = input.value === settings.corner;
    }
    for (const input of scaleGroup.inputs) {
      input.checked = input.value === settings.scale;
    }
  }

  function commit(next: Settings): void {
    current = next;
    restartNotice.hidden = true;
    saveErrorNotice.hidden = true;
    void options.onChange(next).then(
      (result) => {
        saved = result.settings;
        current = result.settings;
        syncControls(result.settings);
        restartNotice.hidden = !result.restart_required;
      },
      () => {
        if (saved !== null) {
          current = saved;
          syncControls(saved);
        }
        saveErrorNotice.hidden = false;
      },
    );
  }

  function fieldsetInputs(inputs: HTMLInputElement[]): void {
    for (const input of inputs) {
      input.addEventListener("change", () => {
        if (!input.checked || current === null) {
          return;
        }
        if (input.dataset.setting === "corner") {
          commit({ ...current, corner: input.value as Corner });
        } else {
          commit({ ...current, scale: input.value as Scale });
        }
      });
    }
  }

  formatField.input.addEventListener("change", () => {
    if (current !== null) {
      commit({ ...current, format_24h: formatField.input.checked });
    }
  });

  secondsField.input.addEventListener("change", () => {
    if (current !== null) {
      commit({ ...current, show_seconds: secondsField.input.checked });
    }
  });

  dateField.input.addEventListener("change", () => {
    if (current !== null) {
      commit({ ...current, show_date: dateField.input.checked });
    }
  });

  if (options.layerShellAvailable) {
    pinnedField.input.addEventListener("change", () => {
      if (current !== null) {
        commit({ ...current, pinned: pinnedField.input.checked });
      }
    });
  }

  fieldsetInputs(cornerGroup.inputs);
  fieldsetInputs(scaleGroup.inputs);

  function close(): void {
    overlay.hidden = true;
    (document.activeElement as HTMLElement | null)?.blur();
  }

  closeButton.addEventListener("click", close);
  overlay.addEventListener("click", (event) => {
    if (event.target === overlay) {
      close();
    }
  });

  return {
    open(settings: Settings): void {
      saved = settings;
      current = settings;
      syncControls(settings);

      restartNotice.hidden = true;
      saveErrorNotice.hidden = true;
      overlay.hidden = false;

      dialog.querySelector<HTMLInputElement>("input:not(:disabled)")?.focus();
    },
    close,
    isOpen(): boolean {
      return !overlay.hidden;
    },
    showRestartNotice(): void {
      restartNotice.hidden = false;
    },
  };
}
