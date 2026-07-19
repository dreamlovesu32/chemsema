import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const assetDir = join(rootDir, "docs", "assets", "readme", "comparison");

const outputPath = join(assetDir, "published-cdxml-comparison.svg");

function readSvg(name) {
  const path = join(assetDir, name);
  const source = readFileSync(path, "utf8").trim();
  const openTag = source.match(/<svg\b([^>]*)>/i);
  if (!openTag) {
    throw new Error(`${name} does not contain an SVG root`);
  }
  const attrs = openTag[1];
  const viewBoxMatch = attrs.match(/\bviewBox="([^"]+)"/i);
  const widthMatch = attrs.match(/\bwidth="([^"]+)"/i);
  const heightMatch = attrs.match(/\bheight="([^"]+)"/i);
  const viewBox = viewBoxMatch?.[1]
    || `0 0 ${parseFloat(widthMatch?.[1] || "0")} ${parseFloat(heightMatch?.[1] || "0")}`;
  const parts = viewBox.trim().split(/\s+/).map(Number);
  if (parts.length !== 4 || parts.some((value) => !Number.isFinite(value)) || parts[2] <= 0 || parts[3] <= 0) {
    throw new Error(`${name} has an invalid viewBox: ${viewBox}`);
  }
  const inner = source
    .replace(/^<\?xml[^>]*>\s*/i, "")
    .replace(/^<!DOCTYPE[^>]*>\s*/i, "")
    .replace(/<svg\b[^>]*>/i, "")
    .replace(/<\/svg>\s*$/i, "")
    .trim();
  return { name, viewBox, width: parts[2], height: parts[3], inner };
}

function escapeAttr(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function embeddedSvg(svg, x, y, width, height, padding = 18) {
  const maxWidth = width - padding * 2;
  const maxHeight = height - padding * 2;
  const scale = Math.min(maxWidth / svg.width, maxHeight / svg.height);
  const renderedWidth = svg.width * scale;
  const renderedHeight = svg.height * scale;
  const offsetX = x + (width - renderedWidth) / 2;
  const offsetY = y + (height - renderedHeight) / 2;
  return [
    `<svg x="${offsetX.toFixed(3)}" y="${offsetY.toFixed(3)}" width="${renderedWidth.toFixed(3)}" height="${renderedHeight.toFixed(3)}" viewBox="${escapeAttr(svg.viewBox)}">`,
    svg.inner,
    "</svg>",
  ].join("\n");
}

const figures = [
  {
    label: "Figure 1",
    top: 132,
    height: 454,
    chemdraw: readSvg("figure1.chemdraw.svg"),
    chemsema: readSvg("figure1.chemsema.svg"),
  },
  {
    label: "Figure 2",
    top: 650,
    height: 238,
    chemdraw: readSvg("figure2.chemdraw.svg"),
    chemsema: readSvg("figure2.chemsema.svg"),
  },
];

const leftX = 146;
const rightX = 710;
const cardWidth = 540;
const output = [
  '<svg xmlns="http://www.w3.org/2000/svg" width="1278" height="924" viewBox="0 0 1278 924">',
  '  <rect width="100%" height="100%" fill="#ffffff"/>',
  '  <text x="28" y="38" font-family="Arial, sans-serif" font-size="24" font-weight="700" fill="#111827">ChemDraw vs ChemSema: CDXML rendering from published figures</text>',
  '  <text x="28" y="62" font-family="Arial, sans-serif" font-size="13" fill="#4b5563">Source CDXML: Copper-catalyzed site- and enantioselective C-H cyanation of trisubstituted allenes</text>',
  '  <text x="416" y="112" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" font-weight="700" fill="#111827">ChemDraw export</text>',
  '  <text x="980" y="112" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" font-weight="700" fill="#111827">ChemSema export</text>',
];

for (const figure of figures) {
  const labelY = figure.top + figure.height / 2;
  output.push(`  <text x="28" y="${labelY.toFixed(0)}" text-anchor="start" dominant-baseline="middle" font-family="Arial, sans-serif" font-size="15" font-weight="700" fill="#374151">${figure.label}</text>`);
  output.push(`  <rect x="${leftX}" y="${figure.top}" width="${cardWidth}" height="${figure.height}" rx="8" fill="#ffffff" stroke="#d1d5db"/>`);
  output.push(embeddedSvg(figure.chemdraw, leftX, figure.top, cardWidth, figure.height));
  output.push(`  <rect x="${rightX}" y="${figure.top}" width="${cardWidth}" height="${figure.height}" rx="8" fill="#ffffff" stroke="#d1d5db"/>`);
  output.push(embeddedSvg(figure.chemsema, rightX, figure.top, cardWidth, figure.height));
}

output.push("</svg>");
writeFileSync(outputPath, `${output.join("\n")}\n`);
console.log(outputPath);
