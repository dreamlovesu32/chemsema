import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex").toUpperCase();
}

function requireFile(path, label) {
  if (!existsSync(path) || !statSync(path).isFile()) {
    throw new Error(`${label} is missing: ${path}`);
  }
  return path;
}

const viewerWasm = requireFile(join(rootDir, "viewer", "engine", "chemsema_engine_bg.wasm"), "Web WASM");
const harmonyWasm = requireFile(
  join(rootDir, "apps", "chemsema-harmony", "entry", "src", "main", "resources", "rawfile", "chemsema", "engine", "chemsema_engine_bg.wasm"),
  "HarmonyOS WASM",
);
const desktop = requireFile(join(rootDir, "target", "release", "chemsema-desktop.exe"), "Desktop executable");
const windowsCli = requireFile(join(rootDir, "target", "release", "chemsema-cli.exe"), "Windows CLI");
const linuxCli = requireFile(join(rootDir, "target", "wsl-ubuntu", "release", "chemsema-cli"), "Ubuntu CLI");
const skillAssets = join(rootDir, "ChemSemaSkills", "skills", "chemsema-cli", "assets");
const manifest = JSON.parse(readFileSync(join(skillAssets, "runtime-manifest.json"), "utf8"));

const checks = {
  webHarmonyWasmEqual: sha256(viewerWasm) === sha256(harmonyWasm),
  skillRuntimes: {},
};
for (const [platform, entry] of Object.entries(manifest.platforms || {})) {
  const path = requireFile(join(skillAssets, entry.path), `Skill runtime ${platform}`);
  checks.skillRuntimes[platform] = {
    sizeMatches: statSync(path).size === entry.size,
    sha256Matches: sha256(path) === String(entry.sha256).toUpperCase(),
  };
}
checks.windowsSkillMatchesRelease = sha256(windowsCli) === sha256(join(skillAssets, manifest.platforms["win-x64"].path));
checks.linuxSkillMatchesWslBuild = sha256(linuxCli) === sha256(join(skillAssets, manifest.platforms["linux-x64"].path));

const ok = checks.webHarmonyWasmEqual
  && checks.windowsSkillMatchesRelease
  && checks.linuxSkillMatchesWslBuild
  && Object.values(checks.skillRuntimes).every((entry) => entry.sizeMatches && entry.sha256Matches);
const report = {
  ok,
  artifacts: {
    webWasm: { path: viewerWasm, sha256: sha256(viewerWasm) },
    harmonyWasm: { path: harmonyWasm, sha256: sha256(harmonyWasm) },
    desktop: { path: desktop, sha256: sha256(desktop) },
    windowsCli: { path: windowsCli, sha256: sha256(windowsCli) },
    ubuntuCli: { path: linuxCli, sha256: sha256(linuxCli) },
  },
  checks,
};
console.log(JSON.stringify(report, null, 2));
if (!ok) {
  process.exit(1);
}
