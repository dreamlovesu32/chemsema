import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const retiredName = ["chem", "core"].join("");
const allowedFiles = new Set([
  "README.md",
  "README.zh-CN.md",
  "docs/migration-to-chemsema.md",
  "docs/migration-to-chemsema.zh-CN.md",
  "scripts/check-legacy-redirects.mjs",
]);

const trackedFiles = execFileSync("git", ["ls-files", "-z"], {
  encoding: "utf8",
}).split("\0").filter(Boolean);

const violations = [];
for (const file of trackedFiles) {
  if (allowedFiles.has(file)) {
    continue;
  }
  if (file.toLowerCase().includes(retiredName)) {
    violations.push(`${file} (path)`);
    continue;
  }
  const content = readFileSync(file).toString("latin1").toLowerCase();
  if (content.includes(retiredName)) {
    violations.push(`${file} (content)`);
  }
}

if (violations.length > 0) {
  console.error("Retired project name found outside the compatibility allowlist:");
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

console.log(`Retired-name audit passed for ${trackedFiles.length} tracked files.`);
