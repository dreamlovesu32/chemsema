import { mkdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const defaultFixtureDir = join(rootDir, "tmp", "stability", "fixtures");

function fixed(value, places = 2) {
  return Number(value.toFixed(places));
}

function makeSceneObject({
  id,
  type,
  zIndex,
  translate = [0, 0],
  rotate = 0,
  scale = [1, 1],
  styleRef = null,
  payload = {},
  meta = null,
  children = [],
}) {
  return {
    id,
    type,
    name: id.replace(/^obj_/, "").replaceAll("_", " "),
    visible: true,
    locked: false,
    zIndex,
    transform: { translate, rotate, scale },
    ...(styleRef ? { styleRef } : {}),
    payload,
    meta,
    children,
  };
}

function makeLargeFragment(nodeCount) {
  const columns = 96;
  const spacing = 20;
  const nodes = [];
  const bonds = [];
  const elementCycle = [
    ["C", 6],
    ["C", 6],
    ["N", 7],
    ["O", 8],
    ["S", 16],
    ["P", 15],
  ];
  for (let index = 0; index < nodeCount; index += 1) {
    const column = index % columns;
    const row = Math.floor(index / columns);
    const [element, atomicNumber] = elementCycle[index % elementCycle.length];
    const x = 48 + column * spacing;
    const y = 48 + row * spacing + (column % 2) * 5;
    nodes.push({
      id: `n${index}`,
      element,
      atomicNumber,
      position: [fixed(x), fixed(y)],
      charge: 0,
      numHydrogens: 0,
      meta: null,
    });
    if (index > 0 && column !== 0) {
      bonds.push({
        id: `b${bonds.length}`,
        begin: `n${index - 1}`,
        end: `n${index}`,
        order: index % 17 === 0 ? 2 : 1,
        strokeWidth: 0.85,
        meta: null,
      });
    }
    if (row > 0 && column % 13 === 0) {
      bonds.push({
        id: `b${bonds.length}`,
        begin: `n${index - columns}`,
        end: `n${index}`,
        order: 1,
        strokeWidth: 0.85,
        meta: null,
      });
    }
  }
  const rows = Math.ceil(nodeCount / columns);
  return {
    bbox: [0, 0, 120 + columns * spacing, 120 + rows * spacing],
    nodes,
    bonds,
  };
}

function makeTextObject(index, x, y) {
  const text = index % 3 === 0 ? "H2O / MeOH" : index % 3 === 1 ? "Yield 86%" : "n = 3";
  return makeSceneObject({
    id: `obj_text_${index}`,
    type: "text",
    zIndex: 200 + index,
    translate: [x, y],
    styleRef: "style_text_default",
    payload: {
      bbox: [0, 0, 72, 16],
      text,
      fontSize: 10,
      lineHeight: 12,
      align: "left",
      preserveLines: true,
    },
  });
}

function makeLineObject(index, x, y) {
  return makeSceneObject({
    id: `obj_arrow_${index}`,
    type: "line",
    zIndex: 260 + index,
    styleRef: "style_line_default",
    payload: {
      kind: "arrow",
      points: [[x, y], [x + 72, y + (index % 2 ? 18 : 0)]],
      stroke: "#111111",
      strokeWidth: 1,
      head: "end",
      tail: "none",
      arrowHead: {
        head: "full",
        tail: "none",
        size: "small",
        curve: index % 4 === 0 ? 90 : 0,
      },
    },
  });
}

function makeShapeObject(index, x, y) {
  const kinds = ["rect", "ellipse", "round-rect"];
  return makeSceneObject({
    id: `obj_shape_${index}`,
    type: "shape",
    zIndex: 320 + index,
    translate: [x, y],
    styleRef: "style_shape_default",
    payload: {
      kind: kinds[index % kinds.length],
      bbox: [0, 0, 46 + (index % 3) * 8, 28 + (index % 2) * 12],
      stroke: "#111111",
      fill: index % 5 === 0 ? "#f2f7ff" : null,
      strokeWidth: 1,
    },
  });
}

function makeBracketGroup(index, x, y) {
  const height = 52 + (index % 3) * 12;
  const width = 92 + (index % 2) * 18;
  const kind = index % 2 === 0 ? "square" : "round";
  const left = makeSceneObject({
    id: `obj_bracket_${index}_left`,
    type: "bracket",
    zIndex: 400 + index * 3 + 1,
    translate: [x, y],
    payload: {
      bbox: [0, 0, 16, height],
      kind,
      side: "left",
      stroke: "#111111",
      strokeWidth: 1,
    },
  });
  const right = makeSceneObject({
    id: `obj_bracket_${index}_right`,
    type: "bracket",
    zIndex: 400 + index * 3 + 2,
    translate: [x + width, y],
    payload: {
      bbox: [0, 0, 16, height],
      kind,
      side: "right",
      stroke: "#111111",
      strokeWidth: 1,
    },
  });
  return makeSceneObject({
    id: `obj_bracket_group_${index}`,
    type: "group",
    zIndex: 400 + index * 3,
    payload: { bbox: [x, y, width + 16, height] },
    meta: { kind: "bracket-group" },
    children: [left, right],
  });
}

function makeSymbolObject(index, x, y) {
  const kinds = ["circle-plus", "plus", "lone-pair", "electron", "circle-minus", "radical-cation"];
  return makeSceneObject({
    id: `obj_symbol_${index}`,
    type: "symbol",
    zIndex: 520 + index,
    translate: [x, y],
    payload: {
      bbox: [0, 0, 16, 16],
      kind: kinds[index % kinds.length],
      fill: "#111111",
      stroke: "#111111",
      strokeWidth: 1,
    },
  });
}

export function makeSyntheticLargeDocument(options = {}) {
  const nodeCount = Math.max(50, Number(options.nodeCount || 6400));
  const objectRepeats = Math.max(8, Number(options.objectRepeats || 48));
  const fragment = makeLargeFragment(nodeCount);
  const [pageWidth, moleculeHeight] = [Math.max(2200, fragment.bbox[2] + 180), fragment.bbox[3] + 80];
  const objects = [
    makeSceneObject({
      id: "obj_large_molecule",
      type: "molecule",
      zIndex: 10,
      styleRef: "style_molecule_default",
      payload: {
        resourceRef: "mol_large",
        bbox: fragment.bbox,
        extra: {},
      },
    }),
  ];

  for (let index = 0; index < objectRepeats; index += 1) {
    const column = index % 12;
    const row = Math.floor(index / 12);
    const x = 72 + column * 160;
    const y = moleculeHeight + 80 + row * 110;
    objects.push(makeTextObject(index, x, y));
    objects.push(makeLineObject(index, x + 78, y + 10));
    objects.push(makeShapeObject(index, x + 8, y + 34));
    objects.push(makeBracketGroup(index, x + 78, y + 34));
    objects.push(makeSymbolObject(index, x + 124, y + 6));
  }

  const objectRows = Math.ceil(objectRepeats / 12);
  const pageHeight = moleculeHeight + 160 + objectRows * 118;
  return {
    format: { name: "chemsema", version: "0.1", unit: "pt" },
    document: {
      id: "doc_synthetic_stability_large",
      title: "Synthetic ChemSema stability fixture",
      page: { width: pageWidth, height: pageHeight, background: "#ffffff" },
      meta: { synthetic: true, generatedFor: "chemsema-stability" },
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        fill: "#000000",
        strokeWidth: 0.85,
        fontFamily: "Arial",
        fontSize: 11,
      },
      style_text_default: {
        kind: "text",
        fill: "#111111",
        fontFamily: "Arial",
        fontSize: 10,
      },
      style_line_default: {
        kind: "line",
        stroke: "#111111",
        strokeWidth: 1,
      },
      style_shape_default: {
        kind: "shape",
        stroke: "#111111",
        fill: null,
        strokeWidth: 1,
      },
    },
    objects,
    resources: {
      mol_large: {
        id: "mol_large",
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: {
          schema: "chemsema.molecule.fragment2d",
          bbox: fragment.bbox,
          nodes: fragment.nodes,
          bonds: fragment.bonds,
        },
      },
    },
  };
}

