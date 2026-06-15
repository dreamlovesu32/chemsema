import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import opentype from "opentype.js";

const rootDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const fontPath = process.argv[2] || "C:/Windows/Fonts/arial.ttf";
const profilePath = path.join(rootDir, "shared", "glyph_profiles.json");
const outputPath = path.join(rootDir, "shared", "glyph_outlines.json");

function round(value) {
  return Number(Number(value).toFixed(8));
}

function commandPayload(command) {
  const points = [];
  if (Number.isFinite(command.x1) && Number.isFinite(command.y1)) {
    points.push([round(command.x1), round(command.y1)]);
  }
  if (Number.isFinite(command.x2) && Number.isFinite(command.y2)) {
    points.push([round(command.x2), round(command.y2)]);
  }
  if (Number.isFinite(command.x) && Number.isFinite(command.y)) {
    points.push([round(command.x), round(command.y)]);
  }
  return { op: command.type, points };
}

function commandBounds(commands) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const command of commands) {
    for (const point of command.points || []) {
      minX = Math.min(minX, point[0]);
      minY = Math.min(minY, point[1]);
      maxX = Math.max(maxX, point[0]);
      maxY = Math.max(maxY, point[1]);
    }
  }
  if (!Number.isFinite(minX)) {
    return null;
  }
  return [round(minX), round(minY), round(maxX), round(maxY)];
}

const fontBytes = fs.readFileSync(fontPath);
const font = opentype.parse(
  fontBytes.buffer.slice(fontBytes.byteOffset, fontBytes.byteOffset + fontBytes.byteLength),
);
const profiles = JSON.parse(fs.readFileSync(profilePath, "utf8"));
const visibleChars = Object.entries(profiles.specials)
  .filter(([, profile]) => profile.visible !== false)
  .map(([character]) => character);

const glyphs = {};
for (const character of visibleChars) {
  const glyph = font.charToGlyph(character);
  if (!glyph || glyph.index === 0) {
    console.warn(`skip ${JSON.stringify(character)}: missing glyph`);
    continue;
  }
  const commands = glyph.getPath(0, 0, 1).commands
    .map(commandPayload)
    .filter((command) => command.op === "Z" || command.points.length > 0);
  const bounds = commandBounds(commands);
  if (!bounds) {
    console.warn(`skip ${JSON.stringify(character)}: empty outline`);
    continue;
  }
  glyphs[character] = {
    advanceEm: round(glyph.advanceWidth / font.unitsPerEm),
    boundsEm: bounds,
    commands,
  };
}

const payload = {
  version: 1,
  sourceFont: fontPath,
  unitsPerEm: font.unitsPerEm,
  glyphs,
};

fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
console.log(outputPath);
