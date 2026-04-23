import { chromium } from "playwright";
import path from "node:path";

const url = process.argv[2] || "http://127.0.0.1:8765/viewer/";
const output = process.argv[3] || path.resolve("tmp/viewer-screenshot.png");
const sample = process.argv[4] || "";

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1440, height: 1100 },
  deviceScaleFactor: 1.5,
});

await page.goto(url, { waitUntil: "networkidle" });
if (sample) {
  await page.selectOption("#sample-select", sample);
  await page.waitForTimeout(250);
}
await page.screenshot({ path: output, fullPage: true });
await browser.close();

console.log(output);