export function makeAgentCommandScript() {
  return [
    { type: "apply-document-style", preset: "acs-document-1996" },
    {
      type: "add-bond",
      begin: { x: 110, y: 120 },
      end: { x: 154, y: 120 },
      order: 1,
      variant: "single",
    },
    {
      type: "add-bond",
      begin: { x: 154, y: 120 },
      end: { x: 198, y: 144 },
      order: 2,
      variant: "double",
    },
    {
      type: "add-text",
      position: { x: 120, y: 182 },
      text: "H2O + EtOH",
      fontFamily: "Arial",
      fontSize: 10,
      fill: "#000000",
      defaultChemical: true,
    },
    {
      type: "add-arrow",
      begin: { x: 250, y: 130 },
      end: { x: 360, y: 130 },
      variant: "solid",
      headSize: "small",
      curve: "arc270",
      headStyle: "full",
      tailStyle: "none",
      head: true,
      tail: false,
      bold: false,
      noGo: "none",
    },
    {
      type: "add-shape",
      kind: "rect",
      style: "solid",
      color: "#111111",
      begin: { x: 390, y: 104 },
      end: { x: 460, y: 150 },
    },
    {
      type: "add-bracket",
      kind: "square",
      begin: { x: 492, y: 96 },
      end: { x: 588, y: 166 },
    },
    {
      type: "add-symbol",
      kind: "circle-plus",
      center: { x: 628, y: 132 },
    },
    {
      type: "add-element",
      symbol: "Cl",
      atomicNumber: 17,
      center: { x: 670, y: 132 },
    },
    {
      type: "add-orbital",
      template: "p",
      style: "shaded",
      phase: "plus",
      color: "#111111",
      center: { x: 730, y: 132 },
      end: { x: 780, y: 178 },
    },
    {
      type: "inspect-document",
      include: ["summary", "objects", "molecules", "resources", "styles"],
    },
    { type: "export-document", format: "svg" },
  ];
}

