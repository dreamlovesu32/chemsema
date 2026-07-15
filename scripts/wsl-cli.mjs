import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const mode = process.argv[2] || "test";
const allowedModes = new Set(["build", "smoke", "test"]);

if (process.platform !== "win32") {
  console.error("wsl-cli.mjs is a Windows host helper; run cargo directly on native Linux.");
  process.exit(1);
}
if (!allowedModes.has(mode)) {
  console.error("Usage: node scripts/wsl-cli.mjs <build|smoke|test>");
  process.exit(1);
}

function runWsl(args, options = {}) {
  const result = spawnSync("wsl.exe", ["--", ...args], {
    cwd: rootDir,
    encoding: options.capture ? "utf8" : undefined,
    stdio: options.capture ? "pipe" : "inherit",
    shell: false,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    if (options.capture) {
      process.stdout.write(result.stdout || "");
      process.stderr.write(result.stderr || "");
    }
    process.exit(result.status ?? 1);
  }
  return result;
}

function windowsPathToWsl(value) {
  const match = /^([A-Za-z]):[\\/](.*)$/.exec(value);
  if (!match) {
    throw new Error(`Expected an absolute Windows path, got: ${value}`);
  }
  return `/mnt/${match[1].toLowerCase()}/${match[2].replaceAll("\\", "/")}`;
}

const translated = windowsPathToWsl(rootDir);

function shellQuote(value) {
  return `'${String(value).replaceAll("'", `'"'"'`)}'`;
}

const repo = shellQuote(translated);
const target = shellQuote(`${translated}/target/wsl-ubuntu`);
const cli = shellQuote(`${translated}/target/wsl-ubuntu/release/chemcore-cli`);
const common = [
  "set -euo pipefail",
  `cd ${repo}`,
  `export CARGO_TARGET_DIR=${target}`,
  "export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-$(nproc)}",
];
const build = ["cargo build --locked --release -p chemcore-cli"];
const smoke = [
  `test -x ${cli}`,
  `${cli} version --pretty`,
  `${cli} capabilities >/dev/null`,
  `${cli} label-query --text CF3 --connection-angle 0 >/dev/null`,
];

let commands;
if (mode === "build") {
  commands = [...common, ...build];
} else if (mode === "smoke") {
  commands = [...common, ...smoke];
} else {
  commands = [
    ...common,
    "cargo test --locked -p chemcore-engine -p chemcore-cli",
    ...build,
    ...smoke,
  ];
}

runWsl(["bash", "-lc", commands.join("\n")]);
console.log(`[wsl-cli] ${mode} ok (${translated})`);
