export function documentTitleForFileName(documentData) {
  const rawTitle = String(documentData?.document?.title || "chemcore-document").trim();
  const safeTitle = rawTitle
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${safeTitle || "chemcore-document"}.chemcore.json`;
}

export function saveFormatFromFileName(fileName) {
  const lowerName = String(fileName || "").toLowerCase();
  if (lowerName.endsWith(".svg")) {
    return "svg";
  }
  if (lowerName.endsWith(".cdxml")) {
    return "cdxml";
  }
  return "json";
}

export function looksLikeCdxmlFile(file, text) {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  if (name.endsWith(".cdxml") || type.includes("cdxml")) {
    return true;
  }
  return /^\s*(?:<\?xml[^>]*>\s*)?<CDXML\b/i.test(text);
}

export function downloadTextFile(content, fileName, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
