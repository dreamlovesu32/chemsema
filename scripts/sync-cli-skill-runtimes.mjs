import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const skillAssets = join(rootDir, "ChemSemaSkills", "skills", "chemsema-cli", "assets");
const manifestPath = join(skillAssets, "runtime-manifest.json");
const runtimes = {
  "win-x64": {
    source: join(rootDir, "target", "release", "chemsema-cli.exe"),
    path: "bin/win-x64/chemsema-cli.exe",
  },
  "linux-x64": {
    source: join(rootDir, "target", "wsl-ubuntu", "release", "chemsema-cli"),
    path: "bin/linux-x64/chemsema-cli",
  },
};

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex").toUpperCase();
}

const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
manifest.platforms = {};
for (const [platform, runtime] of Object.entries(runtimes)) {
  const destination = join(skillAssets, runtime.path);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(runtime.source, destination);
  const stat = statSync(destination);
  manifest.platforms[platform] = {
    path: runtime.path,
    size: stat.size,
    sha256: sha256(destination),
    signed: false,
    signature: "unsigned",
  };
}
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, manifest: manifestPath, platforms: manifest.platforms }, null, 2));
