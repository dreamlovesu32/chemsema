export function createTransientNotificationHost({ root = document.body } = {}) {
  let timer = null;
  let host = null;

  function show(message, { error = false, duration = 2400 } = {}) {
    if (!host) {
      host = document.createElement("div");
      host.className = "transient-notification";
      host.setAttribute("role", "status");
      host.setAttribute("aria-live", "polite");
      root.appendChild(host);
    }
    clearTimeout(timer);
    host.textContent = String(message || "");
    host.classList.toggle("is-error", error);
    host.classList.add("is-visible");
    timer = setTimeout(() => host?.classList.remove("is-visible"), duration);
  }

  return { show };
}
