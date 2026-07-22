import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const fine = process.argv.includes("--fine");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(
  root,
  outputArg ?? (fine
    ? "tmp/chemdraw-hashed-wedge-probe-fine"
    : "tmp/chemdraw-hashed-wedge-probe"),
);
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const profiles = [
  { name: "default", lineWidth: 1.0, boldWidth: 4.0, hashSpacing: 2.7 },
  { name: "acs", lineWidth: 0.6, boldWidth: 2.0, hashSpacing: 2.5 },
  { name: "wide-line", lineWidth: 1.4, boldWidth: 4.0, hashSpacing: 2.7 },
  { name: "tight-hash", lineWidth: 1.0, boldWidth: 4.0, hashSpacing: 2.0 },
  { name: "loose-hash", lineWidth: 1.0, boldWidth: 4.0, hashSpacing: 3.4 },
];

function lengthsFor(profile) {
  if (!fine) {
    return Array.from({ length: 113 }, (_, index) => 4 + index * 0.5);
  }
  const values = new Set();
  for (let stripeCount = 2; stripeCount <= 12; stripeCount += 1) {
    const threshold = profile.lineWidth + (stripeCount - 1) * profile.hashSpacing;
    for (const delta of [-0.02, -0.01, 0, 0.01, 0.02]) {
      values.add(Number((threshold + delta).toFixed(2)));
    }
  }
  return [...values].sort((a, b) => a - b);
}

function fixed(value) {
  return Number(value).toFixed(2);
}

function sourceFor(profile, length) {
  const x0 = 12;
  const y = 20;
  const x1 = x0 + length;
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 ${fixed(x1 + 12)} 40" FractionalWidths="yes" InterpretChemically="yes" LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" LineWidth="${fixed(profile.lineWidth)}" BoldWidth="${fixed(profile.boldWidth)}" BondLength="${fixed(length)}" BondSpacing="18" HashSpacing="${fixed(profile.hashSpacing)}" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 ${fixed(x1 + 12)} 40">
    <fragment id="2">
      <n id="3" p="${fixed(x0)} ${fixed(y)}" Z="1" Element="6" NumHydrogens="3"/>
      <n id="4" p="${fixed(x1)} ${fixed(y)}" Z="2" Element="6" NumHydrogens="3"/>
      <b id="5" Z="3" B="3" E="4" Order="1" Display="WedgedHashBegin"/>
    </fragment>
  </page>
</CDXML>
`;
}

function blackShapeCount(svg) {
  const tags = svg.match(/<(?:path|polygon|rect)\b[^>]*>/gi) ?? [];
  return tags.filter((tag) => /\bfill="#000000"/i.test(tag) && !/\bstroke="#ffffff"/i.test(tag)).length;
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const jobs = [];
for (const profile of profiles) {
  for (const length of lengthsFor(profile)) {
    const stem = `${profile.name}-l${fixed(length).replace(".", "_")}`;
    const input = path.join(sourceDir, `${stem}.cdxml`);
    await fs.writeFile(input, sourceFor(profile, length), "utf8");
    jobs.push({ profile, length, stem, input });
  }
}

const oracleJobs = [];
for (const job of jobs) {
  const svgPath = path.join(oracleDir, `${job.stem}.chemdraw.svg`);
  if (refresh) {
    oracleJobs.push(job);
    continue;
  }
  try {
    await fs.access(svgPath);
  } catch {
    oracleJobs.push(job);
  }
}
if (oracleJobs.length > 0) {
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["svg"],
    inputs: oracleJobs.map((job) => job.input),
  });
}

const rows = [];
for (const job of jobs) {
  const svgPath = path.join(oracleDir, `${job.stem}.chemdraw.svg`);
  const svg = await fs.readFile(svgPath, "utf8");
  rows.push({
    profile: job.profile.name,
    lineWidth: job.profile.lineWidth,
    boldWidth: job.profile.boldWidth,
    hashSpacing: job.profile.hashSpacing,
    length: job.length,
    stripeCount: blackShapeCount(svg),
    expectedStripeCount: Math.max(
      1,
      1 + Math.floor((job.length - job.profile.lineWidth + 1e-9) / job.profile.hashSpacing),
    ),
  });
}

for (const row of rows) {
  row.matchesRule = row.stripeCount === row.expectedStripeCount;
}

await fs.writeFile(path.join(outDir, "measurements.json"), `${JSON.stringify(rows, null, 2)}\n`, "utf8");
const mismatches = rows.filter((row) => !row.matchesRule);
if (mismatches.length > 0) {
  throw new Error(`Hashed-wedge rule mismatched ${mismatches.length} ChemDraw samples`);
}
console.log(path.join(outDir, "measurements.json"));