export function writeStabilityFixtures(options = {}) {
  const fixtureDir = options.fixtureDir || process.env.CHEMSEMA_STABILITY_FIXTURE_DIR || defaultFixtureDir;
  const nodeCount = Number(options.nodeCount || process.env.CHEMSEMA_STABILITY_SYNTHETIC_NODES || 6400);
  const objectRepeats = Number(options.objectRepeats || process.env.CHEMSEMA_STABILITY_SYNTHETIC_OBJECT_REPEATS || 48);
  mkdirSync(fixtureDir, { recursive: true });
  const largeDocument = makeSyntheticLargeDocument({ nodeCount, objectRepeats });
  const commandScript = makeAgentCommandScript();
  const largePath = join(fixtureDir, "synthetic-large.ccjs");
  const commandsPath = join(fixtureDir, "synthetic-agent-commands.json");
  writeFileSync(largePath, `${JSON.stringify(largeDocument, null, 2)}\n`, "utf8");
  writeFileSync(commandsPath, `${JSON.stringify(commandScript, null, 2)}\n`, "utf8");
  return {
    fixtureDir,
    files: {
      syntheticLargeCcjs: largePath,
      syntheticAgentCommands: commandsPath,
    },
    synthetic: {
      nodes: largeDocument.resources.mol_large.data.nodes.length,
      bonds: largeDocument.resources.mol_large.data.bonds.length,
      objects: largeDocument.objects.length,
      bytes: statSync(largePath).size,
    },
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const manifest = writeStabilityFixtures();
  console.log(JSON.stringify(manifest, null, 2));
}
