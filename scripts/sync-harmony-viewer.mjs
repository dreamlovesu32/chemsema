import { cp, mkdir, readdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = path.join(repoRoot, "viewer");
const targetDir = path.join(
  repoRoot,
  "apps",
  "chemsema-harmony",
  "entry",
  "src",
  "main",
  "resources",
  "rawfile",
  "chemsema",
);

async function countFiles(dir) {
  let count = 0;
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      count += await countFiles(fullPath);
    } else if (entry.isFile()) {
      count += 1;
    }
  }
  return count;
}

await rm(targetDir, { force: true, recursive: true });
await mkdir(path.dirname(targetDir), { recursive: true });
await cp(sourceDir, targetDir, {
  filter: (source) => !source.endsWith(".d.ts"),
  recursive: true,
});

const copiedFiles = await countFiles(targetDir);
console.log(`Synced ${copiedFiles} viewer files to ${path.relative(repoRoot, targetDir)}`);
