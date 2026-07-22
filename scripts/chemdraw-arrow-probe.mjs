import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const positional = process.argv.slice(2).filter((value) => !value.startsWith("--"));
const outDir = path.resolve(root, positional[0] ?? "tmp/chemdraw-arrow-probe");
const cli = path.resolve(root, positional[1] ?? "target/debug/chemsema-cli.exe");
const sourcePath = path.join(outDir, "arrow-matrix.cdxml");
const oracleDir = path.join(outDir, "chemdraw");
const oracleCdx = path.join(oracleDir, "arrow-matrix.chemdraw.cdx");
const oracleCdxml = path.join(oracleDir, "arrow-matrix.chemdraw.cdxml");
const chemsemaCdxml = path.join(outDir, "arrow-matrix.chemsema.cdxml");

const cases = [
  { name: "modern-none", tag: "arrow", attrs: 'ArrowheadType="Solid"' },
  { name: "modern-full", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid"' },
  { name: "modern-half-left", tag: "arrow", attrs: 'ArrowheadHead="HalfLeft" ArrowheadType="Solid"' },
  { name: "modern-half-right", tag: "arrow", attrs: 'ArrowheadHead="HalfRight" ArrowheadType="Solid"' },
  { name: "modern-double", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadTail="Full" ArrowheadType="Solid"' },
  { name: "modern-hollow", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Hollow" ArrowShaftSpacing="1100"' },
  { name: "modern-open", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Angle" ArrowShaftSpacing="800"' },
  { name: "modern-equilibrium", tag: "arrow", attrs: 'ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid" ArrowShaftSpacing="420"' },
  { name: "modern-unequal-equilibrium", tag: "arrow", attrs: 'ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid" ArrowShaftSpacing="420" ArrowEquilibriumRatio="300"' },
  { name: "modern-cross", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" NoGo="Cross"' },
  { name: "modern-hash", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" NoGo="Hash"' },
  { name: "modern-dipole", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" Dipole="yes"' },
  { name: "modern-custom-size", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" HeadSize="1250" ArrowheadCenterSize="1100" ArrowheadWidth="325"' },
  { name: "modern-bold-dashed", tag: "arrow", attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" LineType="Bold Dashed"' },
  { name: "modern-curve-90", tag: "arrow", curve: 90, attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" FillType="Solid" AngularSize="90"' },
  { name: "modern-curve-negative-90", tag: "arrow", curve: -90, attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" FillType="Solid" AngularSize="-90"' },
  { name: "modern-curve-120", tag: "arrow", curve: 120, attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" FillType="Solid" AngularSize="120"' },
  { name: "modern-curve-180", tag: "arrow", curve: 180, attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" FillType="Solid" AngularSize="180"' },
  { name: "modern-curve-270", tag: "arrow", curve: 270, attrs: 'ArrowheadHead="Full" ArrowheadType="Solid" FillType="Solid" AngularSize="270"' },
  { name: "legacy-no-head", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="NoHead"' },
  { name: "legacy-half-left", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="HalfHead" HeadSize="1000"' },
  { name: "legacy-half-right", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="HalfHead" HeadSize="-1000"' },
  { name: "legacy-full", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="FullHead"' },
  { name: "legacy-resonance", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="Resonance"' },
  { name: "legacy-equilibrium", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="Equilibrium"' },
  { name: "legacy-hollow", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="Hollow"' },
  { name: "legacy-retrosynthetic", tag: "graphic", attrs: 'GraphicType="Line" ArrowType="RetroSynthetic"' },
];

function sourceDocument() {
  const rows = cases.map((entry, index) => {
    const id = 10 + index;
    const y = 20 + index * 20;
    let geometry;
    if (entry.tag === "graphic") {
      geometry = `BoundingBox="150 ${y} 10 ${y}"`;
    } else if (entry.curve !== undefined) {
      const radians = entry.curve * Math.PI / 180;
      const tailX = 80 + 70 * Math.cos(radians);
      const tailY = y + 70 * Math.sin(radians);
      geometry = `Tail3D="${tailX.toFixed(4)} ${tailY.toFixed(4)} 0" Head3D="150 ${y} 0" Center3D="80 ${y} 0" MajorAxisEnd3D="150 ${y} 0" MinorAxisEnd3D="80 ${y + 70} 0"`;
    } else {
      geometry = `Tail3D="10 ${y} 0" Head3D="150 ${y} 0"`;
    }
    return `    <${entry.tag} id="${id}" ${geometry} ${entry.attrs}/>`;
  });
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 180 ${cases.length * 20 + 40}" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1" BoundingBox="0 0 180 ${cases.length * 20 + 40}">
${rows.join("\n")}
  </page>
</CDXML>
`;
}

function attributes(tag) {
  const values = {};
  for (const match of tag.matchAll(/([A-Za-z][A-Za-z0-9]*)="([^"]*)"/g)) {
    values[match[1]] = match[2];
  }
  return values;
}

function arrowRows(xml) {
  return [...xml.matchAll(/<arrow\b[^>]*\/>/gi)].map((match) => attributes(match[0]));
}

function sourceIdsByArrowId(xml) {
  return new Map(
    [...xml.matchAll(/<graphic\b[^>]*\/>/gi)]
      .map((match) => attributes(match[0]))
      .filter((row) => row.id && row.SupersededBy)
      .map((row) => [row.SupersededBy, row.id]),
  );
}

function normalized(row) {
  return {
    head: row.ArrowheadHead ?? "None",
    tail: row.ArrowheadTail ?? "None",
    type: row.ArrowheadType ?? "Solid",
    headSize: row.HeadSize ?? "1000",
    centerSize: row.ArrowheadCenterSize ?? "875",
    width: row.ArrowheadWidth ?? "250",
    shaftSpacing: row.ArrowShaftSpacing ?? null,
    equilibriumRatio: row.ArrowEquilibriumRatio ?? null,
    angularSize: row.AngularSize ?? null,
    noGo: row.NoGo ?? null,
    dipole: row.Dipole ?? null,
    lineType: row.LineType?.split(/\s+/).sort().join(" ") ?? null,
  };
}

await fs.mkdir(outDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
await fs.writeFile(sourcePath, sourceDocument(), "utf8");

let haveOracle = false;
if (!refresh) {
  try {
    await Promise.all([fs.access(oracleCdx), fs.access(oracleCdxml)]);
    haveOracle = true;
  } catch {
    haveOracle = false;
  }
}
if (!haveOracle) {
  await generateChemDrawOracle({ outDir: oracleDir, formats: ["cdx", "cdxml"], inputs: [sourcePath] });
}

const conversion = spawnSync(cli, ["convert", oracleCdx, chemsemaCdxml], {
  cwd: root,
  encoding: "utf8",
});
if (conversion.status !== 0) {
  throw new Error(`ChemSema conversion failed: ${conversion.stderr || conversion.stdout}`);
}

const referenceXml = await fs.readFile(oracleCdxml, "utf8");
const referenceRows = arrowRows(referenceXml);
const candidateRows = arrowRows(await fs.readFile(chemsemaCdxml, "utf8"));
if (referenceRows.length !== cases.length || candidateRows.length !== cases.length) {
  throw new Error(`Arrow count mismatch: cases=${cases.length}, ChemDraw=${referenceRows.length}, ChemSema=${candidateRows.length}`);
}

const sourceIds = sourceIdsByArrowId(referenceXml);
const results = referenceRows.map((referenceRow, index) => {
  const sourceId = sourceIds.get(referenceRow.id);
  const entry = cases[Number(sourceId) - 10];
  if (!entry) {
    throw new Error(`ChemDraw arrow ${referenceRow.id} has no source case mapping`);
  }
  const reference = normalized(referenceRows[index]);
  const candidate = normalized(candidateRows[index]);
  return {
    name: entry.name,
    sourceId,
    passed: JSON.stringify(reference) === JSON.stringify(candidate),
    reference,
    candidate,
  };
});
await fs.writeFile(path.join(outDir, "report.json"), `${JSON.stringify(results, null, 2)}\n`, "utf8");
const failures = results.filter((result) => !result.passed);
if (failures.length > 0) {
  throw new Error(`Arrow probe mismatched ${failures.length} cases: ${failures.map((result) => result.name).join(", ")}`);
}
console.log(path.join(outDir, "report.json"));
