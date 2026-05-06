const BASIC_COLORS = [
  "#ff7777", "#ffff77", "#77ff77", "#00e878", "#77e6e6", "#006bd6", "#f46bb4", "#ee66ee",
  "#ff0000", "#ffff00", "#66ff00", "#00ff3b", "#1fd6d6", "#0b75a8", "#ff00dd", "#ff0090",
  "#8b3d3d", "#ff7438", "#00e800", "#007a68", "#004b88", "#7a7de0", "#820047", "#f20073",
  "#900000", "#ff7900", "#007000", "#007748", "#0000ff", "#00007d", "#800080", "#7500ff",
  "#4b0000", "#8a4b00", "#004b00", "#004b4b", "#000075", "#00004b", "#3d003d", "#310075",
  "#000000", "#808000", "#808040", "#808080", "#408080", "#c0c0c0", "#3a003a", "#ffffff",
];

const CURATED_CUSTOM_COLORS = [
  "#111827", "#374151", "#6b7280", "#9ca3af",
  "#e5e7eb", "#f8fafc", "#334155", "#0f172a",
  "#0f766e", "#0e7490", "#2563eb", "#4f46e5",
  "#7c3aed", "#be185d", "#dc2626", "#ea580c",
];

class ChemcoreColorHost {
  constructor(root = document.body) {
    this.kind = "chemcore";
    this.root = root;
  }

  chooseColor(initialColor, customColors = []) {
    const existing = document.querySelector(".color-dialog-backdrop");
    existing?.querySelector(".color-dialog-close")?.click();
    return new Promise((resolve) => {
      const dialog = new ChemcoreColorDialog({
        root: this.root,
        initialColor,
        customColors,
        resolve,
      });
      dialog.open();
    });
  }
}

class ChemcoreColorDialog {
  constructor({ root, initialColor, customColors, resolve }) {
    this.root = root;
    this.resolve = resolve;
    this.selected = normalizeHexColor(initialColor) || "#000000";
    this.hsv = rgbToHsv(hexToRgb(this.selected));
    this.customColors = normalizeCustomColors(customColors);
    this.backdrop = null;
  }

  open() {
    this.backdrop = document.createElement("div");
    this.backdrop.className = "color-dialog-backdrop";
    this.backdrop.innerHTML = this.html();
    this.root.appendChild(this.backdrop);
    this.bind();
    this.sync(this.selected, this.hsv);
    this.backdrop.tabIndex = -1;
    this.backdrop.focus();
  }

