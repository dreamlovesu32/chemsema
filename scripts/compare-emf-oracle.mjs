import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { inspectEmf, inspectionMarkdown } from "./emf-inspect.mjs";
import { renderEmfPreviews } from "./render-emf-preview.mjs";

function parseArgs(argv) {
  const args = {
    outDir: "tmp/emf-oracle-compare",
    inputs: [],
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--out") args.outDir = argv[++i];
    else if (arg === "--skip-chemdraw") args.skipChemDraw = true;
    else if (arg === "--help" || arg === "-h") args.help = true;
    else args.inputs.push(arg);
  }
  return args;
}

function safeStem(inputPath) {
  return path.basename(inputPath, path.extname(inputPath)).replace(/[<>:"/\\|?*\x00-\x1f]/g, "_");
}

function run(command, args, options = {}) {
  console.log(`[RUN] ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`${command} failed with exit code ${result.status}`);
  }
}

async function defaultInputs() {
  const candidates = ["tmp/color.cdxml", "tmp/arrows-acs.cdxml", "tmp/kuohao.cdxml"];
  const existing = [];
  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      existing.push(candidate);
    } catch {
      // local fixture may not exist
    }
  }
  return existing;
}

function countDelta(leftCounts, rightCounts) {
  const keys = new Set([...Object.keys(leftCounts), ...Object.keys(rightCounts)]);
  return [...keys]
    .map((key) => ({ name: key, chemdraw: leftCounts[key] ?? 0, chemsema: rightCounts[key] ?? 0 }))
    .filter((row) => row.chemdraw !== row.chemsema)
    .sort((a, b) => Math.abs(b.chemdraw - b.chemsema) - Math.abs(a.chemdraw - a.chemsema));
}

function compactCounts(counts) {
  const interesting = [
    "EMR_POLYLINE",
    "EMR_POLYLINE16",
    "EMR_POLYGON",
    "EMR_POLYGON16",
    "EMR_LINETO",
    "EMR_CREATEPEN",
    "EMR_EXTCREATEPEN",
    "EMR_CREATEBRUSHINDIRECT",
    "EMR_EXTCREATEFONTINDIRECTW",
    "EMR_EXTTEXTOUTA",
    "EMR_EXTTEXTOUTW",
    "EMR_STRETCHDIBITS",
    "EMR_BEGINPATH",
    "EMR_STROKEPATH",
    "EMR_FILLPATH",
    "EMR_STROKEANDFILLPATH",
  ];
  return interesting
    .filter((name) => counts[name])
    .map((name) => `${name}=${counts[name]}`)
    .join(", ");
}

export async function compareEmfOracle(options = {}) {
  const outDir = path.resolve(options.outDir ?? "tmp/emf-oracle-compare");
  const inputs = (options.inputs?.length ? options.inputs : await defaultInputs()).map((input) =>
    path.resolve(input)
  );
  if (!inputs.length) {
    throw new Error("No CDXML inputs were provided or found.");
  }
  await fs.mkdir(outDir, { recursive: true });

  let oracleJobs = [];
  if (!options.skipChemDraw) {
    oracleJobs = await generateChemDrawOracle({
      outDir,
      formats: ["svg", "emf"],
      inputs,
    });
  } else {
    oracleJobs = inputs.map((input) => {
      const stem = safeStem(input);
      return {
        input,
        outputs: {
          svg: path.join(outDir, `${stem}.chemdraw.svg`),
          emf: path.join(outDir, `${stem}.chemdraw.emf`),
        },
      };
    });
  }

  const reports = [];
  for (const job of oracleJobs) {
    const stem = safeStem(job.input);
    const payload = path.join(outDir, `${stem}.chemsema.payload.json`);
    const chemsemaEmf = path.join(outDir, `${stem}.chemsema.emf`);
    const chemsemaSvg = path.join(outDir, `${stem}.chemsema.svg`);
    const chemdrawPng = path.join(outDir, `${stem}.chemdraw.emf.png`);
    const chemsemaPng = path.join(outDir, `${stem}.chemsema.emf.png`);

    run("cargo", [
      "run",
      "-p",
      "chemsema-engine",
      "--example",
      "cdxml_to_svg",
      "--",
      job.input,
      chemsemaSvg,
    ]);
    run("cargo", [
      "run",
      "-p",
      "chemsema-engine",
      "--example",
      "cdxml_to_clipboard_payload",
      "--",
      job.input,
      payload,
    ]);
    run("cargo", [
      "run",
      "-p",
      "chemsema-office",
      "--",
      "--write-emf-payload",
      payload,
      chemsemaEmf,
    ]);

    const chemdrawInspection = await inspectEmf(job.outputs.emf, { includeRecords: false });
    const chemsemaInspection = await inspectEmf(chemsemaEmf, { includeRecords: false });
    const chemdrawJson = path.join(outDir, `${stem}.chemdraw.emf.inspect.json`);
    const chemsemaJson = path.join(outDir, `${stem}.chemsema.emf.inspect.json`);
    const chemdrawMd = path.join(outDir, `${stem}.chemdraw.emf.inspect.md`);
    const chemsemaMd = path.join(outDir, `${stem}.chemsema.emf.inspect.md`);
    await fs.writeFile(chemdrawJson, JSON.stringify(chemdrawInspection, null, 2), "utf8");
    await fs.writeFile(chemsemaJson, JSON.stringify(chemsemaInspection, null, 2), "utf8");
    await fs.writeFile(chemdrawMd, inspectionMarkdown(chemdrawInspection), "utf8");
    await fs.writeFile(chemsemaMd, inspectionMarkdown(chemsemaInspection), "utf8");
    await renderEmfPreviews([
      { input: job.outputs.emf, output: chemdrawPng },
      { input: chemsemaEmf, output: chemsemaPng },
    ]);

    reports.push({
      stem,
      input: job.input,
      chemdraw: chemdrawInspection,
      chemsema: chemsemaInspection,
      delta: countDelta(chemdrawInspection.typeCounts, chemsemaInspection.typeCounts).slice(0, 24),
      outputs: {
        chemdrawSvg: job.outputs.svg,
        chemdrawEmf: job.outputs.emf,
        chemsemaSvg,
        chemsemaEmf,
        chemdrawPng,
        chemsemaPng,
        chemdrawJson,
        chemsemaJson,
      },
    });
  }

  const summary = [];
  summary.push("# ChemDraw / ChemSema EMF Oracle Comparison");
  summary.push("");
  summary.push(`Generated: ${new Date().toISOString()}`);
  summary.push("");
  for (const report of reports) {
    summary.push(`## ${report.stem}`);
    summary.push("");
    summary.push(`- Input: \`${report.input}\``);
    summary.push(`- ChemDraw EMF: \`${report.outputs.chemdrawEmf}\``);
    summary.push(`- ChemSema EMF: \`${report.outputs.chemsemaEmf}\``);
    summary.push(`- ChemDraw PNG preview: \`${report.outputs.chemdrawPng}\``);
    summary.push(`- ChemSema PNG preview: \`${report.outputs.chemsemaPng}\``);
    summary.push(`- ChemDraw bytes/records: ${report.chemdraw.bytes} / ${report.chemdraw.recordCount}`);
    summary.push(`- ChemSema bytes/records: ${report.chemsema.bytes} / ${report.chemsema.recordCount}`);
    summary.push(`- ChemDraw key counts: ${compactCounts(report.chemdraw.typeCounts) || "(none)"}`);
    summary.push(`- ChemSema key counts: ${compactCounts(report.chemsema.typeCounts) || "(none)"}`);
    summary.push("");
    if (report.delta.length) {
      summary.push("| Record | ChemDraw | ChemSema |");
      summary.push("| --- | ---: | ---: |");
      for (const row of report.delta) {
        summary.push(`| ${row.name} | ${row.chemdraw} | ${row.chemsema} |`);
      }
      summary.push("");
    }
  }
  const summaryPath = path.join(outDir, "summary.md");
  await fs.writeFile(summaryPath, summary.join("\n"), "utf8");
  console.log(`[WROTE] ${summaryPath}`);
  return { outDir, reports, summaryPath };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/compare-emf-oracle.mjs [--out dir] [--skip-chemdraw] <file.cdxml>...");
    return;
  }
  await compareEmfOracle(args);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
