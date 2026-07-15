import { spawn } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const npmCli = process.env.npm_execpath;

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      shell: false,
      stdio: "inherit",
    });

    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (signal) {
        reject(new Error(`${command} exited with signal ${signal}`));
      } else if (code) {
        reject(new Error(`${command} exited with code ${code}`));
      } else {
        resolve();
      }
    });
  });
}

if (!npmCli) {
  throw new Error("npm_execpath is not available; run this script through npm run harmony:build.");
}

await run(process.execPath, [npmCli, "run", "build:engine-wasm"]);
await run(process.execPath, [npmCli, "run", "harmony:sync-viewer"]);
const daemonSecurityCache = path.join(
  homedir(),
  ".hvigor",
  "daemon",
  "cache",
  "daemon-sec.json",
);
if (existsSync(daemonSecurityCache)) {
  await run(process.execPath, ["scripts/harmony-hvigor.mjs", "--stop-daemon"]);
} else {
  console.log("Skipping hvigor --stop-daemon because no daemon cache exists.");
}

const harmonyProject = path.join(repoRoot, "apps", "chemcore-harmony");
const buildProfile = path.join(harmonyProject, "build-profile.json5");
const exampleBuildProfile = path.join(harmonyProject, "build-profile.example.json5");
let originalBuildProfile = null;
if (existsSync(buildProfile)) {
  const content = readFileSync(buildProfile, "utf8");
  try {
    const profile = JSON.parse(content);
    const signingConfigs = profile?.app?.signingConfigs || [];
    const missingMaterial = signingConfigs.some((config) => {
      const material = config?.material || {};
      return [material.certpath, material.profile, material.storeFile]
        .filter(Boolean)
        .some((file) => !existsSync(file));
    });
    if (missingMaterial) {
      originalBuildProfile = content;
      writeFileSync(buildProfile, readFileSync(exampleBuildProfile, "utf8"));
      console.warn("Harmony signing material is unavailable; building an unsigned HAP.");
    }
  } catch {
    // Let hvigor report malformed local profiles with its native diagnostics.
  }
}

try {
  await run(process.execPath, [
    "scripts/harmony-hvigor.mjs",
    "assembleHap",
    "--mode",
    "module",
    "-p",
    "product=default",
  ]);
} finally {
  if (originalBuildProfile !== null) {
    writeFileSync(buildProfile, originalBuildProfile);
  }
}
