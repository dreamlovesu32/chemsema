import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestPath = join(repoRoot, "benchmarks", "public-cdxml", "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const args = process.argv.slice(2);

function option(name, fallback) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
}

const corpusRoot = resolve(
  option(
    "--root",
    process.env.CHEMSEMA_PUBLIC_CDXML_DIR || join(repoRoot, "tmp", "public-cdxml-corpus"),
  ),
);
const outputRoot = resolve(option("--out-dir", join(repoRoot, "tmp", "public-cdxml-roundtrip")));
const reportPath = join(outputRoot, "report.json");
const summaryOutput = option("--summary-out", null);
const workRoot = join(outputRoot, "work");
const limit = Number.parseInt(option("--limit", "0"), 10);
const strictCounts = args.includes("--strict-counts");

function discoverCli() {
  const explicit = option("--cli", process.env.CHEMSEMA_CLI);
  const suffix = process.platform === "win32" ? ".exe" : "";
  const candidates = [
    explicit,
    join(repoRoot, "target", "debug", `chemsema-cli${suffix}`),
    join(repoRoot, "target", "release", `chemsema-cli${suffix}`),
  ].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(
      "chemsema-cli not found. Build it with `cargo build -p chemsema-cli` or pass --cli <path>.",
    );
  }
  return resolve(found);
}

const cliPath = discoverCli();
mkdirSync(workRoot, { recursive: true });

function runCli(commandArgs) {
  const result = spawnSync(cliPath, commandArgs, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
  return {
    ok: result.status === 0,
    status: result.status,
    stdout: result.stdout?.trim() || "",
    stderr: result.stderr?.trim() || "",
    error: result.error?.message || null,
  };
}

function collectFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory).sort()) {
      if (entry === ".git") continue;
      const path = join(directory, entry);
      const stat = statSync(path);
      if (stat.isDirectory()) {
        visit(path);
      } else if ([".cdxml", ".cdx"].includes(extname(entry).toLowerCase())) {
        files.push(path);
      }
    }
  };
  visit(root);
  return files;
}

function normalizedRelative(root, path) {
  return relative(root, path).split(sep).join("/");
}

function readSummary(path) {
  return JSON.parse(readFileSync(path, "utf8")).summary;
}

const countKeys = ["molecules", "nodes", "bonds", "objects", "resources", "styles"];

function compareCounts(before, after) {
  const delta = Object.fromEntries(countKeys.map((key) => [key, (after[key] || 0) - (before[key] || 0)]));
  const objectTypes = [...new Set([
    ...Object.keys(before.objectTypes || {}),
    ...Object.keys(after.objectTypes || {}),
  ])].sort();
  const objectTypeDelta = Object.fromEntries(
    objectTypes.map((key) => [key, (after.objectTypes?.[key] || 0) - (before.objectTypes?.[key] || 0)]),
  );
  return {
    exact: Object.values(delta).every((value) => value === 0)
      && Object.values(objectTypeDelta).every((value) => value === 0),
    delta,
    objectTypeDelta,
  };
}

const versionResult = runCli(["version"]);
const cliVersion = versionResult.ok ? JSON.parse(versionResult.stdout).version : "unknown";
const cases = [];
let caseIndex = 0;