  html() {
    return `
      <div class="color-dialog" role="dialog" aria-modal="true" aria-label="Color">
        <div class="color-dialog-titlebar">
          <span>颜色</span>
          <button class="color-dialog-close" type="button" aria-label="Close">×</button>
        </div>
        <div class="color-dialog-body">
          <section class="color-dialog-palette">
            <p class="color-dialog-label">基本颜色(B):</p>
            <div class="color-dialog-basic-grid">
              ${BASIC_COLORS.map((color) => colorChipHtml(color, this.selected)).join("")}
            </div>
            <div class="color-dialog-custom">
              <p class="color-dialog-label">自定义颜色(C):</p>
              <div class="color-dialog-custom-grid"></div>
            </div>
          </section>
          <section class="color-dialog-main">
            <div class="color-dialog-picker">
              <div class="color-dialog-spectrum" data-color-spectrum aria-label="Color spectrum" role="slider" tabindex="0">
                <span class="color-dialog-spectrum-cursor"></span>
              </div>
              <div class="color-dialog-value-slider" data-color-value-slider aria-label="Brightness" role="slider" tabindex="0">
                <span class="color-dialog-value-cursor"></span>
              </div>
            </div>
            <div class="color-dialog-bottom">
              <div class="color-dialog-preview-block">
                <div class="color-dialog-preview"></div>
                <span>颜色|纯色(O)</span>
              </div>
              <div class="color-dialog-fields">
                <label class="color-dialog-field"><span>色调(E):</span><input data-hsv-field="h" type="number" min="0" max="359"></label>
                <label class="color-dialog-field"><span>红(R):</span><input data-rgb-field="r" type="number" min="0" max="255"></label>
                <label class="color-dialog-field"><span>饱和度(S):</span><input data-hsv-field="s" type="number" min="0" max="100"></label>
                <label class="color-dialog-field"><span>绿(G):</span><input data-rgb-field="g" type="number" min="0" max="255"></label>
                <label class="color-dialog-field"><span>亮度(L):</span><input data-hsv-field="v" type="number" min="0" max="100"></label>
                <label class="color-dialog-field"><span>蓝(U):</span><input data-rgb-field="b" type="number" min="0" max="255"></label>
                <label class="color-dialog-field color-dialog-hex-field"><span>Hex:</span><input data-color-field="hex"></label>
              </div>
            </div>
            <div class="color-dialog-add-row">
              <button type="button" data-color-dialog-add-custom>添加到自定义颜色(A)</button>
            </div>
            <div class="color-dialog-actions">
              <button type="button" data-color-dialog-ok>确定</button>
              <button type="button" data-color-dialog-cancel>取消</button>
            </div>
          </section>
        </div>
      </div>
    `;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target === this.backdrop || event.target.closest("[data-color-dialog-cancel]") || event.target.closest(".color-dialog-close")) {
        this.close(null);
        return;
      }
      const chip = event.target.closest("[data-color-dialog-value]");
      if (chip) {
        this.sync(chip.dataset.colorDialogValue);
        return;
      }
      if (event.target.closest("[data-color-dialog-add-custom]")) {
        this.customColors = [
          this.selected,
          ...this.customColors.filter((color) => color !== this.selected),
        ].slice(0, 16);
        this.renderCustomColors();
        this.sync(this.selected, this.hsv);
        return;
      }
      if (event.target.closest("[data-color-dialog-ok]")) {
        this.close(this.selected);
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(null);
      }
    });
    this.bindSpectrumDrag();
    this.bindFields();
  }

  bindSpectrumDrag() {
    const spectrum = this.backdrop.querySelector("[data-color-spectrum]");
    const valueSlider = this.backdrop.querySelector("[data-color-value-slider]");
    const bindDrag = (target, update) => {
      target.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        target.setPointerCapture?.(event.pointerId);
        update(event);
        const move = (moveEvent) => update(moveEvent);
        const up = () => {
          window.removeEventListener("pointermove", move);
          window.removeEventListener("pointerup", up);
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up, { once: true });
      });
    };
    bindDrag(spectrum, (event) => {
      const rect = spectrum.getBoundingClientRect();
      this.syncFromHsv({
        h: ((event.clientX - rect.left) / rect.width) * 359,
        s: (1 - ((event.clientY - rect.top) / rect.height)) * 100,
      });
    });
    bindDrag(valueSlider, (event) => {
      const rect = valueSlider.getBoundingClientRect();
      this.syncFromHsv({ v: (1 - ((event.clientY - rect.top) / rect.height)) * 100 });
    });
  }

  bindFields() {
    const hexInput = this.backdrop.querySelector('[data-color-field="hex"]');
    const rgbInputs = Array.from(this.backdrop.querySelectorAll("[data-rgb-field]"));
    const hsvInputs = Array.from(this.backdrop.querySelectorAll("[data-hsv-field]"));
    hexInput.addEventListener("change", () => this.sync(hexInput.value));
    for (const input of rgbInputs) {
      input.addEventListener("change", () => {
        const values = Object.fromEntries(rgbInputs.map((field) => [
          field.dataset.rgbField,
          clampRgb(field.value),
        ]));
        this.sync(rgbToHex(values.r, values.g, values.b));
      });
    }
    for (const input of hsvInputs) {
      input.addEventListener("change", () => {
        const values = Object.fromEntries(hsvInputs.map((field) => [
          field.dataset.hsvField,
          Number.parseInt(String(field.value || 0), 10) || 0,
        ]));
        this.syncFromHsv({ h: values.h, s: values.s, v: values.v });
      });
    }
  }

  sync(color, nextHsv = null) {
    this.selected = normalizeHexColor(color) || this.selected;
    this.hsv = nextHsv ? {
      h: clampHue(nextHsv.h),
      s: clampPercent(nextHsv.s),
      v: clampPercent(nextHsv.v),
    } : rgbToHsv(hexToRgb(this.selected));
    this.backdrop.style.setProperty("--dialog-hue-position", `${(this.hsv.h / 359) * 100}%`);
    this.backdrop.style.setProperty("--dialog-saturation", `${this.hsv.s}%`);
    this.backdrop.style.setProperty("--dialog-value", `${this.hsv.v}%`);
    this.backdrop.querySelector(".color-dialog-preview").style.setProperty("--swatch", this.selected);
    this.backdrop.querySelector('[data-color-field="hex"]').value = this.selected.toUpperCase();
    const { r, g, b } = hexToRgb(this.selected);
    for (const input of this.backdrop.querySelectorAll("[data-rgb-field]")) {
      input.value = String({ r, g, b }[input.dataset.rgbField]);
    }
    for (const input of this.backdrop.querySelectorAll("[data-hsv-field]")) {
      input.value = String({
        h: Math.round(this.hsv.h),
        s: Math.round(this.hsv.s),
        v: Math.round(this.hsv.v),
      }[input.dataset.hsvField]);
    }
    this.backdrop.querySelectorAll(".color-dialog-chip").forEach((chip) => {
      chip.classList.toggle("is-selected", normalizeHexColor(chip.dataset.colorDialogValue) === this.selected);
    });
    this.renderCustomColors();
  }

  syncFromHsv(next) {
    this.hsv = {
      h: clampHue(next.h ?? this.hsv.h),
      s: clampPercent(next.s ?? this.hsv.s),
      v: clampPercent(next.v ?? this.hsv.v),
    };
    const { r, g, b } = hsvToRgb(this.hsv);
    this.sync(rgbToHex(r, g, b), this.hsv);
  }

  renderCustomColors() {
    const customGrid = this.backdrop.querySelector(".color-dialog-custom-grid");
    customGrid.innerHTML = this.customColors
      .map((color) => colorChipHtml(color, this.selected))
      .join("");
  }

  close(color) {
    this.backdrop?.remove();
    this.resolve(color);
  }
}

