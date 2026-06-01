import fs from "node:fs/promises";
import path from "node:path";
import { launchBrowser } from "./playwright-browser.mjs";

const url = process.argv[2] || "http://127.0.0.1:8765/viewer/";
const outputBase = process.argv[3] || path.resolve("tmp/frontend-junction-debug");
const forceWideAtCenter = !process.argv.includes("--keep-wedge-direction");

function escapeAttr(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("\"", "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
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

function svgFromDomPolygons(domPolygons) {
  const points = domPolygons.flatMap((polygon) => polygon.parsedPoints);
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  const pad = 0.2;
  const viewBox = [
    minX - pad,
    minY - pad,
    maxX - minX + pad * 2,
    maxY - minY + pad * 2,
  ];
  const body = domPolygons
    .map(
      (polygon) =>
        `<polygon points="${escapeAttr(polygon.points)}" fill="${escapeAttr(
          polygon.fill || "#000000",
        )}" stroke="${escapeAttr(polygon.stroke || "none")}" stroke-width="${escapeAttr(
          polygon.strokeWidth || "0",
        )}" data-bond-id="${escapeAttr(polygon.bondId || "")}"/>`,
    )
    .join("\n  ");
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="900" height="680" viewBox="${viewBox.join(
    " ",
  )}">
  <rect x="${viewBox[0]}" y="${viewBox[1]}" width="${viewBox[2]}" height="${viewBox[3]}" fill="#ffffff"/>
  ${body}
</svg>
`;
}

await fs.mkdir(path.dirname(outputBase), { recursive: true });

const browser = await launchBrowser({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1100, height: 900 },
  deviceScaleFactor: 2,
});

await page.goto(url, { waitUntil: "networkidle" });
await page.waitForFunction(() => window.__chemcoreDebug?.state?.editorEngine);

const result = await page.evaluate((forceWideAtCenter) => {
  const debug = window.__chemcoreDebug;
  const engine = debug.state.editorEngine;
  const draw = (variant, x1, y1, x2, y2) => {
    engine.setTool("bond", variant);
    engine.pointerDown(x1, y1, false);
    engine.pointerMove(x2, y2, false);
    engine.pointerUp(x2, y2, false);
    debug.syncDocument();
  };

  draw("single", 7.5, 6.5, 6.45, 6.5);
  draw("single", 7.5, 6.5, 8.03, 5.58);
  draw("wedge", 7.5, 6.5, 8.03, 7.42);

  if (forceWideAtCenter) {
    const documentData = JSON.parse(engine.documentJson());
    const bonds = documentData.resources.mol_editor.data.bonds;
    const wedge = bonds.find((bond) => bond.stereo?.kind === "solid-wedge");
    if (wedge) {
      wedge.stereo.wideEnd = wedge.begin === "n_1" ? "begin" : "end";
      engine.loadDocumentJson(JSON.stringify(documentData));
      debug.syncDocument();
    }
  }

  const primitives = JSON.parse(engine.renderListJson()).filter(
    (primitive) => primitive.role === "document-bond",
  );
  const domPolygons = Array.from(document.querySelectorAll("#viewer-svg polygon")).map((node) => {
    const points = node.getAttribute("points") || "";
    return {
      points,
      parsedPoints: points
        .trim()
        .split(/\s+/)
        .filter(Boolean)
        .map((pair) => {
          const [x, y] = pair.split(",").map(Number);
          return { x, y };
        }),
      fill: node.getAttribute("fill"),
      stroke: node.getAttribute("stroke"),
      strokeWidth: node.getAttribute("stroke-width"),
      bondId: node.getAttribute("data-bond-id"),
      className: node.getAttribute("class"),
    };
  });
  return {
    document: JSON.parse(engine.documentJson()),
    primitives,
    domPolygons,
  };
}, forceWideAtCenter);

const primitivePointStrings = result.primitives
  .filter((primitive) => primitive.kind === "polygon")
  .map((primitive) => (primitive.points || []).map((point) => `${point.x},${point.y}`).join(" "));
const domPointStrings = result.domPolygons.map((polygon) =>
  polygon.parsedPoints.map((point) => `${point.x},${point.y}`).join(" "),
);
const domMatchesPrimitives =
  primitivePointStrings.length === domPointStrings.length &&
  primitivePointStrings.every((points, index) => points === domPointStrings[index]);

await fs.writeFile(`${outputBase}.json`, `${JSON.stringify(result, null, 2)}\n`);
await fs.writeFile(
  `${outputBase}-primitives.json`,
  `${JSON.stringify(result.primitives, null, 2)}\n`,
);
await fs.writeFile(`${outputBase}.svg`, svgFromDomPolygons(result.domPolygons));
await page.locator("#viewer-svg").screenshot({ path: `${outputBase}.png` });

await browser.close();

const primitivePointCount = result.primitives.flatMap(primitivePoints).length;
console.log(`${outputBase}.svg`);
console.log(`${outputBase}.json`);
console.log(`${outputBase}-primitives.json`);
console.log(`${outputBase}.png`);
console.log(
  `document-bond primitives: ${result.primitives.length}, primitive points: ${primitivePointCount}, DOM matches primitives: ${domMatchesPrimitives}`,
);
