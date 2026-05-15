import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    ours: null,
    reference: null,
    region: null,
    output: null,
    includeSingles: false,
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
    } else if (arg === "--include-singles") {
      args.includeSingles = true;
    } else if (!args.ours) {
      args.ours = arg;
    } else if (!args.reference) {
      args.reference = arg;
    }
  }
  return args;
}

function normalizeText(text) {
  return text.replaceAll(" ", "<sp>");
}

function extractTextRecords(recordsJson, options) {
  const region = options.region;
  const records = [];
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
    records.push({
      index: record.index,
      text,
      x,
      y,
      right: record.bounds?.right ?? null,
      bottom: record.bounds?.bottom ?? null,
    });
  }
  return records;
}

function alignSequences(ours, reference) {
  const rows = ours.length + 1;
  const cols = reference.length + 1;
  const dp = Array.from({ length: rows }, () => Array(cols).fill(0));
  for (let i = 1; i < rows; i += 1) {
    for (let j = 1; j < cols; j += 1) {
      if (ours[i - 1].text === reference[j - 1].text) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  const aligned = [];
  let i = ours.length;
  let j = reference.length;
  while (i > 0 || j > 0) {
    if (
      i > 0 &&
      j > 0 &&
      ours[i - 1].text === reference[j - 1].text &&
      dp[i][j] === dp[i - 1][j - 1] + 1
    ) {
      aligned.push({ ours: ours[i - 1], reference: reference[j - 1], kind: "match" });
      i -= 1;
      j -= 1;
      continue;
    }
    if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      aligned.push({ ours: null, reference: reference[j - 1], kind: "reference-only" });
      j -= 1;
      continue;
    }
    aligned.push({ ours: ours[i - 1], reference: null, kind: "ours-only" });
    i -= 1;
  }

  aligned.reverse();
  return aligned;
}

function formatCell(value) {
  return value === null || value === undefined ? "" : String(value);
}

function buildMarkdown(args, aligned) {
  const lines = [];
  lines.push("# EMR_EXTTEXTOUTW Token Compare");
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
    "| kind | token | ours idx | ours x | ours y | ref idx | ref x | ref y | dx | dy |"
  );
  lines.push("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");

  let matched = 0;
  let dxSum = 0;
  let dySum = 0;

  for (const row of aligned) {
    const token = normalizeText(row.ours?.text ?? row.reference?.text ?? "");
    const dx =
      row.ours && row.reference ? row.ours.x - row.reference.x : null;
    const dy =
      row.ours && row.reference ? row.ours.y - row.reference.y : null;
    if (Number.isFinite(dx) && Number.isFinite(dy)) {
      matched += 1;
      dxSum += dx;
      dySum += dy;
    }
    lines.push(
      `| ${row.kind} | \`${token}\` | ${formatCell(row.ours?.index)} | ${formatCell(
        row.ours?.x
      )} | ${formatCell(row.ours?.y)} | ${formatCell(row.reference?.index)} | ${formatCell(
        row.reference?.x
      )} | ${formatCell(row.reference?.y)} | ${formatCell(dx)} | ${formatCell(dy)} |`
    );
  }

  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- matched: ${matched}`);
  if (matched > 0) {
    lines.push(`- avg dx: ${(dxSum / matched).toFixed(3)}`);
    lines.push(`- avg dy: ${(dySum / matched).toFixed(3)}`);
  }

  return `${lines.join("\n")}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.ours || !args.reference) {
    console.error(
      "Usage: node scripts/emf-text-compare.mjs <ours.records.json> <reference.records.json> [--region left,top,right,bottom] [--output file.md] [--include-singles]"
    );
    process.exit(1);
  }

  const [oursJsonText, referenceJsonText] = await Promise.all([
    fs.readFile(path.resolve(args.ours), "utf8"),
    fs.readFile(path.resolve(args.reference), "utf8"),
  ]);

  const oursRecords = extractTextRecords(JSON.parse(oursJsonText), args);
  const referenceRecords = extractTextRecords(JSON.parse(referenceJsonText), args);
  const aligned = alignSequences(oursRecords, referenceRecords);
  const markdown = buildMarkdown(args, aligned);

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
