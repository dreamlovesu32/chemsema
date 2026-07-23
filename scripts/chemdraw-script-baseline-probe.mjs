import fs from "node:fs/promises";
import path from "node:path";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const OUT_DIR = path.resolve(process.argv[2] || "tmp/chemdraw-script-baseline-probe");

const fonts = [
  [3, "Arial"],
  [4, "Times New Roman"],
  [5, "Calibri"],
  [6, "Helvetica"],
];
const sizes = [7, 10, 14.45, 18];
const scripts = [
  ["subscript", 32],
  ["superscript", 64],
  ["bold-subscript", 33],
  ["bold-superscript", 65],
];

const cases = [];
for (const [font, fontName] of fonts) {
  for (const size of sizes) {
    for (const [script, face] of scripts) {
      const serial = String(cases.length + 1).padStart(3, "0");
      cases.push({ serial, font, fontName, size, script, face });
    }
  }
}

function documentXml() {
  const rows = cases.map((probe, index) => {
    const y = 40 + index * 28;
    return `<t id="${1000 + index}" p="72 ${y}" InterpretChemically="no"><s font="${probe.font}" size="${probe.size}" face="0">A${probe.serial}</s><s font="${probe.font}" size="${probe.size}" face="${probe.face}">X${probe.serial}</s><s font="${probe.font}" size="${probe.size}" face="0">Z${probe.serial}</s></t>`;
  });
  return `<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema script-baseline probe" BoundingBox="0 0 520 ${cases.length * 28 + 80}" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10" BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable>${fonts.map(([id, name]) => `<font id="${id}" charset="iso-8859-1" name="${name}"/>`).join("")}</fonttable>
<page id="1" BoundingBox="0 0 520 ${cases.length * 28 + 80}" HeightPages="1" WidthPages="1">${rows.join("\n")}</page>
</CDXML>`;
}

function svgTextRecords(svg) {
  return [...svg.matchAll(/<text\b([^>]*)>([\s\S]*?)<\/text>/g)].map((match) => {
    const attributes = match[1];
    const matrix = /transform="matrix\(([^)]+)\)"/.exec(attributes)?.[1]
      .trim().split(/[ ,]+/).map(Number);
    return {
      text: match[2].replace(/<[^>]+>/g, "").trim(),
      x: matrix?.[4],
      y: matrix?.[5],
      scale: matrix?.[0],
      fontSize: Number(/font-size="([^"]+)px"/.exec(attributes)?.[1]),
    };
  });
}

await fs.mkdir(OUT_DIR, { recursive: true });
const input = path.join(OUT_DIR, "script-baselines.cdxml");
await fs.writeFile(input, documentXml(), "utf8");
const oracleDir = path.join(OUT_DIR, "oracle");
await generateChemDrawOracle({ inputs: [input], outDir: oracleDir, formats: ["svg"] });
const svgPath = path.join(oracleDir, "script-baselines.chemdraw.svg");
const records = svgTextRecords(await fs.readFile(svgPath, "utf8"));
const byText = new Map(records.map((record) => [record.text, record]));
const measurements = cases.map((probe) => {
  const normal = byText.get(`A${probe.serial}`);
  const scripted = byText.get(`X${probe.serial}`);
  const pixelsPerPoint = normal ? normal.scale * normal.fontSize / probe.size : null;
  const shiftPoints = normal && scripted && pixelsPerPoint
    ? (scripted.y - normal.y) / pixelsPerPoint
    : null;
  return {
    ...probe,
    renderedScriptSizeRatio: normal && scripted ? scripted.fontSize / normal.fontSize : null,
    shiftPoints,
    shiftEm: shiftPoints === null ? null : shiftPoints / probe.size,
  };
});
const report = { generatedAt: new Date().toISOString(), input, svgPath, measurements };
const reportPath = path.join(OUT_DIR, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
console.log(JSON.stringify({ reportPath, count: measurements.length }));
