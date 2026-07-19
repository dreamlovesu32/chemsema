import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import net from "node:net";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const defaultFixturePath = "C:\\Users\\Dream\\OneDrive\\DFT\\Pd-Esterification\\钯催化-jjb.cdxml";
const fixturePath = process.env.CHEMSEMA_LARGE_CDXML_PATH || defaultFixturePath;
const sampleCount = Number(process.env.CHEMSEMA_LARGE_CDXML_SAMPLES || 3);
const maxOpenMs = Number(process.env.CHEMSEMA_LARGE_CDXML_MAX_OPEN_MS || 5000);
const maxFitMs = Number(process.env.CHEMSEMA_LARGE_CDXML_MAX_FIT_MS || 1000);
const maxZoomMs = Number(process.env.CHEMSEMA_LARGE_CDXML_MAX_ZOOM_MS || 600);
const minRenderPrimitives = Number(process.env.CHEMSEMA_LARGE_CDXML_MIN_PRIMITIVES || 100);
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const outputDir = join(rootDir, "tmp", "performance", "large-cdxml-speed");
const reportPath = join(outputDir, `large-cdxml-speed-${runId}.json`);
const screenshotPath = join(outputDir, `large-cdxml-speed-${runId}.png`);

function portIsOpen() {
  return new Promise((resolve) => {
    const socket = net.connect({ host, port }, () => {
      socket.end();
      resolve(true);
    });
    socket.on("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

function waitForPort(timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect({ host, port }, () => {
        socket.end();
        resolve(true);
      });
      socket.on("error", () => {
        socket.destroy();
        if (Date.now() >= deadline) {
          reject(new Error(`Timed out waiting for ${host}:${port}`));
        } else {
          setTimeout(attempt, 100);
        }
      });
    };
    attempt();
  });
}

async function ensureServer() {
  if (await portIsOpen()) {
    return null;
  }
  const child = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
    cwd: rootDir,
    stdio: "ignore",
    windowsHide: true,
  });
  await waitForPort();
  return child;
}

function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  if (!sorted.length) {
    return null;
  }
  const index = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[index];
}

function round(value) {
  return Number(value.toFixed(1));
}

function rel(path) {
  return relative(rootDir, path).replaceAll("\\", "/");
}

async function nextPaint(page) {
  await page.evaluate(() => new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }));
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => !!window.__chemsemaDebug?.state?.editorEngine && !!window.__chemsemaDebug?.document,
    null,
    { timeout: 30000 },
  );
}

async function installNativeFileBridge(page, fileName, filePath, cdxmlText) {
  await page.addInitScript(({ name, path, text }) => {
    const respond = (id, response) => {
      setTimeout(() => {
        window.__chemsemaHarmonyResolve?.(id, JSON.stringify(response));
      }, 0);
    };
    window.__chemsemaLargeCdxmlFixture = {
      name,
      path,
      byteLength: new TextEncoder().encode(text).byteLength,
      charLength: text.length,
    };
    window.chemsemaHarmony = {
      postMessage(message) {
        const request = JSON.parse(message);
        const { id, command, payload = {} } = request;
        if (command === "chooseOpenPath") {
          respond(id, { ok: true, value: path });
          return "true";
        }
        if (command === "readPath") {
          respond(id, {
            ok: true,
            value: {
              path: payload.path || path,
              fileName: name,
              format: "cdxml",
              text,
            },
          });
          return "true";
        }
        if (command === "setWindowTitle" || command === "traceEvent") {
          respond(id, { ok: true, value: true });
          return "true";
        }
        respond(id, { ok: false, error: `Unsupported large CDXML bridge command: ${command}` });
        return "true";
      },
    };
  }, { name: fileName, path: filePath, text: cdxmlText });
}

function capturePageErrors(page, errors, phaseRef) {
  page.on("console", (message) => {
    if (message.type() === "error") {
      const location = message.location();
      errors.push(`${phaseRef.name}: ${message.text()} @ ${location.url}:${location.lineNumber}:${location.columnNumber}`);
    }
  });
  page.on("pageerror", (error) => errors.push(`${phaseRef.name}: ${error.message}`));
}

async function collectSummary(page) {
  return page.evaluate(() => {
    const debug = window.__chemsemaDebug;
    const doc = debug?.document || null;
    const resources = Object.values(doc?.resources || {});
    const resourceNodes = resources.reduce((sum, resource) => sum + (resource?.data?.nodes?.length || 0), 0);
    const resourceBonds = resources.reduce((sum, resource) => sum + (resource?.data?.bonds?.length || 0), 0);
    const renderListLength = debug?.state?.coreRenderList?.length || 0;
    const documentContent = document.querySelector('[data-layer="document-content"]');
    return {
      currentFileName: debug?.state?.currentFileName || null,
      objects: doc?.objects?.length || 0,
      resources: resources.length,
      resourceNodes,
      resourceBonds,
      renderListLength,
      renderedBonds: document.querySelectorAll("[data-bond-id]").length,
      renderedNodes: document.querySelectorAll("[data-node-id]").length,
      documentElementCount: documentContent?.querySelectorAll("*").length || 0,
      documentHtmlLength: documentContent?.outerHTML.length || 0,
      renderStats: debug?.renderStats || null,
    };
  });
}

