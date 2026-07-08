import { spawn } from "node:child_process";
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
await run(process.execPath, ["scripts/harmony-hvigor.mjs", "--stop-daemon"]);
await run(process.execPath, [
  "scripts/harmony-hvigor.mjs",
  "assembleHap",
  "--mode",
  "module",
  "-p",
  "product=default",
]);
