import fs from "node:fs/promises";
import path from "node:path";
import init, { WasmEngine } from "../viewer/engine/chemsema_engine.js";
import { launchBrowser } from "./playwright-browser.mjs";

const CASES = [
  {
    name: "kuohao-symbols",
    cdxml: "tmp/kuohao.cdxml",
    svg: "tmp/kuohao.svg",
    groups: [
      { kind: "circle-plus", paths: [15, 16, 17] },
      { kind: "plus", paths: [18, 19] },
      { kind: "radical-cation", paths: [20, 21, 22] },
      { kind: "lone-pair", paths: [23, 24] },
      { kind: "circle-minus", paths: [25, 26] },
      { kind: "minus", paths: [27] },
      { kind: "radical-anion", paths: [28, 29] },
      { kind: "electron", paths: [30] },
    ],
  },
  {
    name: "kuohao-acs-symbols",
    cdxml: "tmp/kuohao-acs.cdxml",
    svg: "tmp/kuohao-acs.svg",
    groups: [
      { kind: "circle-plus", paths: [15, 16, 17] },
      { kind: "plus", paths: [18, 19] },
      { kind: "radical-cation", paths: [20, 21, 22] },
      { kind: "lone-pair", paths: [23, 24] },
      { kind: "circle-minus", paths: [25, 26] },
      { kind: "minus", paths: [27] },
      { kind: "radical-anion", paths: [28, 29] },
      { kind: "electron", paths: [30] },
    ],
  },
  {
    name: "duibi-symbols",
    cdxml: "tmp/duibi.cdxml",
    svg: "tmp/duibi.svg",
    groups: [
      { kind: "circle-plus", paths: [2, 3, 4] },
      { kind: "plus", paths: [5, 6] },
    ],
  },
];

const OUT_DIR = "tmp/symbol-pixel-compare";
const GENERATED_TO_CHEMDRAW_SCALE = 2;

function attr(tag, name) {
  return tag.match(new RegExp(`${name}="([^"]*)"`))?.[1] ?? "";
}

function numbers(value) {
  return [...value.matchAll(/-?\d+(?:\.\d+)?/g)].map((match) => Number(match[0]));
}

function parseMatrix(value) {
  const nums = numbers(value);
  if (nums.length < 6) {
    return [1, 0, 0, 1, 0, 0];
  }
  return nums.slice(0, 6);
}

function transformPoint(matrix, point) {
  const [a, b, c, d, e, f] = matrix;
  return {
    x: a * point.x + c * point.y + e,
    y: b * point.x + d * point.y + f,
  };
}

function transformBBox(matrix, bbox) {
  const corners = [
    transformPoint(matrix, { x: bbox[0], y: bbox[1] }),
    transformPoint(matrix, { x: bbox[2], y: bbox[1] }),
    transformPoint(matrix, { x: bbox[2], y: bbox[3] }),
    transformPoint(matrix, { x: bbox[0], y: bbox[3] }),
  ];
  return [
    Math.min(...corners.map((point) => point.x)),
    Math.min(...corners.map((point) => point.y)),
    Math.max(...corners.map((point) => point.x)),
    Math.max(...corners.map((point) => point.y)),
  ];
}

function pathLocalBBox(d) {
  const boxes = [];
  const withoutArcs = d.replace(
    /M\s*(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)\s*A\s*(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)[^A]*?(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)\s*A\s*\3,\4[^A]*?\1,\2\s*Z?/g,
    (_match, leftValue, centerYValue, rxValue, ryValue, rightValue) => {
      const left = Number(leftValue);
      const centerY = Number(centerYValue);
      const rx = Number(rxValue);
      const ry = Number(ryValue);
      const right = Number(rightValue);
      boxes.push([
        Math.min(left, right),
        centerY - ry,
        Math.max(left, right),
        centerY + ry,
      ]);
      return " ";
    },
  );

  const nums = numbers(withoutArcs);
  if (nums.length >= 2) {
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const pair of chunks(nums, 2)) {
      minX = Math.min(minX, pair[0]);
      minY = Math.min(minY, pair[1]);
      maxX = Math.max(maxX, pair[0]);
      maxY = Math.max(maxY, pair[1]);
    }
    boxes.push([minX, minY, maxX, maxY]);
  }

  if (boxes.length > 0) {
    return unionBBox(boxes);
  }
  return [Infinity, Infinity, -Infinity, -Infinity];
}

