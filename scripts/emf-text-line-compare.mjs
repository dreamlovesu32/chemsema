import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    ours: null,
    reference: null,
    region: null,
    output: null,
    includeSingles: true,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--region") {
      const [left, top, right, bottom] = argv[++index]
        .split(",")
        .map((value) => Number.parseFloat(value.trim()));
      args.region = { left, top, right, bottom };
    } else if (arg === "--output") {
      args.output = argv[++index];
    } else if (arg === "--exclude-singles") {
      args.includeSingles = false;
    } else if (!args.ours) {
      args.ours = arg;
    } else if (!args.reference) {
      args.reference = arg;
    }
  }
  return args;
}

function extractLines(recordsJson, options) {
  const region = options.region;
  const rows = new Map();
  for (const record of recordsJson.records ?? []) {
    if (record?.name !== "EMR_EXTTEXTOUTW") continue;
    const payload = record.text ?? {};
    const text = payload.text ?? "";
    if (!options.includeSingles && text.length <= 1 && text !== " ") continue;
    const reference = payload.reference ?? {};
    const x = reference.x;
    const y = reference.y;
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    if (
      region &&
      (x < region.left || x > region.right || y < region.top || y > region.bottom)
    ) {
      continue;
    }
    if (!rows.has(y)) rows.set(y, []);
    rows.get(y).push({
      text,
      x,
      left: record.bounds?.left ?? null,
      right: record.bounds?.right ?? null,
      index: record.index,
    });
  }

  return [...rows.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([y, items]) => {
      const sorted = items.sort((a, b) => a.x - b.x);
      const left = Math.min(...sorted.map((item) => item.left ?? item.x));
      const right = Math.max(...sorted.map((item) => item.right ?? item.x));
      return {
        y,
        left,
        right,
        width: right - left,
        center: (left + right) / 2,
        text: sorted.map((item) => item.text).join(""),
        items: sorted,
      };
    });
}

function normalizeText(text) {
  return text.replaceAll(" ", "<sp>");
}

function formatCell(value) {
  return value === null || value === undefined ? "" : String(value);
}

function buildMarkdown(args, ours, reference) {
  const lines = [];
  lines.push("# EMR_EXTTEXTOUTW Line Compare");
  lines.push("");
  lines.push(`- ours: \`${args.ours}\``);
  lines.push(`- reference: \`${args.reference}\``);
  if (args.region) {
    lines.push(
      `- region: \`${args.region.left},${args.region.top},${args.region.right},${args.region.bottom}\``
    );
  }
  lines.push("");
  lines.push(
    "| row | ours y | ref y | dy | ours left | ref left | dleft | ours right | ref right | dright | ours width | ref width | dwidth | dcenter | text |"
  );
  lines.push("|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|");

  const count = Math.max(ours.length, reference.length);
  for (let index = 0; index < count; index += 1) {
    const left = ours[index];
    const right = reference[index];
    const dy = left && right ? left.y - right.y : null;
    const dleft = left && right ? left.left - right.left : null;
    const dright = left && right ? left.right - right.right : null;
    const dwidth = left && right ? left.width - right.width : null;
    const dcenter = left && right ? left.center - right.center : null;
    lines.push(
      `| ${index} | ${formatCell(left?.y)} | ${formatCell(right?.y)} | ${formatCell(
        dy
      )} | ${formatCell(left?.left)} | ${formatCell(right?.left)} | ${formatCell(
        dleft
      )} | ${formatCell(left?.right)} | ${formatCell(right?.right)} | ${formatCell(
        dright
      )} | ${formatCell(left?.width)} | ${formatCell(right?.width)} | ${formatCell(
        dwidth
      )} | ${formatCell(dcenter)} | \`${normalizeText(left?.text ?? right?.text ?? "")}\` |`
    );
  }

  return `${lines.join("\n")}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.ours || !args.reference) {
    console.error(
      "Usage: node scripts/emf-text-line-compare.mjs <ours.records.json> <reference.records.json> [--region left,top,right,bottom] [--output file.md] [--exclude-singles]"
    );
    process.exit(1);
  }

  const [oursJsonText, referenceJsonText] = await Promise.all([
    fs.readFile(path.resolve(args.ours), "utf8"),
    fs.readFile(path.resolve(args.reference), "utf8"),
  ]);

  const ours = extractLines(JSON.parse(oursJsonText), args);
  const reference = extractLines(JSON.parse(referenceJsonText), args);
  const markdown = buildMarkdown(args, ours, reference);

  if (args.output) {
    await fs.writeFile(path.resolve(args.output), markdown, "utf8");
  } else {
    process.stdout.write(markdown);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
