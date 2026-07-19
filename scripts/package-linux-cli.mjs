import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const { version } = JSON.parse(readFileSync(join(rootDir, "package.json"), "utf8"));

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function windowsPathToWsl(value) {
  const match = /^([A-Za-z]):[\\/](.*)$/.exec(value);
  if (!match) throw new Error(`Expected an absolute Windows path, got: ${value}`);
  return `/mnt/${match[1].toLowerCase()}/${match[2].replaceAll("\\", "/")}`;
}

if (process.platform === "win32") {
  run(process.execPath, ["scripts/wsl-cli.mjs", "build"]);
  const repo = windowsPathToWsl(rootDir);
  run("wsl.exe", [
    "--",
    "bash",
    `${repo}/scripts/package-linux-cli.sh`,
    "--version",
    version,
    "--cli",
    `${repo}/target/wsl-ubuntu/release/chemsema-cli`,
    "--out-dir",
    `${repo}/dist/chemsema-cli`,
  ]);
} else if (process.platform === "linux") {
  run("cargo", ["build", "--locked", "--release", "-p", "chemsema-cli"]);
  run("bash", [
    "scripts/package-linux-cli.sh",
    "--version",
    version,
    "--cli",
    "target/release/chemsema-cli",
    "--out-dir",
    "dist/chemsema-cli",
  ]);
} else {
  console.error(`Linux CLI packaging is not supported on ${process.platform}.`);
  process.exit(1);
}
