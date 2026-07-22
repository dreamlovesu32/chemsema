import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { inspectEmf } from "./emf-inspect.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outputArgument = process.argv.slice(2).find((argument) => !argument.startsWith("--"));
const outDir = path.resolve(root, outputArgument ?? "tmp/chemdraw-emf-bond-probe");
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");
const refresh = process.argv.includes("--refresh");
const verifyChemsema = process.argv.includes("--verify-chemsema");
const chemsemaDir = path.join(outDir, "chemsema");

const profiles = [
  { name: "default", lineWidth: 1, boldWidth: 4, hashSpacing: 2.7 },
  { name: "acs", lineWidth: 0.6, boldWidth: 2, hashSpacing: 2.5 },
  { name: "wide", lineWidth: 1.4, boldWidth: 6, hashSpacing: 3.4 },
];
const lengths = [1, 2, 3, 4, 12, 20, 30, 40, 60];
const angles = [0, 30, 90, 150];

function fixed(value) {
  return Number(value).toFixed(3);
}

function sourceFor(profile, length, angle, display = null) {
  const theta = angle * Math.PI / 180;
  const x0 = 20;
  const y0 = 20;
  const x1 = x0 + length * Math.cos(theta);
  const y1 = y0 + length * Math.sin(theta);
  const minX = Math.min(x0, x1) - 15;
  const minY = Math.min(y0, y1) - 15;
  const maxX = Math.max(x0, x1) + 15;
  const maxY = Math.max(y0, y1) + 15;
  const displayAttribute = display ? ` Display="${display}"` : "";
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="${fixed(minX)} ${fixed(minY)} ${fixed(maxX)} ${fixed(maxY)}" FractionalWidths="yes" InterpretChemically="yes" LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" LineWidth="${fixed(profile.lineWidth)}" BoldWidth="${fixed(profile.boldWidth)}" BondLength="${fixed(length)}" BondSpacing="18" HashSpacing="${fixed(profile.hashSpacing)}" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="${fixed(minX)} ${fixed(minY)} ${fixed(maxX)} ${fixed(maxY)}">
    <fragment id="2">
      <n id="3" p="${fixed(x0)} ${fixed(y0)}" Z="1" Element="6" NumHydrogens="3"/>
      <n id="4" p="${fixed(x1)} ${fixed(y1)}" Z="2" Element="6" NumHydrogens="3"/>
      <b id="5" Z="3" B="3" E="4" Order="1"${displayAttribute}/>
    </fragment>
  </page>
</CDXML>
`;
}

function plusRecords(inspection) {
  return inspection.records.flatMap((record) => record.emfPlusRecords ?? []);
}

function summarize(inspection) {
  const records = plusRecords(inspection);
  return {
    plusRecordCounts: Object.fromEntries(
      [...new Set(records.map((record) => record.name))]
        .sort()
        .map((name) => [name, records.filter((record) => record.name === name).length]),
    ),
    pens: records.filter((record) => record.pen).map((record) => record.pen),
    drawLines: records
      .filter((record) => record.name === "EmfPlusDrawLines")
      .map(({ penId, points }) => ({ penId, points })),
    gdiPens: inspection.records
      .filter((record) => record.name === "EMR_EXTCREATEPEN")
      .map(({ penStyle, width, color, styles }) => ({ penStyle, width, color, styles })),
    gdiLines: inspection.records
      .filter((record) => record.name === "EMR_POLYLINE16")
      .map(({ bounds, pointCount }) => ({ bounds, pointCount })),
  };
}

function requireRule(condition, message) {
  if (!condition) throw new Error(message);
}

function closeEnough(left, right, tolerance = 1e-3) {
  return Math.abs(left - right) <= tolerance;
}

function lineInvariant(emf) {
  return emf.drawLines.map((line, index) => {
    requireRule(line.points?.length === 2, `draw line ${index} does not contain two points`);
    const [start, end] = line.points;
    const pen = emf.pens[index];
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    return {
      center: { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 },
      vector: { x: dx, y: dy },
      lengthInPenWidths: Math.hypot(dx, dy) / pen.width,
    };
  });
}

function assertRecordStrategy(measurement, source) {
  const { emf, kind, lineWidth, boldWidth, hashSpacing, length, angle } = measurement;
  const expectedCount = kind === "single"
    ? 1
    : Math.max(1, 1 + Math.floor((length - lineWidth) / hashSpacing));
  requireRule(emf.pens.length === expectedCount, `${source} ${kind}: expected ${expectedCount} pens, got ${emf.pens.length}`);
  requireRule(emf.drawLines.length === expectedCount, `${source} ${kind}: expected ${expectedCount} DrawLines records, got ${emf.drawLines.length}`);
  requireRule(!emf.plusRecordCounts.EmfPlusFillPath, `${source} ${kind}: unexpected filled path`);
  requireRule(!emf.plusRecordCounts.EmfPlusSave, `${source} ${kind}: unexpected graphics save`);
  requireRule(!emf.plusRecordCounts.EmfPlusRestore, `${source} ${kind}: unexpected graphics restore`);
  for (const [index, pen] of emf.pens.entries()) {
    requireRule(pen.penDataFlags === 214, `${source} ${kind} pen ${index}: flags ${pen.penDataFlags}`);
    requireRule(pen.startCap === 2 && pen.endCap === 2, `${source} ${kind} pen ${index}: caps are not round`);
    requireRule(pen.dashedLineCap === 2, `${source} ${kind} pen ${index}: dash cap is not round`);
    requireRule(closeEnough(pen.miterLimit, 2), `${source} ${kind} pen ${index}: miter limit ${pen.miterLimit}`);
  }
  for (const [index, pen] of emf.gdiPens.entries()) {
    requireRule(pen.penStyle === 73728, `${source} ${kind} GDI pen ${index}: style ${pen.penStyle}`);
  }
  const invariant = lineInvariant(emf);
  const axis = {
    x: Math.cos(angle * Math.PI / 180),
    y: Math.sin(angle * Math.PI / 180),
  };
  for (const [index, line] of invariant.entries()) {
    const lineLength = Math.hypot(line.vector.x, line.vector.y);
    const axialProjection = Math.abs(line.vector.x * axis.x + line.vector.y * axis.y) / lineLength;
    const expectedProjection = kind === "single" ? 1 : 0;
    requireRule(
      closeEnough(axialProjection, expectedProjection, 2.5e-3),
      `${source} ${kind}: line ${index} is not ${kind === "single" ? "parallel" : "perpendicular"} to the bond axis`,
    );
  }
  if (kind === "single") {
    requireRule(closeEnough(invariant[0].lengthInPenWidths, length / lineWidth, 6e-3), `${source} single: length/width mismatch`);
    return invariant;
  }
  const ordered = [...invariant].sort((left, right) => left.lengthInPenWidths - right.lengthInPenWidths);
  if (ordered.length === 1) return ordered;
  const endpointDelta = 1.5 * boldWidth - lineWidth;
  const narrow = 1 + endpointDelta / (2 * length);
  const wide = 1.5 * boldWidth / lineWidth - endpointDelta / (2 * length);
  requireRule(closeEnough(ordered[0].lengthInPenWidths, narrow, 2e-3), `${source} hashed wedge: narrow stripe centerline mismatch`);
  requireRule(closeEnough(ordered.at(-1).lengthInPenWidths, wide, 2e-3), `${source} hashed wedge: wide stripe centerline mismatch`);
  for (let index = 0; index < ordered.length; index += 1) {
    const t = ordered.length === 1 ? 0 : index / (ordered.length - 1);
    const expected = narrow + t * (wide - narrow);
    requireRule(closeEnough(ordered[index].lengthInPenWidths, expected, 3e-3), `${source} hashed wedge: stripe ${index} width interpolation mismatch`);
  }
  return ordered;
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
if (verifyChemsema) await fs.mkdir(chemsemaDir, { recursive: true });
const jobs = [];
for (const profile of profiles) {
  for (const length of lengths) {
    for (const angle of angles) {
      for (const [kind, display] of [["single", null], ["hashed-wedge", "WedgedHashBegin"]]) {
        const stem = `${kind}-${profile.name}-l${length}-a${angle}`;
        const input = path.join(sourceDir, `${stem}.cdxml`);
        await fs.writeFile(input, sourceFor(profile, length, angle, display), "utf8");
        jobs.push({ kind, profile, length, angle, stem, input });
      }
    }
  }
}

const missing = [];
for (const job of jobs) {
  const output = path.join(oracleDir, `${job.stem}.chemdraw.emf`);
  try {
    if (refresh) throw new Error("refresh");
    await fs.access(output);
  } catch {
    missing.push(job);
  }
}
if (missing.length) {
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["emf"],
    inputs: missing.map((job) => job.input),
  });
}

const measurements = [];
for (const job of jobs) {
  const emf = path.join(oracleDir, `${job.stem}.chemdraw.emf`);
  const measurement = {
    kind: job.kind,
    profile: job.profile.name,
    lineWidth: job.profile.lineWidth,
    boldWidth: job.profile.boldWidth,
    hashSpacing: job.profile.hashSpacing,
    length: job.length,
    angle: job.angle,
    emf: summarize(await inspectEmf(emf)),
  };
  assertRecordStrategy(measurement, `ChemDraw ${job.stem}`);
  if (verifyChemsema) {
    const cli = path.join(root, "target", "debug", "chemsema-cli.exe");
    const chemsemaEmf = path.join(chemsemaDir, `${job.stem}.chemsema.emf`);
    execFileSync(cli, ["convert", job.input, chemsemaEmf], { cwd: root, stdio: "ignore" });
    measurement.chemsema = summarize(await inspectEmf(chemsemaEmf));
    const expected = assertRecordStrategy(measurement, `ChemDraw ${job.stem}`);
    const actual = assertRecordStrategy(
      { ...measurement, emf: measurement.chemsema },
      `ChemSema ${job.stem}`,
    );
    requireRule(expected.length === actual.length, `ChemSema ${job.stem}: invariant count mismatch`);
    for (let index = 0; index < expected.length; index += 1) {
      // Imported node coordinates are stored at 0.01 pt precision. The
      // absolute effect is only visible in these deliberately tiny 1 pt
      // diagonal probes, so compare the scale-free ratios with a tolerance
      // that covers that documented coordinate quantization.
      requireRule(closeEnough(expected[index].lengthInPenWidths, actual[index].lengthInPenWidths, 1.2e-2), `ChemSema ${job.stem}: stripe ${index} geometry differs from ChemDraw`);
    }
  }
  measurements.push(measurement);
}
const output = path.join(outDir, "measurements.json");
await fs.writeFile(output, `${JSON.stringify(measurements, null, 2)}\n`, "utf8");
console.log(output);
