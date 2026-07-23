import fs from "node:fs/promises";
import path from "node:path";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-shadow-probe");
const input = path.join(outDir, "shadow-matrix.cdxml");
const oracleDir = path.join(outDir, "oracle");

const shadowSizes = [100, 200, 300, 400, 600, 800];
const lineWidths = [0.6, 1, 2];
const cells = [];
let id = 10;
for (const lineWidth of lineWidths) {
  for (const shadowSize of shadowSizes) {
    const column = shadowSizes.indexOf(shadowSize);
    const row = lineWidths.indexOf(lineWidth);
    const left = 20 + column * 70;
    const top = 20 + row * 60;
    cells.push(`    <graphic id="${id++}" BoundingBox="${left} ${top} ${left + 44} ${top + 30}" GraphicType="Rectangle" RectangleType="RoundEdge Shadow" CornerRadius="600" ShadowSize="${shadowSize}" LineWidth="${lineWidth}"/>`);
  }
}

const source = `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemSema shadow probe" BoundingBox="0 0 450 210" LineWidth="0.6" BoldWidth="2" BondLength="14.4" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 450 210">
${cells.join("\n")}
  </page>
</CDXML>
`;

await fs.mkdir(outDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
await fs.writeFile(input, source, "utf8");
await generateChemDrawOracle({
  inputs: [input],
  outDir: oracleDir,
  formats: ["svg"],
  outputNames: ["shadow-matrix"],
});
console.log(input);
console.log(path.join(oracleDir, "shadow-matrix.chemdraw.svg"));
