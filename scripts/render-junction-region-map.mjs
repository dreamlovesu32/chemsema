import fs from "node:fs/promises";
import path from "node:path";

const input = process.argv[2] || path.resolve("tmp/junction-endpoint-ring.json");
const output = process.argv[3] || path.resolve("tmp/junction-endpoint-ring-regions.svg");
const closeup = process.argv.includes("--closeup");
const WIDTH = 980;
const HEIGHT = 680;
const TOLERANCE = 1.0e-6;

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("\"", "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function distance(a, b) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function samePoint(a, b) {
  return distance(a, b) <= TOLERANCE;
}

function pointKey(point) {
  return `${point.x.toFixed(12)},${point.y.toFixed(12)}`;
}

function polygonCentroid(points) {
  const sum = points.reduce(
    (acc, point) => ({ x: acc.x + point.x, y: acc.y + point.y }),
    { x: 0, y: 0 },
  );
  return { x: sum.x / points.length, y: sum.y / points.length };
}

function pointString(points, project) {
  return points
    .map((point) => {
      const projected = project(point);
      return `${projected.x.toFixed(2)},${projected.y.toFixed(2)}`;
    })
    .join(" ");
}

const primitives = JSON.parse(await fs.readFile(input, "utf8"));
const polygons = primitives.filter(
  (primitive) => primitive.kind === "polygon" && Array.isArray(primitive.points),
);
const patch = polygons.find((primitive) => !primitive.bondId);
if (!patch) {
  throw new Error("No center patch polygon found in primitive JSON.");
}

const patchPoints = patch.points;
const incident = polygons.filter(
  (primitive) =>
    primitive.bondId &&
    primitive.points.filter((point) =>
      patchPoints.some((patchPoint) => samePoint(point, patchPoint)),
    ).length >= 2,
);
const nonIncident = polygons.filter(
  (primitive) => primitive.bondId && !incident.includes(primitive),
);

const drawnPoints = closeup
  ? patchPoints
  : [...incident, patch].flatMap((primitive) => primitive.points);
const minX = Math.min(...drawnPoints.map((point) => point.x));
const maxX = Math.max(...drawnPoints.map((point) => point.x));
const minY = Math.min(...drawnPoints.map((point) => point.y));
const maxY = Math.max(...drawnPoints.map((point) => point.y));
const pad = closeup ? 0.09 : 0.16;
const worldWidth = maxX - minX + pad * 2;
const worldHeight = maxY - minY + pad * 2;
const scale = Math.min((WIDTH - 120) / worldWidth, (HEIGHT - 120) / worldHeight);
const offsetX = (WIDTH - worldWidth * scale) / 2;
const offsetY = (HEIGHT - worldHeight * scale) / 2;

function project(point) {
  return {
    x: offsetX + (point.x - minX + pad) * scale,
    y: offsetY + (point.y - minY + pad) * scale,
  };
}

const colors = ["#ffd6d6", "#fff3a6", "#c9f7ff", "#d7f8d2"];
const labels = patchPoints.map((point, index) => ({
  point,
  label: `P${index + 1}`,
}));
const uniquePoints = new Map();
for (const primitive of incident) {
  for (const point of primitive.points) {
    const key = pointKey(point);
    uniquePoints.set(key, point);
  }
}
const centerCandidates = new Map();
for (const primitive of incident) {
  primitive.points.forEach((point, index) => {
    const previous = primitive.points[(index + primitive.points.length - 1) % primitive.points.length];
    const next = primitive.points[(index + 1) % primitive.points.length];
    if (
      !patchPoints.some((patchPoint) => samePoint(point, patchPoint)) &&
      patchPoints.some((patchPoint) => samePoint(previous, patchPoint)) &&
      patchPoints.some((patchPoint) => samePoint(next, patchPoint))
    ) {
      const key = pointKey(point);
      const candidate = centerCandidates.get(key) ?? { point, count: 0 };
      candidate.count += 1;
      centerCandidates.set(key, candidate);
    }
  });
}
const centerPoint = Array.from(centerCandidates.values()).sort((a, b) => b.count - a.count)[0]
  ?.point;

const centerPatchCentroid = polygonCentroid(patchPoints);
const regions = incident
  .slice()
  .sort((a, b) => polygonCentroid(a.points).x - polygonCentroid(b.points).x)
  .map((primitive, index) => ({
    primitive,
    color: colors[index % colors.length],
  }));

