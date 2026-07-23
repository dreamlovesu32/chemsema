export function normalizeHexColor(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  const match = raw.match(/^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/);
  if (!match) {
    return null;
  }
  const channels = match.slice(1).map((channel) => {
    const value = Number.parseInt(channel, 10);
    return Math.max(0, Math.min(255, value)).toString(16).padStart(2, "0");
  });
  return `#${channels.join("")}`;
}