function pathBBox(tag) {
  const bbox = pathLocalBBox(attr(tag, "d"));
  const transform = attr(tag, "transform");
  return transform ? transformBBox(parseMatrix(transform), bbox) : bbox;
}

function primitiveBBox(primitive) {
  if (primitive.kind === "ellipse") {
    const stroke = Number(primitive.strokeWidth ?? primitive.stroke_width ?? 0);
    const grow = stroke * 0.5;
    return [
      primitive.center.x - primitive.rx - grow,
      primitive.center.y - primitive.ry - grow,
      primitive.center.x + primitive.rx + grow,
      primitive.center.y + primitive.ry + grow,
    ];
  }
  if (primitive.kind === "filled-path" || primitive.kind === "path") {
    return pathLocalBBox(primitive.d);
  }
  return [Infinity, Infinity, -Infinity, -Infinity];
}

function unionBBox(boxes) {
  return [
    Math.min(...boxes.map((box) => box[0])),
    Math.min(...boxes.map((box) => box[1])),
    Math.max(...boxes.map((box) => box[2])),
    Math.max(...boxes.map((box) => box[3])),
  ];
}

function bboxCenter(box) {
  return {
    x: (box[0] + box[2]) * 0.5,
    y: (box[1] + box[3]) * 0.5,
  };
}