const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">
  <metadata>${escapeXml(JSON.stringify({ input, patchPoints, incidentBondIds: incident.map((p) => p.bondId) }))}</metadata>
  <style>
    text { font-family: Arial, Helvetica, sans-serif; font-size: 16px; fill: #000; }
    .small { font-size: 12px; }
    .region { stroke: none; opacity: 0.9; }
    .other { fill: none; stroke: #aaa; stroke-width: 2; stroke-dasharray: 7 5; }
    .outline { fill: none; stroke: #000; stroke-width: 3; stroke-linejoin: round; stroke-linecap: butt; }
    .connector { fill: none; stroke: #000; stroke-width: 4; stroke-linejoin: round; stroke-linecap: butt; }
    .dot { fill: #000; stroke: #000; stroke-width: 2; }
    .patch-fill { fill: #e6e6e6; opacity: 0.75; stroke: none; }
  </style>
  <rect width="${WIDTH}" height="${HEIGHT}" fill="#fff"/>

  ${nonIncident
    .map(
      (primitive) =>
        `<polygon class="other" points="${pointString(primitive.points, project)}"/>`,
    )
    .join("\n  ")}

  ${regions
    .map(
      ({ primitive, color }) =>
        `<polygon class="region" fill="${color}" points="${pointString(primitive.points, project)}"/>`,
    )
    .join("\n  ")}
  <polygon class="patch-fill" points="${pointString(patchPoints, project)}"/>

  ${regions
    .map(
      ({ primitive }) =>
        `<polygon class="outline" points="${pointString(primitive.points, project)}"/>`,
    )
    .join("\n  ")}
  <polygon class="connector" points="${pointString(patchPoints, project)}"/>

  ${labels
    .map(({ point, label }, index) => {
      const projected = project(point);
      const labelOffsets = closeup
        ? [
            { x: 26, y: -60 },
            { x: -220, y: 70 },
            { x: 34, y: 94 },
          ]
        : [
            { x: 12, y: -10 },
            { x: 12, y: 18 },
            { x: 12, y: 46 },
          ];
      const offset = labelOffsets[index] ?? { x: 12, y: 18 };
      const labelX = projected.x + offset.x;
      const labelY = projected.y + offset.y;
      return `<line x1="${projected.x.toFixed(2)}" y1="${projected.y.toFixed(2)}" x2="${labelX.toFixed(2)}" y2="${labelY.toFixed(2)}" stroke="#000" stroke-width="1.5"/>
  <circle class="dot" cx="${projected.x.toFixed(2)}" cy="${projected.y.toFixed(2)}" r="7"/>
  <text x="${labelX.toFixed(2)}" y="${labelY.toFixed(2)}">${label}</text>
  <text class="small" x="${labelX.toFixed(2)}" y="${(labelY + 18).toFixed(2)}">${point.x.toFixed(6)}, ${point.y.toFixed(6)}</text>`;
    })
    .join("\n  ")}

  ${
    centerPoint
      ? (() => {
          const projected = project(centerPoint);
          const labelX = projected.x + 74;
          const labelY = projected.y - 54;
          return `<line x1="${projected.x.toFixed(2)}" y1="${projected.y.toFixed(2)}" x2="${labelX.toFixed(2)}" y2="${labelY.toFixed(2)}" stroke="#000" stroke-width="1.5"/>
  <circle class="dot" cx="${projected.x.toFixed(2)}" cy="${projected.y.toFixed(2)}" r="7"/>
  <text x="${labelX.toFixed(2)}" y="${labelY.toFixed(2)}">C</text>
  <text class="small" x="${labelX.toFixed(2)}" y="${(labelY + 18).toFixed(2)}">${centerPoint.x.toFixed(6)}, ${centerPoint.y.toFixed(6)}</text>`;
        })()
      : ""
  }

  ${regions
    .map(({ primitive, color }) => {
      const centroid = project(polygonCentroid(primitive.points));
      return `<text x="${centroid.x.toFixed(2)}" y="${centroid.y.toFixed(2)}" fill="${color}" stroke="#fff" stroke-width="4" paint-order="stroke">${primitive.bondId}</text>`;
    })
    .join("\n  ")}

  <text x="24" y="34">Region map generated from renderListJson primitives</text>
  <text class="small" x="24" y="56">Colored regions are actual bond polygons; P1/P2/P3 are contour intersections; bond polygons pass through C between adjacent P points.</text>
</svg>
`;

await fs.mkdir(path.dirname(output), { recursive: true });
await fs.writeFile(output, svg);
console.log(output);
console.log(`incident bonds: ${incident.map((primitive) => primitive.bondId).join(", ")}`);
console.log(`unique incident polygon points: ${uniquePoints.size}`);
console.log(
  `center patch centroid: ${centerPatchCentroid.x.toFixed(6)}, ${centerPatchCentroid.y.toFixed(6)}`,
);
