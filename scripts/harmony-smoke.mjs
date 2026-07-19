import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const hdc = "D:\\Huawei\\DevEco Studio\\sdk\\default\\openharmony\\toolchains\\hdc.exe";
const target = process.env.CHEMSEMA_HARMONY_TARGET || "127.0.0.1:5555";
const hap = path.join(repoRoot, "apps", "chemsema-harmony", "entry", "build", "default", "outputs", "default", "entry-default-signed.hap");
const fallbackHap = path.join(repoRoot, "apps", "chemsema-harmony", "entry", "build", "default", "outputs", "default", "entry-default-unsigned.hap");
const rawfileRoot = path.join(repoRoot, "apps", "chemsema-harmony", "entry", "src", "main", "resources", "rawfile", "chemsema");

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    shell: false,
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (!options.allowFailure && result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
  return result;
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function assertPath(filePath, message) {
  if (!existsSync(filePath)) {
    throw new Error(`${message}: ${path.relative(repoRoot, filePath)}`);
  }
}

function waitForEntryAbility() {
  for (let attempt = 1; attempt <= 15; attempt += 1) {
    const dump = run(hdc, ["-t", target, "shell", "aa", "dump", "-a"], {
      capture: true,
      allowFailure: true,
    });
    if (
      dump.status === 0
      && dump.stdout.includes("org.chemsema.harmony")
      && dump.stdout.includes("EntryAbility")
    ) {
      return;
    }
    sleep(1000);
  }
  throw new Error("Harmony smoke did not find org.chemsema.harmony EntryAbility after launch");
}

function captureWebLogs() {
  return run(hdc, ["-t", target, "shell", "hilog", "-x"], {
    capture: true,
    allowFailure: true,
  });
}

function waitForWebReady() {
  for (let attempt = 1; attempt <= 20; attempt += 1) {
    const logs = captureWebLogs();
    if (
      logs.status === 0
      && logs.stdout.includes("[ChemSemaWeb] page end: https://chemsema.local/index.html")
      && logs.stdout.includes("[ChemSemaWeb] first screen painted")
    ) {
      return;
    }
    sleep(1000);
  }
  throw new Error("Harmony smoke did not observe ChemSema ArkWeb first-screen paint");
}

function validateWebLogs() {
  const logs = captureWebLogs();
  if (logs.status !== 0) {
    console.warn("[harmony-smoke] hilog capture failed; skipped web log checks");
    return;
  }
  const text = logs.stdout;
  if (!text.includes("[ChemSemaWeb] page end: https://chemsema.local/index.html")) {
    throw new Error("Harmony smoke did not observe ChemSema ArkWeb page load completion");
  }
  const forbidden = [
    "CORS policy",
    "ERR_FAILED",
    "Incorrect response MIME type",
    "[ChemSemaWeb] load error",
    "[ChemSemaWeb] blank screen detected",
  ];
  const hit = forbidden.find((pattern) => text.includes(pattern));
  if (hit) {
    throw new Error(`Harmony smoke found ArkWeb failure in logs: ${hit}`);
  }
}

run(process.execPath, ["scripts/toolbar-regression.mjs"]);
run(process.execPath, ["scripts/build-harmony.mjs"]);

assertPath(path.join(rawfileRoot, "index.html"), "Harmony rawfile viewer is missing");
assertPath(path.join(rawfileRoot, "toolbar.js"), "Harmony rawfile toolbar is missing");
assertPath(path.join(rawfileRoot, "engine", "chemsema_engine.js"), "Harmony rawfile engine JS is missing");
assertPath(path.join(rawfileRoot, "engine", "chemsema_engine_bg.wasm"), "Harmony rawfile engine WASM is missing");

const installHap = existsSync(hap) ? hap : fallbackHap;
assertPath(installHap, "Harmony HAP is missing");

if (!existsSync(hdc)) {
  console.warn("[harmony-smoke] hdc not found; skipped device install");
  process.exit(0);
}

const targets = run(hdc, ["list", "targets"], { capture: true, allowFailure: true });
if (!targets.stdout.includes(target)) {
  console.warn(`[harmony-smoke] target ${target} is not online; skipped device install`);
  process.exit(0);
}

run(hdc, ["-t", target, "uninstall", "org.chemsema.harmony"], { allowFailure: true });
run(hdc, ["-t", target, "install", "-r", installHap]);
run(hdc, ["-t", target, "shell", "hilog", "-r"], { allowFailure: true });
run(hdc, ["-t", target, "shell", "aa", "start", "-b", "org.chemsema.harmony", "-a", "EntryAbility"]);
waitForEntryAbility();
waitForWebReady();
sleep(500);

mkdirSync(path.join(repoRoot, "tmp"), { recursive: true });
const remoteShot = "/data/local/tmp/chemsema-harmony-smoke.jpeg";
const localShot = path.join(repoRoot, "tmp", "chemsema-harmony-smoke.jpeg");
run(hdc, ["-t", target, "shell", "snapshot_display", "-f", remoteShot], { allowFailure: true });
run(hdc, ["-t", target, "file", "recv", remoteShot, localShot], { allowFailure: true });
validateWebLogs();

console.log(`[harmony-smoke] ok (${path.relative(repoRoot, installHap)})`);
