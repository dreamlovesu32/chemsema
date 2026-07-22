import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const DEFAULT_OUT_DIR = "tmp/chemdraw-label-retreat-probe";
const DEFAULT_ANGLES = Array.from({ length: 24 }, (_, index) => index * 15);
const FULL_GLYPHS = [
  ..."ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-=()[]{}.,:'\"/\\|_;%#@&*?!",
  "±", "·", "×", "÷", "°", "μ", "π", "σ", "Δ", "→", "⇌",
];
const FOCUSED_GLYPHS = [..."AIMNOWXabegijmo0+().:'"];
const CHEMICAL_LABELS = [
  "Cl", "Br", "OH", "NH", "NH2", "CF3", "OMe", "Me", "Ph", "R'", "R''",
  "Fe", "Fe3+", "CO2H", "SO3H", "NMe2", "t-Bu", "α", "β", "δ", "λ",
];
const REPRESENTATIVE_LABELS = [
  "A", "B", "C", "F", "H", "I", "M", "N", "O", "Q", "R", "S", "W", "X",
  "a", "e", "f", "g", "i", "j", "m", "p", "s", "0", "2", "8", "+", "-", "=",
  "(", ")", "[", "]", ".", ":", "'", ...CHEMICAL_LABELS,
];
const FONTS = [
  { font: "Arial", fontId: 3 },
  { font: "Times New Roman", fontId: 4 },
  { font: "Calibri", fontId: 5 },
  { font: "Cambria", fontId: 6 },
  { font: "Segoe UI", fontId: 7 },
  { font: "Courier New", fontId: 8 },
  { font: "Symbol", fontId: 9 },
];

function slug(value) {
  return String(value).replaceAll(".", "p").replaceAll(" ", "-").toLowerCase();
}

