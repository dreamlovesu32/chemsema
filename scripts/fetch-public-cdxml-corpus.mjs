import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestPath = join(repoRoot, "benchmarks", "public-cdxml", "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const destinationRoot = resolve(
  process.env.CHEMSEMA_PUBLIC_CDXML_DIR || join(repoRoot, "tmp", "public-cdxml-corpus"),
);

function runGit(args, cwd) {
  execFileSync("git", args, { cwd, stdio: "inherit" });
}

function collectChemDrawFiles(root) {
  const counts = { cdxml: 0, cdx: 0 };
  const visit = (directory) => {
    for (const entry of readdirSync(directory)) {
      if (entry === ".git") continue;
      const path = join(directory, entry);
      const stat = statSync(path);
      if (stat.isDirectory()) {
        visit(path);
      } else {
        const extension = extname(entry).toLowerCase().slice(1);
        if (extension === "cdxml" || extension === "cdx") counts[extension] += 1;
      }
    }
  };
  visit(root);
  return counts;
}

mkdirSync(destinationRoot, { recursive: true });

for (const source of manifest.sources) {
  const destination = join(destinationRoot, source.id);
  const gitDirectory = join(destination, ".git");

  if (!existsSync(destination)) {
    runGit(["clone", "--filter=blob:none", "--no-checkout", source.repository, destination], repoRoot);
  } else if (!existsSync(gitDirectory)) {
    throw new Error(`Refusing to reuse non-Git directory: ${destination}`);
  }

  runGit(["fetch", "origin", source.revision, "--depth", "1"], destination);
  runGit(["sparse-checkout", "init", "--cone"], destination);
  runGit(["sparse-checkout", "set", ...source.sparsePaths], destination);
  runGit(["checkout", "--detach", source.revision], destination);

  const counts = collectChemDrawFiles(destination);
  for (const format of ["cdxml", "cdx"]) {
    if (counts[format] !== source.expectedFiles[format]) {
      throw new Error(
        `${source.id}: expected ${source.expectedFiles[format]} ${format.toUpperCase()} files, found ${counts[format]}`,
      );
    }
  }
  console.log(
    `${source.id}: ${counts.cdxml} CDXML + ${counts.cdx} CDX @ ${source.revision.slice(0, 12)} (${source.license.spdx})`,
  );
}

console.log(`Public CDXML/CDX corpus ready at ${destinationRoot}`);
