import path from "node:path";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { launchBrowser } from "./playwright-browser.mjs";

const url = process.argv[2] || "http://127.0.0.1:8765/viewer/";
const output = process.argv[3] || path.resolve("tmp/viewer-screenshot.png");
const sample = process.argv[4] || "";

function convertToDocumentJson(inputPath) {
  const extension = path.extname(inputPath).toLowerCase();
  if (extension === ".json" || extension === ".ccjs") {
    return JSON.parse(readFileSync(inputPath, "utf8"));
  }
  const tempDir = mkdtempSync(path.join(tmpdir(), "chemsema-viewer-screenshot-"));
  const outputPath = path.join(tempDir, `${path.basename(inputPath, extension) || "document"}.ccjs`);
  const result = spawnSync(
    "cargo",
    ["run", "-p", "chemsema-cli", "--", "convert", inputPath, outputPath, "--format", "ccjs"],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  if (result.status !== 0) {
    throw new Error(`Failed to convert ${inputPath} for screenshot:\n${result.stdout || ""}${result.stderr || ""}`);
  }
  return JSON.parse(readFileSync(outputPath, "utf8"));
}

const browser = await launchBrowser({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1440, height: 1100 },
  deviceScaleFactor: 1.5,
});

await page.goto(url, { waitUntil: "networkidle" });
if (sample) {
  const samplePath = path.resolve(sample);
  if (existsSync(samplePath)) {
    const documentData = convertToDocumentJson(samplePath);
    await page.evaluate(async (data) => {
      await window.__chemsemaDebug?.loadDocumentForTest?.(data);
    }, documentData);
    await page.waitForFunction(() => window.__chemsemaDebug?.document?.objects?.length > 0);
  } else {
    await page.selectOption("#sample-select", sample);
  }
  await page.waitForTimeout(350);
}
await page.screenshot({ path: output, fullPage: true });
await browser.close();

console.log(output);