async function timedClick(page, selector) {
  const started = await page.evaluate(() => performance.now());
  await page.locator(selector).click();
  await nextPaint(page);
  const ended = await page.evaluate(() => performance.now());
  return round(ended - started);
}

async function runSample(browser, cdxmlText, fileName, index) {
  const phase = { name: `sample-${index}:startup` };
  const errors = [];
  const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } });
  page.setDefaultTimeout(30000);
  capturePageErrors(page, errors, phase);

  const startupStarted = Date.now();
  await installNativeFileBridge(page, fileName, fixturePath, cdxmlText);
  await page.goto(`${baseUrl}?largeCdxmlSpeed=${Date.now()}-${index}`, { waitUntil: "domcontentloaded" });
  await waitForReady(page);
  const startupMs = Date.now() - startupStarted;

  phase.name = `sample-${index}:open`;
  const openStarted = await page.evaluate(() => performance.now());
  await page.locator('.editor-topbar button[data-command="open"]').click();
  await page.waitForFunction(
    ({ name, minPrimitives }) => (
      window.__chemsemaDebug?.state?.currentFileName === name
      && (window.__chemsemaDebug?.state?.coreRenderList?.length || 0) >= minPrimitives
    ),
    { name: fileName, minPrimitives: minRenderPrimitives },
    { timeout: maxOpenMs + 5000 },
  );
  await nextPaint(page);
  const openMs = round(await page.evaluate((started) => performance.now() - started, openStarted));
  const summary = await collectSummary(page);

  phase.name = `sample-${index}:fit`;
  const fitMs = await timedClick(page, "#fit-button");
  phase.name = `sample-${index}:zoom`;
  const zoomInMs = await timedClick(page, 'button[data-command="zoom-in"]');
  const zoomOutMs = await timedClick(page, 'button[data-command="zoom-out"]');

  if (index === 1) {
    await page.screenshot({ path: screenshotPath, fullPage: false });
  }
  await page.close();

  return {
    index,
    startupMs: round(startupMs),
    openMs,
    fitMs,
    zoomInMs,
    zoomOutMs,
    summary,
    errors,
  };
}

async function main() {
  assert(existsSync(fixturePath), `Large CDXML fixture does not exist: ${fixturePath}`);
  mkdirSync(outputDir, { recursive: true });

  const cdxmlText = readFileSync(fixturePath, "utf8");
  const fileName = basename(fixturePath);
  const server = await ensureServer();
  let browser = null;
  try {
    browser = await chromium.launch({
      headless: true,
      executablePath: existsSync(edgePath) ? edgePath : undefined,
    });
    const samples = [];
    for (let index = 1; index <= sampleCount; index += 1) {
      const sample = await runSample(browser, cdxmlText, fileName, index);
      samples.push(sample);
      console.log(
        `[large-cdxml-speed] sample ${index}: open=${sample.openMs}ms fit=${sample.fitMs}ms zoom=${sample.zoomInMs}/${sample.zoomOutMs}ms primitives=${sample.summary.renderListLength}`,
      );
    }

    const openValues = samples.map((sample) => sample.openMs);
    const fitValues = samples.map((sample) => sample.fitMs);
    const zoomValues = samples.flatMap((sample) => [sample.zoomInMs, sample.zoomOutMs]);
    const allErrors = samples.flatMap((sample) => sample.errors);
    const report = {
      fixture: {
        path: fixturePath,
        fileName,
        bytes: Buffer.byteLength(cdxmlText),
        chars: cdxmlText.length,
      },
      thresholds: {
        maxOpenMs,
        maxFitMs,
        maxZoomMs,
        minRenderPrimitives,
      },
      aggregate: {
        openMs: {
          min: round(Math.min(...openValues)),
          median: round(percentile(openValues, 50)),
          p95: round(percentile(openValues, 95)),
          max: round(Math.max(...openValues)),
        },
        fitMs: {
          max: round(Math.max(...fitValues)),
        },
        zoomMs: {
          max: round(Math.max(...zoomValues)),
        },
      },
      samples,
      artifacts: {
        report: rel(reportPath),
        screenshot: existsSync(screenshotPath) ? rel(screenshotPath) : null,
      },
    };

    writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

    const firstSummary = samples[0]?.summary || {};
    assert.equal(allErrors.length, 0, `Large CDXML regression saw console/page errors:\n${allErrors.join("\n")}`);
    assert(firstSummary.renderListLength >= minRenderPrimitives, `Rendered too few primitives: ${JSON.stringify(firstSummary)}`);
    assert(report.aggregate.openMs.max <= maxOpenMs, `Large CDXML open is too slow: max ${report.aggregate.openMs.max}ms > ${maxOpenMs}ms`);
    assert(report.aggregate.fitMs.max <= maxFitMs, `Large CDXML fit is too slow: max ${report.aggregate.fitMs.max}ms > ${maxFitMs}ms`);
    assert(report.aggregate.zoomMs.max <= maxZoomMs, `Large CDXML zoom is too slow: max ${report.aggregate.zoomMs.max}ms > ${maxZoomMs}ms`);

    console.log(
      `[large-cdxml-speed] ok open median=${report.aggregate.openMs.median}ms max=${report.aggregate.openMs.max}ms primitives=${firstSummary.renderListLength}`,
    );
    console.log(`[large-cdxml-speed] report ${rel(reportPath)}`);
  } finally {
    await browser?.close();
    if (server) {
      server.kill();
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
