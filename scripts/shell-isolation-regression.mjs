import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";

function waitForPort(timeoutMs = 5000) {
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

function capturePageErrors(page, errors) {
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(error.message));
}

async function installTauriMock(context) {
  await context.addInitScript(() => {
    window.__TAURI__ = {
      core: {
        invoke: async (command) => {
          if (command === "desktop_recent_files" || command === "desktop_take_startup_open_paths") {
            return [];
          }
          if (command === "desktop_window_take_detached_document" || command === "desktop_file_choose_open") {
            return null;
          }
          if (command === "desktop_window_is_maximized") {
            return false;
          }
          return true;
        },
      },
      event: {
        listen: async () => () => {},
      },
      window: {
        getCurrentWindow: () => ({
          destroy: async () => true,
        }),
      },
    };
  });
}

async function installHarmonyMock(context) {
  await context.addInitScript(() => {
    window.chemcoreHarmony = {
      postMessage(message) {
        const { id } = JSON.parse(message);
        setTimeout(() => {
          window.__chemcoreHarmonyResolve?.(id, JSON.stringify({ ok: true, value: null }));
        }, 0);
        return "true";
      },
    };
  });
}

async function openViewer(context, errors, kind) {
  const page = await context.newPage({ viewport: { width: 1400, height: 900 } });
  page.setDefaultTimeout(12000);
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?shell=${kind}-${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => !!window.__chemcoreDebug?.state?.editorEngine && !!window.__chemcoreDebug?.document,
    null,
    { timeout: 30000 },
  );
  return page;
}

async function shellState(page) {
  return page.evaluate(() => {
    const readRect = (selector) => {
      const rect = document.querySelector(selector)?.getBoundingClientRect();
      return rect ? { width: rect.width, height: rect.height, top: rect.top, bottom: rect.bottom } : null;
    };
    return {
      classes: [...document.body.classList],
      titlebarDisplay: getComputedStyle(document.querySelector(".desktop-titlebar")).display,
      tabbarDisplay: getComputedStyle(document.querySelector(".document-tabbar")).display,
      controlsDisplay: getComputedStyle(document.querySelector(".window-controls")).display,
      titlebarRect: readRect(".desktop-titlebar"),
      tabRect: readRect(".document-tab"),
      newButtonRect: readRect("#document-tab-new"),
      tabTitles: [...document.querySelectorAll(".document-tab-title")].map((node) => node.textContent.trim()),
      activeTabs: [...document.querySelectorAll(".document-tab")].map((node) => node.classList.contains("is-active")),
    };
  });
}

function assertShellClass(state, expectedClass, forbiddenClasses, label) {
  assert(state.classes.includes(expectedClass), `${label} did not set ${expectedClass}: ${JSON.stringify(state)}`);
  for (const forbiddenClass of forbiddenClasses) {
    assert(!state.classes.includes(forbiddenClass), `${label} leaked ${forbiddenClass}: ${JSON.stringify(state)}`);
  }
}

async function verifyHostedShell(page, label, expectedClass, expectedControlsDisplay) {
  const before = await shellState(page);
  assertShellClass(before, expectedClass, ["browser-shell", expectedClass === "desktop-shell" ? "native-frame-shell" : "desktop-shell"], label);
  assert.equal(before.titlebarDisplay, "flex", `${label} titlebar should be visible: ${JSON.stringify(before)}`);
  assert.equal(before.tabbarDisplay, "flex", `${label} tabbar should be visible: ${JSON.stringify(before)}`);
  assert.equal(before.controlsDisplay, expectedControlsDisplay, `${label} window controls visibility is wrong: ${JSON.stringify(before)}`);
  assert.equal(before.tabTitles.length, 1, `${label} should start with one document tab: ${JSON.stringify(before)}`);

  await page.locator("#document-tab-new").click();
  await page.waitForFunction(() => document.querySelectorAll(".document-tab").length === 2);
  const after = await shellState(page);
  assert.equal(after.tabTitles.length, 2, `${label} new-document button did not create an in-app tab: ${JSON.stringify(after)}`);
  assert.equal(after.activeTabs[1], true, `${label} did not activate the new document tab: ${JSON.stringify(after)}`);
  return { before, after };
}

async function verifyTauriShell(page) {
  const { before } = await verifyHostedShell(page, "Tauri", "desktop-shell", "grid");
  assert.equal(Math.round(before.titlebarRect.height), 42, `Tauri titlebar height should stay desktop-specific: ${JSON.stringify(before)}`);
  assert(before.tabRect.height > 34, `Tauri active tab should keep the raised desktop shape: ${JSON.stringify(before)}`);
}

async function verifyHarmonyShell(page) {
  const { before } = await verifyHostedShell(page, "Harmony", "native-frame-shell", "none");
  assert.equal(Math.round(before.titlebarRect.height), 46, `Harmony titlebar height should stay native-frame-specific: ${JSON.stringify(before)}`);
  assert(before.tabRect.height <= 33, `Harmony tab should keep the compact native-frame shape: ${JSON.stringify(before)}`);
}

async function verifyBrowserShell(page) {
  const state = await shellState(page);
  assertShellClass(state, "browser-shell", ["desktop-shell", "native-frame-shell"], "Browser");
  assert.equal(state.titlebarDisplay, "none", `Browser titlebar should be hidden: ${JSON.stringify(state)}`);
  assert.equal(state.tabbarDisplay, "none", `Browser in-app tabbar should be hidden: ${JSON.stringify(state)}`);
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const errors = [];

  const browserContext = await browser.newContext();
  await verifyBrowserShell(await openViewer(browserContext, errors, "browser"));
  await browserContext.close();

  const tauriContext = await browser.newContext();
  await installTauriMock(tauriContext);
  await verifyTauriShell(await openViewer(tauriContext, errors, "tauri"));
  await tauriContext.close();

  const harmonyContext = await browser.newContext();
  await installHarmonyMock(harmonyContext);
  await verifyHarmonyShell(await openViewer(harmonyContext, errors, "harmony"));
  await harmonyContext.close();

  assert.equal(errors.length, 0, `Shell isolation regression saw console/page errors:\n${errors.join("\n")}`);
  console.log("[shell-isolation-regression] ok (browser, Tauri desktop shell, Harmony native-frame shell)");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