function parseArgs(argv) {
  const args = { outDir: DEFAULT_OUT_DIR, profile: "survey", exportOracle: true };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out") args.outDir = argv[++index];
    else if (arg === "--profile") args.profile = argv[++index];
    else if (arg === "--no-export") args.exportOracle = false;
    else if (arg === "--help" || arg === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return args;
}

function xmlEscape(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function experimentSets(profile) {
  const common = { font: "Arial", fontId: 3, size: 10, face: 0, marginWidth: 2, lineWidth: 1 };
  if (profile === "smoke") {
    return [{ name: "smoke", ...common, glyphs: [..."AOMi."], angles: [0, 45, 90, 135, 180, 225, 270, 315] }];
  }
  if (profile === "fine") {
    const angles = Array.from({ length: 72 }, (_, index) => index * 5);
    const glyphs = [..."AIMNOWXi.j:()0+"];
    return [
      ...[0, 0.5, 1, 1.6, 2, 3].map((marginWidth) => ({
        name: `fine-arial-10-margin-${String(marginWidth).replace(".", "p")}`,
        ...common,
        marginWidth,
        glyphs,
        angles,
      })),
      ...[0.5, 2, 4].map((lineWidth) => ({
        name: `fine-arial-10-margin-0-line-${String(lineWidth).replace(".", "p")}`,
        ...common,
        marginWidth: 0,
        lineWidth,
        glyphs,
        angles,
      })),
    ];
  }
  if (profile === "thin") {
    const angles = Array.from({ length: 72 }, (_, index) => index * 5);
    const glyphs = [..."AIMNOWXi.j:()0+"];
    return [0, 0.5, 1, 1.6, 2, 3].map((marginWidth) => ({
      name: `thin-arial-10-margin-${String(marginWidth).replace(".", "p")}`,
      ...common,
      marginWidth,
      lineWidth: 0.05,
      glyphs,
      angles,
    }));
  }
  if (profile === "holdout") {
    const angles = Array.from({ length: 36 }, (_, index) => index * 10);
    const glyphs = [..."AIMNOWXi.j:()0+"];
    return [
      ...[0.25, 0.75, 1.25, 2.5, 3.5].map((marginWidth) => ({
        name: `holdout-arial-10-margin-${String(marginWidth).replace(".", "p")}`,
        ...common,
        marginWidth,
        lineWidth: 0.05,
        glyphs,
        angles,
      })),
      { name: "holdout-arial-14-margin-2", ...common, size: 14, glyphs, angles },
      ...[0.25, 0.75, 1.5, 3].flatMap((lineWidth) => [0, 2].map((marginWidth) => ({
        name: `holdout-arial-10-margin-${marginWidth}-line-${String(lineWidth).replace(".", "p")}`,
        ...common,
        marginWidth,
        lineWidth,
        glyphs,
        angles,
      }))),
    ];
  }
  if (profile === "comprehensive") {
    const angles10 = Array.from({ length: 36 }, (_, index) => index * 10);
    const angles30 = Array.from({ length: 12 }, (_, index) => index * 30);
    const topologyLabels = [...FULL_GLYPHS, ...CHEMICAL_LABELS];
    const fontLabels = REPRESENTATIVE_LABELS.filter((label) => label.length === 1 || CHEMICAL_LABELS.includes(label));
    const compactLabels = ["A", "I", "M", "O", "W", "X", "a", "i", "j", "0", "+", "(", "Cl", "NH2", "Fe3+", "α"];
    const parameterGrid = [
      [6, 0, 0.05], [6, 0.75, 1.5], [6, 2.5, 4], [6, 4, 0.6],
      [8, 0.25, 3], [8, 1, 0.25], [8, 3, 1], [8, 4, 4],
      [10, 0, 2], [10, 0.5, 0.6], [10, 1.25, 4], [10, 3.5, 0.25],
      [12, 0.25, 1], [12, 0.75, 4], [12, 2, 0.05], [12, 4, 2],
      [14, 0, 4], [14, 0.5, 0.25], [14, 1.6, 1.5], [14, 3, 0.6],
      [18, 0.25, 0.05], [18, 1, 2], [18, 2.5, 0.6], [18, 3.5, 3],
      [24, 0, 1], [24, 0.75, 0.25], [24, 1.25, 3], [24, 4, 1.5],
    ];
    return [
      // Dense topology scans expose component-switch thresholds instead of
      // smoothing them away as a single bounding-box radius.
      ...[0, 0.25, 0.75, 1.25, 2.5, 3.5].map((marginWidth) => ({
        name: `comprehensive-topology-margin-${slug(marginWidth)}`,
        ...common,
        marginWidth,
        lineWidth: 0.05,
        glyphs: topologyLabels,
        angles: angles10,
      })),
      // Every installed family is crossed with unseen sizes and two margins.
      ...FONTS.flatMap((font) => [8, 14, 24].flatMap((size) => [0.75, 2.5].map((marginWidth) => ({
        name: `comprehensive-${slug(font.font)}-${size}-margin-${slug(marginWidth)}`,
        ...common,
        ...font,
        size,
        marginWidth,
        glyphs: fontLabels,
        angles: DEFAULT_ANGLES,
      })))),
      // Face is tested in several families so bold/italic is not mistaken for
      // an Arial-only correction.
      ...FONTS.slice(0, 4).flatMap((font) => [0, 1, 2, 3].map((face) => ({
        name: `comprehensive-${slug(font.font)}-face-${face}`,
        ...common,
        ...font,
        face,
        marginWidth: 1.6,
        glyphs: fontLabels,
        angles: DEFAULT_ANGLES,
      }))),
      // A balanced cross-grid varies all three continuous parameters without
      // paying the cost of their full Cartesian product.
      ...parameterGrid.map(([size, marginWidth, lineWidth], index) => ({
        name: `comprehensive-parameters-${String(index + 1).padStart(2, "0")}`,
        ...common,
        size,
        marginWidth,
        lineWidth,
        glyphs: compactLabels,
        angles: angles30,
      })),
    ];
  }
  if (profile === "directional") {
    const angles = Array.from({ length: 360 }, (_, index) => index);
    const labelBatches = [
      ["A", "I", "M", "O"],
      ["W", "X", "a", "i"],
      ["j", "0", "+", "("],
      ["Cl", "NH2", "Fe3+", "t-Bu"],
      ["CO2H", "NMe2", "R''", "α"],
    ];
    return [0, 0.75, 2.5].flatMap((marginWidth) => [0.05, 1].flatMap((lineWidth) => (
      labelBatches.map((glyphs, batchIndex) => ({
        name: `directional-margin-${slug(marginWidth)}-line-${slug(lineWidth)}-batch-${batchIndex + 1}`,
        ...common,
        marginWidth,
        lineWidth,
        glyphs,
        angles,
      }))
    )));
  }
  if (profile === "anchored-directional") {
    const angles = Array.from({ length: 360 }, (_, index) => index);
    const anchoredLabels = [
      ["NH2", 0, "left"], ["NH2", 1, "middle"], ["NH2", 2, "right"],
      ["Fe3+", 0, "left"], ["Fe3+", 2, "middle"], ["Fe3+", 3, "right"],
      ["CO2H", 0, "left"], ["CO2H", 1, "middle"], ["CO2H", 3, "right"],
      ["(PhO)2POH", 0, "left"], ["(PhO)2POH", 6, "middle"], ["(PhO)2POH", 8, "right"],
    ].map(([glyph, anchorIndex, anchorPosition]) => ({ glyph, anchorIndex, anchorPosition }));
    return [0, 0.75, 2.5].flatMap((marginWidth) => [0.05, 1].flatMap((lineWidth) => (
      anchoredLabels.map((labelCase) => ({
        name: `anchored-${slug(labelCase.glyph)}-${labelCase.anchorPosition}-margin-${slug(marginWidth)}-line-${slug(lineWidth)}`,
        ...common,
        marginWidth,
        lineWidth,
        glyphs: [labelCase],
        angles,
      }))
    )));
  }
  if (profile !== "survey") throw new Error(`Unsupported profile: ${profile}`);
  return [
    { name: "arial-10-margin-2", ...common, glyphs: FULL_GLYPHS, angles: DEFAULT_ANGLES },
    ...[0, 0.5, 1, 1.6, 3].map((marginWidth) => ({
      name: `arial-10-margin-${String(marginWidth).replace(".", "p")}`,
      ...common,
      marginWidth,
      glyphs: FOCUSED_GLYPHS,
      angles: DEFAULT_ANGLES,
    })),
    ...[6, 8, 12, 18].map((size) => ({
      name: `arial-${size}-margin-2`,
      ...common,
      size,
      glyphs: FOCUSED_GLYPHS,
      angles: DEFAULT_ANGLES,
    })),
    ...[0.5, 2].map((lineWidth) => ({
      name: `arial-10-margin-2-line-${String(lineWidth).replace(".", "p")}`,
      ...common,
      lineWidth,
      glyphs: FOCUSED_GLYPHS,
      angles: DEFAULT_ANGLES,
    })),
    { name: "times-10-margin-2", ...common, font: "Times New Roman", fontId: 4, glyphs: FOCUSED_GLYPHS, angles: DEFAULT_ANGLES },
    { name: "calibri-10-margin-2", ...common, font: "Calibri", fontId: 5, glyphs: FOCUSED_GLYPHS, angles: DEFAULT_ANGLES },
    { name: "arial-bold-10-margin-2", ...common, face: 1, glyphs: FOCUSED_GLYPHS, angles: DEFAULT_ANGLES },
    { name: "arial-italic-10-margin-2", ...common, face: 2, glyphs: FOCUSED_GLYPHS, angles: DEFAULT_ANGLES },
  ];
}

function makeCases(experiment) {
  const cases = [];
  for (const labelCase of experiment.glyphs) {
    const normalized = typeof labelCase === "string" ? { glyph: labelCase } : labelCase;
    for (const angleDeg of experiment.angles) cases.push({ ...normalized, angleDeg });
  }
  return cases;
}

function makeCdxml(experiment) {
  const cases = makeCases(experiment);
  const columns = 12;
  const cellWidth = 100;
  const cellHeight = 90;
  const bondLength = 32;
  const padding = 50;
  const rows = Math.ceil(cases.length / columns);
  const width = padding * 2 + columns * cellWidth;
  const height = padding * 2 + rows * cellHeight;
  let nextId = 1;
  const fragments = [];
  const metadata = [];

  for (let index = 0; index < cases.length; index += 1) {
    const { glyph, angleDeg, anchorIndex = null, anchorPosition = null } = cases[index];
    const column = index % columns;
    const row = Math.floor(index / columns);
    const target = {
      x: padding + column * cellWidth + cellWidth / 2,
      y: padding + row * cellHeight + cellHeight / 2,
    };
    const radians = angleDeg * Math.PI / 180;
    const source = {
      x: target.x + bondLength * Math.cos(radians),
      y: target.y + bondLength * Math.sin(radians),
    };
    const sourceId = nextId++;
    const targetId = nextId++;
    const bondId = nextId++;
    const fragmentId = nextId++;
    const targetNode = anchorIndex === null
      ? `<n id="${targetId}" p="${target.x.toFixed(4)} ${target.y.toFixed(4)}" NodeType="GenericNickname" GenericNickname="${xmlEscape(glyph)}" AS="N" NumHydrogens="0"/>`
      : `<n id="${targetId}" p="${target.x.toFixed(4)} ${target.y.toFixed(4)}" NodeType="Fragment" AS="N" NumHydrogens="0"><t p="${target.x.toFixed(4)} ${target.y.toFixed(4)}" LabelAlignment="Center" LabelJustification="Center" InterpretChemically="yes"><s font="${experiment.fontId}" size="${experiment.size}" face="${experiment.face}" color="0">${xmlEscape(glyph)}</s></t></n>`;
    const attachAttribute = anchorIndex === null ? "" : ` EndAttach="${anchorIndex}"`;
    fragments.push(`<fragment id="${fragmentId}"><n id="${sourceId}" p="${source.x.toFixed(4)} ${source.y.toFixed(4)}" AS="N"/>${targetNode}<b id="${bondId}" B="${sourceId}" E="${targetId}"${attachAttribute}/></fragment>`);
    metadata.push({ index, glyph, anchorIndex, anchorPosition, angleDeg, source, target, bondLength, sourceId, targetId, bondId, fragmentId });
  }

  const fontTable = [
    '<font id="3" charset="iso-8859-1" name="Arial"/>',
    '<font id="4" charset="iso-8859-1" name="Times New Roman"/>',
    '<font id="5" charset="iso-8859-1" name="Calibri"/>',
    '<font id="6" charset="iso-8859-1" name="Cambria"/>',
    '<font id="7" charset="iso-8859-1" name="Segoe UI"/>',
    '<font id="8" charset="iso-8859-1" name="Courier New"/>',
    '<font id="9" charset="Symbol" name="Symbol"/>',
  ].join("");
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd">
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 ${width} ${height}" FractionalWidths="yes" InterpretChemically="yes" LabelFont="${experiment.fontId}" LabelSize="${experiment.size}" LabelFace="${experiment.face}" CaptionFont="${experiment.fontId}" CaptionSize="${experiment.size}" MarginWidth="${experiment.marginWidth}" LineWidth="${experiment.lineWidth}" BoldWidth="4" BondLength="${bondLength}" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable>${fontTable}</fonttable>
<page id="${nextId}" BoundingBox="0 0 ${width} ${height}" Width="${width}" Height="${height}">${fragments.join("\n")}</page>
</CDXML>
`;
  return { xml, metadata, width, height };
}

function parseMatrix(value) {
  const numbers = value.match(/[-+]?(?:\d*\.)?\d+(?:[eE][-+]?\d+)?/g)?.map(Number) ?? [];
  if (numbers.length !== 6) throw new Error(`Unsupported SVG transform: ${value}`);
  return numbers;
}

function transformPoint(matrix, point) {
  const [a, b, c, d, e, f] = matrix;
  return { x: a * point.x + c * point.y + e, y: b * point.x + d * point.y + f };
}

function lineAngleDifferenceDeg(left, right) {
  let difference = Math.abs(left - right) % 180;
  if (difference > 90) difference = 180 - difference;
  return difference;
}

function principalAxisMeasurement(points) {
  const center = points.reduce((sum, point) => ({ x: sum.x + point.x, y: sum.y + point.y }), { x: 0, y: 0 });
  center.x /= points.length;
  center.y /= points.length;
  let xx = 0;
  let xy = 0;
  let yy = 0;
  for (const point of points) {
    const x = point.x - center.x;
    const y = point.y - center.y;
    xx += x * x;
    xy += x * y;
    yy += y * y;
  }
  const angleRadians = 0.5 * Math.atan2(2 * xy, xx - yy);
  const axis = { x: Math.cos(angleRadians), y: Math.sin(angleRadians) };
  const projections = points.map((point) => point.x * axis.x + point.y * axis.y);
  return {
    angleDeg: angleRadians * 180 / Math.PI,
    length: Math.max(...projections) - Math.min(...projections),
  };
}

function parseLinearPath(element) {
  const transformMatch = element.match(/\btransform="([^"]+)"/);
  const dataMatch = element.match(/\bd="([^"]+)"/);
  if (!transformMatch || !dataMatch) return null;
  const matrix = parseMatrix(transformMatch[1]);
  const tokens = [...dataMatch[1].matchAll(/([MLZ])|([-+]?(?:\d*\.)?\d+(?:[eE][-+]?\d+)?)/gi)].map((match) => match[1] ?? Number(match[2]));
  const points = [];
  let index = 0;
  while (index < tokens.length) {
    const command = tokens[index++];
    if (command === "Z" || command === "z") continue;
    if (command !== "M" && command !== "m" && command !== "L" && command !== "l") return null;
    if (typeof tokens[index] !== "number" || typeof tokens[index + 1] !== "number") return null;
    points.push(transformPoint(matrix, { x: tokens[index++], y: tokens[index++] }));
  }
  return { matrix, points };
}

function parseMeasurements(svg, experiment, metadata) {
  const pathElements = [...svg.matchAll(/<path\b[^>]*\bd="[^"]+"[^>]*\/?\s*>/g)]
    .map((match) => parseLinearPath(match[0]))
    .filter((entry) => entry?.points.length >= 2);
  if (pathElements.length !== metadata.length) {
    throw new Error(`${experiment.name}: expected ${metadata.length} simple bond paths, found ${pathElements.length}`);
  }
  return metadata.map((probe, index) => {
    const pathData = pathElements[index];
    const radians = probe.angleDeg * Math.PI / 180;
    const axis = { x: Math.cos(radians), y: Math.sin(radians) };
    const projections = pathData.points.map((point) => point.x * axis.x + point.y * axis.y);
    const visibleLengthSvg = Math.max(...projections) - Math.min(...projections);
    const svgPerPoint = Math.hypot(pathData.matrix[0], pathData.matrix[1]) * 20;
    const visibleLength = visibleLengthSvg / svgPerPoint;
    const retreat = probe.bondLength - visibleLength;
    const actual = principalAxisMeasurement(pathData.points);
    const actualVisibleLength = actual.length / svgPerPoint;
    return {
      ...probe,
      font: experiment.font,
      size: experiment.size,
      face: experiment.face,
      marginWidth: experiment.marginWidth,
      lineWidth: experiment.lineWidth,
      visibleLength: Number(visibleLength.toFixed(5)),
      retreat: Number(retreat.toFixed(5)),
      measurementMode: probe.anchorIndex === null ? "label-retreat" : "attached-label-effective-displacement",
      actualVisibleLength: Number(actualVisibleLength.toFixed(5)),
      effectiveEndpointDisplacement: Number((probe.bondLength - actualVisibleLength).toFixed(5)),
      angularDeflectionDeg: Number(lineAngleDifferenceDeg(actual.angleDeg, probe.angleDeg).toFixed(5)),
      svgPerPoint: Number(svgPerPoint.toFixed(5)),
    };
  });
}

function quantile(sorted, fraction) {
  if (!sorted.length) return null;
  const position = (sorted.length - 1) * fraction;
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) return sorted[lower];
  return sorted[lower] * (upper - position) + sorted[upper] * (position - lower);
}

function summarize(measurements) {
  const values = measurements.map((entry) => entry.retreat).sort((a, b) => a - b);
  return {
    count: values.length,
    min: values[0],
    median: quantile(values, 0.5),
    p95: quantile(values, 0.95),
    max: values.at(-1),
  };
}

function summarizeCoverage(measurements) {
  const distinct = (field) => [...new Set(measurements.map((entry) => entry[field]))]
    .sort((left, right) => typeof left === "number" ? left - right : String(left).localeCompare(String(right)));
  return {
    fonts: distinct("font"),
    sizes: distinct("size"),
    faces: distinct("face"),
    marginWidths: distinct("marginWidth"),
    lineWidths: distinct("lineWidth"),
    angleDegrees: distinct("angleDeg"),
    labels: distinct("glyph"),
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/chemdraw-label-retreat-probe.mjs [--profile smoke|survey|fine|thin|holdout|comprehensive|directional|anchored-directional] [--out dir] [--no-export]");
    return;
  }
  const outDir = path.resolve(args.outDir);
  const inputDir = path.join(outDir, "input");
  const oracleDir = path.join(outDir, "oracle");
  await fs.mkdir(inputDir, { recursive: true });
  await fs.mkdir(oracleDir, { recursive: true });
  const experiments = experimentSets(args.profile);
  const jobs = [];
  for (const experiment of experiments) {
    const generated = makeCdxml(experiment);
    const inputPath = path.join(inputDir, `${experiment.name}.cdxml`);
    const metadataPath = path.join(inputDir, `${experiment.name}.json`);
    await fs.writeFile(inputPath, generated.xml, "utf8");
    await fs.writeFile(metadataPath, `${JSON.stringify({ experiment, probes: generated.metadata }, null, 2)}\n`, "utf8");
    jobs.push({ experiment, inputPath, metadataPath, probes: generated.metadata });
  }
  if (args.exportOracle) {
    await generateChemDrawOracle({
      outDir: oracleDir,
      formats: ["svg", "cdxml"],
      inputs: jobs.map((job) => job.inputPath),
      outputNames: jobs.map((job) => job.experiment.name),
    });
  }
  const allMeasurements = [];
  const summaries = [];
  for (const job of jobs) {
    const svgPath = path.join(oracleDir, `${job.experiment.name}.chemdraw.svg`);
    const svg = await fs.readFile(svgPath, "utf8");
    const measurements = parseMeasurements(svg, job.experiment, job.probes);
    allMeasurements.push(...measurements);
    summaries.push({ experiment: job.experiment, ...summarize(measurements) });
  }
  const result = {
    generatedAt: new Date().toISOString(),
    profile: args.profile,
    coverage: summarizeCoverage(allMeasurements),
    summaries,
    measurements: allMeasurements,
  };
  const resultPath = path.join(outDir, "measurements.json");
  await fs.writeFile(resultPath, `${JSON.stringify(result, null, 2)}\n`, "utf8");
  console.log(JSON.stringify({ resultPath, experiments: summaries.length, measurements: allMeasurements.length, summaries }, null, 2));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exit(1);
  });
}