function chunks(values, size) {
  const out = [];
  for (let index = 0; index + size - 1 < values.length; index += size) {
    out.push(values.slice(index, index + size));
  }
  return out;
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

function primitiveToSvg(primitive) {
  if (primitive.kind === "ellipse") {
    return `<ellipse ${attrsToString({
      cx: primitive.center.x,
      cy: primitive.center.y,
      rx: primitive.rx,
      ry: primitive.ry,
      fill: primitive.fill || "none",
      stroke: primitive.stroke || "none",
      "stroke-width": primitive.strokeWidth ?? primitive.stroke_width ?? 0,
    })}/>`;
  }
  if (primitive.kind === "filled-path") {
    return `<path ${attrsToString({
      d: primitive.d,
      fill: primitive.fill || "#000000",
      stroke: "none",
      "fill-rule": primitive.fillRule || primitive.fill_rule || undefined,
    })}/>`;
  }
  if (primitive.kind === "path") {
    return `<path ${attrsToString({
      d: primitive.d,
      fill: "none",
      stroke: primitive.stroke || "#000000",
      "stroke-width": primitive.strokeWidth ?? primitive.stroke_width ?? 1,
      "stroke-linecap": primitive.lineCap || primitive.line_cap || undefined,
      "stroke-linejoin": primitive.lineJoin || primitive.line_join || undefined,
    })}/>`;
  }
  return "";
}

function svgDocument(width, height, viewBox, body) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${viewBox.join(" ")}">
<rect x="${viewBox[0]}" y="${viewBox[1]}" width="${viewBox[2]}" height="${viewBox[3]}" fill="#ffffff"/>
${body}
</svg>
`;
}

async function compareSvgPixels(page, referenceSvg, generatedSvg, width, height, diffPath) {
  const result = await page.evaluate(
    async ({ referenceSvg, generatedSvg, width, height }) => {
      async function svgPixels(svg) {
        const image = new Image();
        image.decoding = "sync";
        image.src = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
        await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.drawImage(image, 0, 0, width, height);
        return context.getImageData(0, 0, width, height);
      }

      const reference = await svgPixels(referenceSvg);
      const generated = await svgPixels(generatedSvg);
      const diff = new ImageData(width, height);
      let different = 0;
      let maxChannelDelta = 0;
      for (let index = 0; index < reference.data.length; index += 4) {
        const dr = Math.abs(reference.data[index] - generated.data[index]);
        const dg = Math.abs(reference.data[index + 1] - generated.data[index + 1]);
        const db = Math.abs(reference.data[index + 2] - generated.data[index + 2]);
        const da = Math.abs(reference.data[index + 3] - generated.data[index + 3]);
        const delta = Math.max(dr, dg, db, da);
        maxChannelDelta = Math.max(maxChannelDelta, delta);
        if (delta > 0) {
          different += 1;
          diff.data[index] = 255;
          diff.data[index + 1] = 0;
          diff.data[index + 2] = 0;
          diff.data[index + 3] = 255;
        } else {
          diff.data[index] = 255;
          diff.data[index + 1] = 255;
          diff.data[index + 2] = 255;
          diff.data[index + 3] = 255;
        }
      }

      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      canvas.getContext("2d").putImageData(diff, 0, 0);
      return {
        different,
        total: width * height,
        maxChannelDelta,
        diffPngBase64: canvas.toDataURL("image/png").split(",")[1],
      };
    },
    { referenceSvg, generatedSvg, width, height },
  );
  await fs.writeFile(diffPath, Buffer.from(result.diffPngBase64, "base64"));
  delete result.diffPngBase64;
  return result;
}

async function runCase(page, testCase) {
  const sourceSvg = await fs.readFile(testCase.svg, "utf8");
  const pathTags = [...sourceSvg.matchAll(/<path\b[^>]*>/g)].map((match) => match[0]);
  const referenceGroups = testCase.groups.map((group) => ({
    ...group,
    tags: group.paths.map((index) => pathTags[index]),
  }));
  const referenceBoxes = referenceGroups.map((group) =>
    unionBBox(group.tags.map((tag) => pathBBox(tag))),
  );
  const referenceBox = unionBBox(referenceBoxes);
  const pad = 4;
  const viewBox = [
    referenceBox[0] - pad,
    referenceBox[1] - pad,
    referenceBox[2] - referenceBox[0] + pad * 2,
    referenceBox[3] - referenceBox[1] + pad * 2,
  ];
  const width = Math.ceil(viewBox[2]);
  const height = Math.ceil(viewBox[3]);

  const referenceBody = referenceGroups.flatMap((group) => group.tags).join("\n");
  const referenceSvg = svgDocument(width, height, viewBox, referenceBody);

  const engine = new WasmEngine();
  engine.loadDocumentCdxml(await fs.readFile(testCase.cdxml, "utf8"));
  const document = JSON.parse(engine.documentJson());
  const primitives = JSON.parse(engine.renderListJson());

  const generatedBody = [];
  for (let index = 0; index < testCase.groups.length; index += 1) {
    const group = testCase.groups[index];
    const object = document.objects.find(
      (candidate) => candidate.type === "symbol" && candidate.payload?.kind === group.kind,
    );
    if (!object) {
      throw new Error(`missing generated symbol ${group.kind}`);
    }
    const objectPrimitives = primitives.filter((primitive) => primitive.objectId === object.id);
    const generatedBox = unionBBox(objectPrimitives.map((primitive) => primitiveBBox(primitive)));
    const referenceCenter = bboxCenter(referenceBoxes[index]);
    const generatedCenter = bboxCenter(generatedBox);
    const dx = referenceCenter.x - generatedCenter.x * GENERATED_TO_CHEMDRAW_SCALE;
    const dy = referenceCenter.y - generatedCenter.y * GENERATED_TO_CHEMDRAW_SCALE;
    generatedBody.push(
      `<g transform="translate(${dx} ${dy}) scale(${GENERATED_TO_CHEMDRAW_SCALE})">${objectPrimitives
        .map(primitiveToSvg)
        .filter(Boolean)
        .join("\n")}</g>`,
    );
  }
  const generatedSvg = svgDocument(width, height, viewBox, generatedBody.join("\n"));

  await fs.mkdir(OUT_DIR, { recursive: true });
  const referencePath = path.join(OUT_DIR, `${testCase.name}-reference.svg`);
  const generatedPath = path.join(OUT_DIR, `${testCase.name}-backend.svg`);
  const diffPath = path.join(OUT_DIR, `${testCase.name}-diff.png`);
  await fs.writeFile(referencePath, referenceSvg);
  await fs.writeFile(generatedPath, generatedSvg);
  const result = await compareSvgPixels(page, referenceSvg, generatedSvg, width, height, diffPath);
  const report = {
    name: testCase.name,
    referencePath,
    generatedPath,
    diffPath,
    width,
    height,
    ...result,
  };
  await fs.writeFile(
    path.join(OUT_DIR, `${testCase.name}-report.json`),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  return report;
}

await init(await fs.readFile(new URL("../viewer/engine/chemsema_engine_bg.wasm", import.meta.url)));

const browser = await launchBrowser({ headless: true });
const page = await browser.newPage();
const reports = [];
try {
  for (const testCase of CASES) {
    reports.push(await runCase(page, testCase));
  }
} finally {
  await browser.close();
}

await fs.writeFile(
  path.join(OUT_DIR, "summary.json"),
  `${JSON.stringify(reports, null, 2)}\n`,
);
console.log(JSON.stringify(reports, null, 2));
