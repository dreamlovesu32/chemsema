export function createSmilesDialogHost({ root = document.body, commandEngine, onApply } = {}) {
  return {
    open(point) {
      document.querySelector(".smiles-dialog")?.remove();
      return new Promise((resolve) => {
        const backdrop = document.createElement("div");
        backdrop.className = "smiles-dialog";
        backdrop.innerHTML = [
          '<div class="smiles-dialog-backdrop" data-smiles-dialog-close></div>',
          '<form class="smiles-dialog-panel" aria-label="Generate structure from SMILES">',
          '<div class="smiles-dialog-title">Generate Structure from SMILES</div>',
          '<label class="smiles-dialog-field"><span>SMILES</span>',
          '<textarea name="smiles" rows="4" spellcheck="false" autocomplete="off"></textarea></label>',
          '<div class="smiles-dialog-error" role="alert" aria-live="polite"></div>',
          '<div class="smiles-dialog-actions">',
          '<button type="button" data-smiles-dialog-close>Close</button>',
          '<button type="submit">Generate</button></div></form>',
        ].join("");
        root.appendChild(backdrop);
        const finish = (result) => {
          backdrop.remove();
          resolve(result);
        };
        backdrop.addEventListener("click", (event) => {
          if (event.target.closest("[data-smiles-dialog-close]")) {
            finish(false);
          }
        });
        backdrop.addEventListener("keydown", (event) => {
          if (event.key === "Escape") {
            finish(false);
          }
        });
        backdrop.querySelector("form").addEventListener("submit", async (event) => {
          event.preventDefault();
          const input = backdrop.querySelector("textarea");
          const errorHost = backdrop.querySelector(".smiles-dialog-error");
          const smiles = String(input?.value || "").trim();
          errorHost.textContent = "";
          input?.classList.remove("is-invalid");
          if (!smiles) {
            errorHost.textContent = "Enter a SMILES string.";
            input?.classList.add("is-invalid");
            input?.focus?.();
            return;
          }
          try {
            const result = await commandEngine.executeCommand({
              type: "insert-smiles",
              smiles,
              x: Number(point?.x || 0),
              y: Number(point?.y || 0),
              meta: { source: "smiles-dialog" },
            });
            if (!result?.changed) {
              throw new Error("The structure was not inserted.");
            }
            await onApply?.(result);
            finish(true);
          } catch (error) {
            errorHost.textContent = readableError(error);
            input?.classList.add("is-invalid");
            input?.focus?.();
          }
        });
        backdrop.querySelector("textarea")?.focus?.({ preventScroll: true });
      });
    },
  };
}

function readableError(error) {
  const text = String(error?.message || error || "Invalid or unsupported SMILES.");
  return text.replace(/^Error:\s*/i, "") || "Invalid or unsupported SMILES.";
}
