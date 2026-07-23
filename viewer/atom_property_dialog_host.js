export function createAtomPropertyDialogHost({ root = document.body, engine } = {}) {
  return {
    async choose(property) {
      const targetEngine = engine?.();
      if (!targetEngine?.atomPropertyDialogJson) {
        return null;
      }
      const payload = JSON.parse(await targetEngine.atomPropertyDialogJson(property));
      if (!payload?.field || payload.property !== property) {
        return null;
      }
      return new AtomPropertyDialog({ root, payload }).open();
    },
  };
}

class AtomPropertyDialog {
  constructor({ root, payload }) {
    this.root = root;
    this.payload = payload || {};
  }

  open() {
    document.querySelector(".atom-property-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog atom-property-dialog";
      const field = this.payload.field || {};
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-atom-property-dialog-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.payload.title || "Atom Property")}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.payload.title || "Atom Property")}</div>
          <label class="numeric-dialog-field">
            <span>${escapeHtml(field.label || "Value")}</span>
            <input name="value" type="text" inputmode="${escapeHtml(field.inputMode || "text")}" value="${escapeHtml(field.value || "")}">
            <em></em>
          </label>
          <div class="numeric-dialog-actions">
            <button type="button" data-atom-property-dialog-close>Cancel</button>
            <button type="submit">Apply</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
      const input = this.backdrop.querySelector("input");
      input?.focus?.({ preventScroll: true });
      input?.select?.();
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-atom-property-dialog-close]")) {
        this.close(null);
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(null);
      }
    });
    this.backdrop.querySelector("form")?.addEventListener("submit", (event) => {
      event.preventDefault();
      this.submit();
    });
  }

  submit() {
    const input = this.backdrop.querySelector("input");
    const value = String(input?.value || "").trim();
    if (!this.isValid(value)) {
      input?.classList.add("is-invalid");
      input?.focus?.();
      return;
    }
    this.close(value);
  }

  isValid(value) {
    const field = this.payload.field || {};
    if (!value) {
      return field.allowEmpty !== false;
    }
    if (field.valueKind !== "integer" || !/^\d+$/.test(value)) {
      return field.valueKind !== "integer";
    }
    const number = Number(value);
    return Number.isSafeInteger(number)
      && (field.minimum == null || number >= Number(field.minimum))
      && (field.maximum == null || number <= Number(field.maximum));
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