for (const source of manifest.sources) {
  const sourceRoot = join(corpusRoot, source.id);
  if (!existsSync(sourceRoot)) {
    throw new Error(`Missing source ${source.id}. Run npm run benchmark:cdxml-public:fetch first.`);
  }
  const specialCases = new Map((source.specialCases || []).map((item) => [item.path, item]));
  for (const inputPath of collectFiles(sourceRoot)) {
    if (limit > 0 && cases.length >= limit) break;
    caseIndex += 1;
    const relativePath = normalizedRelative(sourceRoot, inputPath);
    const special = specialCases.get(relativePath);
    const classification = special?.class || "valid";
    const format = extname(inputPath).toLowerCase().slice(1);
    const key = String(caseIndex).padStart(4, "0");
    const beforePath = join(workRoot, `${key}-before.json`);
    const afterPath = join(workRoot, `${key}-after.json`);
    const roundTripPath = join(workRoot, `${key}-roundtrip.${format}`);
    for (const path of [beforePath, afterPath, roundTripPath]) rmSync(path, { force: true });

    const record = {
      caseId: key,
      source: source.id,
      path: relativePath,
      format,
      classification,
      status: "pending",
    };

    if (classification === "transport-encoded") {
      record.status = "skipped";
      record.reason = special.reason;
      cases.push(record);
      continue;
    }

    const beforeRun = runCli(["inspect", inputPath, "--include", "summary", "--out", beforePath]);
    if (classification === "invalid") {
      record.status = beforeRun.ok ? "unexpected-accept" : "expected-reject";
      record.reason = special.reason;
      if (!beforeRun.ok) record.error = beforeRun.stderr || beforeRun.stdout || beforeRun.error;
      cases.push(record);
      continue;
    }

    if (!beforeRun.ok) {
      record.status = "import-failed";
      record.error = beforeRun.stderr || beforeRun.stdout || beforeRun.error;
      cases.push(record);
      continue;
    }

    const before = readSummary(beforePath).counts;
    const convertRun = runCli(["convert", inputPath, roundTripPath]);
    if (!convertRun.ok) {
      record.status = "export-failed";
      record.before = before;
      record.error = convertRun.stderr || convertRun.stdout || convertRun.error;
      cases.push(record);
      continue;
    }

    const afterRun = runCli(["inspect", roundTripPath, "--include", "summary", "--out", afterPath]);
    if (!afterRun.ok) {
      record.status = "reimport-failed";
      record.before = before;
      record.error = afterRun.stderr || afterRun.stdout || afterRun.error;
      cases.push(record);
      continue;
    }

    const after = readSummary(afterPath).counts;
    const comparison = compareCounts(before, after);
    record.status = comparison.exact ? "exact-counts" : "count-drift";
    record.before = before;
    record.after = after;
    record.comparison = comparison;
    cases.push(record);
  }
  if (limit > 0 && cases.length >= limit) break;
}

const statuses = {};
const bySource = {};
for (const item of cases) {
  statuses[item.status] = (statuses[item.status] || 0) + 1;
  const source = (bySource[item.source] ||= { total: 0, statuses: {} });
  source.total += 1;
  source.statuses[item.status] = (source.statuses[item.status] || 0) + 1;
}

const unexpectedStatuses = new Set([
  "unexpected-accept",
  "import-failed",
  "export-failed",
  "reimport-failed",
]);
const unexpectedFailures = cases.filter((item) => unexpectedStatuses.has(item.status));
const countDrift = cases.filter((item) => item.status === "count-drift");
const report = {
  schema: "chemsema.public-cdxml-roundtrip-report.v1",
  generatedAt: new Date().toISOString(),
  cliVersion,
  manifest: normalizedRelative(repoRoot, manifestPath),
  corpusRoot,
  summary: {
    total: cases.length,
    statuses,
    bySource,
    unexpectedFailures: unexpectedFailures.length,
    countDrift: countDrift.length,
  },
  cases,
};
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
if (summaryOutput) {
  const summaryPath = resolve(summaryOutput);
  const summaryReport = {
    schema: "chemsema.public-cdxml-roundtrip-summary.v1",
    generatedAt: report.generatedAt,
    cliVersion,
    sources: manifest.sources.map((source) => ({
      id: source.id,
      revision: source.revision,
      license: source.license.spdx,
      expectedFiles: source.expectedFiles,
    })),
    summary: report.summary,
  };
  mkdirSync(dirname(summaryPath), { recursive: true });
  writeFileSync(summaryPath, `${JSON.stringify(summaryReport, null, 2)}\n`, "utf8");
  console.log(`Summary: ${summaryPath}`);
}

console.log(JSON.stringify(report.summary, null, 2));
console.log(`Report: ${reportPath}`);
if (unexpectedFailures.length > 0 || (strictCounts && countDrift.length > 0)) process.exitCode = 1;
