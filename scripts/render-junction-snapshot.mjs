import fs from "node:fs/promises";
import path from "node:path";
import init, { WasmEngine } from "../viewer/engine/chemcore_engine.js";

const BOND_STROKE = 0.035;
const CHEMDRAW_INK = "#000000";

const output = process.argv[2] || path.resolve("tmp/junction-endpoint-ring.svg");
const jsonOutput = output.replace(/\.svg$/i, ".json");

function px(value) {
  return value / 37.8;
}

function primitiveStrokeWidthValue(primitive, fallback = 0) {
  const strokeWidth = primitive?.strokeWidth ?? primitive?.stroke_width;
  const numeric = Number(strokeWidth);
  return Number.isFinite(numeric) ? numeric : fallback;
}

function escapeAttr(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("\"", "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function attrsToString(attrs) {
  return Object.entries(attrs)
    .filter(([, value]) => value !== undefined && value !== null)
    .map(([key, value]) => `${key}="${escapeAttr(value)}"`)
    .join(" ");
}

function primitivePoints(primitive) {
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    return [primitive.from, primitive.to];
  }
  if (
    (primitive.kind === "polygon" || primitive.kind === "polyline") &&
    Array.isArray(primitive.points)
  ) {
    return primitive.points;
  }
  return [];
}

function renderCorePrimitiveSvg(primitive) {
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    return `<line ${attrsToString({
      x1: primitive.from.x,
      y1: primitive.from.y,
      x2: primitive.to.x,
      y2: primitive.to.y,
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "stroke-linecap": primitive.role === "document-bond" ? "butt" : undefined,
      "stroke-linejoin": primitive.role === "document-bond" ? "miter" : undefined,
      "data-bond-id": primitive.bondId || undefined,
    })}/>`;
  }
  if (primitive.kind === "polyline" && Array.isArray(primitive.points)) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    return `<polyline ${attrsToString({
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: "none",
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "stroke-dasharray": (primitive.dashArray || primitive.dash_array)?.join(" ") || undefined,
      "stroke-linecap": primitive.lineCap || primitive.line_cap || "butt",
      "stroke-linejoin": primitive.lineJoin || primitive.line_join || "miter",
      "data-bond-id": primitive.bondId || undefined,
    })}/>`;
  }
  if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    return `<polygon ${attrsToString({
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: primitive.fill || CHEMDRAW_INK,
      stroke: strokeWidth > 0 ? primitive.stroke || primitive.fill || CHEMDRAW_INK : "none",
      "stroke-width": strokeWidth,
      "data-bond-id": primitive.bondId || undefined,
    })}/>`;
  }
  return "";
}

function renderSvg(primitives) {
  const points = primitives.flatMap(primitivePoints);
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const pad = 0.5;
  const viewBox = [
    minX - pad,
    minY - pad,
    maxX - minX + pad * 2,
    maxY - minY + pad * 2,
  ];
  const body = primitives.map(renderCorePrimitiveSvg).filter(Boolean).join("\n  ");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="900" height="620" viewBox="${viewBox.join(
    " ",
  )}">
  <rect x="${viewBox[0]}" y="${viewBox[1]}" width="${viewBox[2]}" height="${viewBox[3]}" fill="#ffffff"/>
  ${body}
</svg>
`;
}

await init(await fs.readFile(new URL("../viewer/engine/chemcore_engine_bg.wasm", import.meta.url)));

const engine = new WasmEngine();
engine.setTool("bond", "single");
engine.pointerDown(px(300), px(260), false);
engine.pointerUp(px(300), px(260), false);

const state = JSON.parse(engine.stateJson());
const endpoint = state.document.resources.mol_editor.data.nodes.find((node) => node.id === "n_2");

engine.setTool("templates", "single");
engine.setTemplate("ring-3");
engine.pointerDown(endpoint.position[0], endpoint.position[1], false);
engine.pointerUp(endpoint.position[0], endpoint.position[1], false);

const primitives = JSON.parse(engine.renderListJson()).filter(
  (primitive) => primitive.role === "document-bond",
);

await fs.mkdir(path.dirname(output), { recursive: true });
await fs.writeFile(jsonOutput, `${JSON.stringify(primitives, null, 2)}\n`);
await fs.writeFile(output, renderSvg(primitives));

const patchCount = primitives.filter(
  (primitive) => primitive.kind === "polygon" && !primitive.bondId,
).length;
console.log(`${output}`);
console.log(`${jsonOutput}`);
console.log(`document-bond primitives: ${primitives.length}, center patches: ${patchCount}`);