export function createColorHost() {
  return new ChemcoreColorHost();
}

function colorChipHtml(color, selected) {
  return `<button class="color-dialog-chip${normalizeHexColor(color) === normalizeHexColor(selected) ? " is-selected" : ""}" type="button" data-color-dialog-value="${color}" style="--swatch:${color}" aria-label="${color}"></button>`;
}

function normalizeCustomColors(colors) {
  const merged = [...colors, ...CURATED_CUSTOM_COLORS]
    .map(normalizeHexColor)
    .filter(Boolean);
  return merged.filter((color, index) => merged.indexOf(color) === index).slice(0, 16);
}

function normalizeHexColor(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  const match = raw.match(/^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/);
  if (match) {
    return rgbToHex(match[1], match[2], match[3]);
  }
  return null;
}

function hexToRgb(color) {
  const hex = normalizeHexColor(color) || "#000000";
  return {
    r: Number.parseInt(hex.slice(1, 3), 16),
    g: Number.parseInt(hex.slice(3, 5), 16),
    b: Number.parseInt(hex.slice(5, 7), 16),
  };
}

function rgbToHex(r, g, b) {
  return `#${[r, g, b].map((value) => clampRgb(value).toString(16).padStart(2, "0")).join("")}`;
}

function clampRgb(value) {
  return Math.max(0, Math.min(255, Number.parseInt(String(value || 0), 10) || 0));
}

function rgbToHsv({ r, g, b }) {
  const red = clampRgb(r) / 255;
  const green = clampRgb(g) / 255;
  const blue = clampRgb(b) / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  let h = 0;
  if (delta !== 0) {
    if (max === red) {
      h = 60 * (((green - blue) / delta) % 6);
    } else if (max === green) {
      h = 60 * ((blue - red) / delta + 2);
    } else {
      h = 60 * ((red - green) / delta + 4);
    }
  }
  if (h < 0) {
    h += 360;
  }
  return {
    h,
    s: max === 0 ? 0 : (delta / max) * 100,
    v: max * 100,
  };
}

function hsvToRgb({ h, s, v }) {
  const hue = clampHue(h);
  const saturation = clampPercent(s) / 100;
  const value = clampPercent(v) / 100;
  const chroma = value * saturation;
  const x = chroma * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = value - chroma;
  let red = 0;
  let green = 0;
  let blue = 0;
  if (hue < 60) {
    [red, green, blue] = [chroma, x, 0];
  } else if (hue < 120) {
    [red, green, blue] = [x, chroma, 0];
  } else if (hue < 180) {
    [red, green, blue] = [0, chroma, x];
  } else if (hue < 240) {
    [red, green, blue] = [0, x, chroma];
  } else if (hue < 300) {
    [red, green, blue] = [x, 0, chroma];
  } else {
    [red, green, blue] = [chroma, 0, x];
  }
  return {
    r: Math.round((red + m) * 255),
    g: Math.round((green + m) * 255),
    b: Math.round((blue + m) * 255),
  };
}

function clampHue(value) {
  const hue = Number.parseFloat(String(value || 0));
  return ((Number.isFinite(hue) ? hue : 0) % 360 + 360) % 360;
}

function clampPercent(value) {
  const percent = Number.parseFloat(String(value || 0));
  return Math.max(0, Math.min(100, Number.isFinite(percent) ? percent : 0));
}
