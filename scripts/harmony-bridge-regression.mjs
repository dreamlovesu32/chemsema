import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
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

async function installHarmonyBridgeMock(context) {
  await context.addInitScript(() => {
    window.confirm = () => true;
    window.__chemcoreHarmonyMock = {
      clipboard: null,
      commands: [],
      files: {},
      openQueue: [],
      saveQueue: [],
      titles: [],
      writes: [],
    };
    window.__chemcoreHarmonySetFile = (path, text) => {
      window.__chemcoreHarmonyMock.files[String(path)] = String(text);
    };
    window.__chemcoreHarmonyQueueOpen = (paths) => {
      window.__chemcoreHarmonyMock.openQueue = Array.from(paths || []);
    };
    window.__chemcoreHarmonyQueueSave = (paths) => {
      window.__chemcoreHarmonyMock.saveQueue = Array.from(paths || []);
    };

    const fileNameFromPath = (path) => {
      try {
        const decoded = decodeURIComponent(String(path || ""));
        return decoded.split(/[\\/]/).filter(Boolean).pop()?.split("?")[0]?.split("#")[0] || "Untitled";
      } catch {
        return String(path || "").split(/[\\/]/).filter(Boolean).pop() || "Untitled";
      }
    };
    const formatFromPath = (path) => {
      const name = fileNameFromPath(path).toLowerCase();
      return name.includes(".") ? name.split(".").pop() : "";
    };
    const respond = (id, response) => {
      setTimeout(() => {
        window.__chemcoreHarmonyResolve?.(id, JSON.stringify(response));
      }, 0);
    };

    window.chemcoreHarmony = {
      postMessage(message) {
        const request = JSON.parse(message);
        const { id, command, payload = {} } = request;
        const mock = window.__chemcoreHarmonyMock;
        mock.commands.push({ command, payload });
        try {
          if (command === "chooseOpenPath") {
            const path = mock.openQueue.shift() || null;
            respond(id, { ok: true, value: path });
            return "true";
          }
          if (command === "chooseSavePath" || command === "chooseExportSavePath") {
            const path = mock.saveQueue.shift() || payload.suggestedName || "chemcore-document.ccjz";
            respond(id, { ok: true, value: path });
            return "true";
          }
          if (command === "readPath") {
            const path = String(payload.path || "");
            respond(id, {
              ok: true,
              value: {
                path,
                fileName: fileNameFromPath(path),
                format: formatFromPath(path),
                text: mock.files[path] || "",
              },
            });
            return "true";
          }
          if (command === "writePath" || command === "writeTransientPath") {
            const path = String(payload.path || "");
            const content = String(payload.content || "");
            mock.files[path] = content;
            mock.writes.push({ command, path, content, format: payload.format || formatFromPath(path) });
            respond(id, { ok: true, value: { path, fileName: fileNameFromPath(path) } });
            return "true";
          }
          if (command === "writeBase64") {
            const path = String(payload.path || "");
            mock.writes.push({ command, path, contentBase64: payload.contentBase64 || "" });
            respond(id, { ok: true, value: { path, fileName: fileNameFromPath(path) } });
            return "true";
          }
          if (command === "writeClipboard") {
            mock.clipboard = payload.payload || null;
            respond(id, { ok: true, value: true });
            return "true";
          }
          if (command === "readClipboard") {
            respond(id, { ok: true, value: mock.clipboard });
            return "true";
          }
          if (command === "setWindowTitle") {
            mock.titles.push(payload.title || "");
            respond(id, { ok: true, value: true });
            return "true";
          }
          if (command === "traceEvent") {
            respond(id, { ok: true, value: true });
            return "true";
          }
          respond(id, { ok: false, error: `Unsupported mock Harmony command: ${command}` });
          return "true";
        } catch (error) {
          respond(id, { ok: false, error: error?.message || String(error) });
          return "true";
        }
      },
    };
  });
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => !!window.__chemcoreDebug?.state?.editorEngine && !!window.__chemcoreDebug?.document,
    null,
    { timeout: 30000 },
  );
}

async function openViewer(context, errors) {
  const page = await context.newPage({ viewport: { width: 1400, height: 1000 } });
  page.setDefaultTimeout(12000);
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?v=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await waitForReady(page);
  return page;
}

async function drawBondWithMouse(page) {
  await page.locator('button[data-tool="bond"]').click();
  const box = await page.locator("#viewer-container").boundingBox();
  assert(box, "Viewer container is not visible.");
  const start = { x: box.x + box.width / 2 - 90, y: box.y + box.height / 2 };
  const end = { x: start.x + 130, y: start.y };
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
}

async function harmonyMock(page) {
  return page.evaluate(() => window.__chemcoreHarmonyMock);
}

async function verifyHarmonyShell(page) {
  const shell = await page.evaluate(() => ({
    desktop: document.body.classList.contains("desktop-shell"),
    nativeFrame: document.body.classList.contains("native-frame-shell"),
    browser: document.body.classList.contains("browser-shell"),
    titlebarDisplay: getComputedStyle(document.querySelector(".desktop-titlebar")).display,
    tabbarDisplay: getComputedStyle(document.querySelector(".document-tabbar")).display,
    brandDisplay: getComputedStyle(document.querySelector(".titlebar-brand")).display,
    controlsDisplay: getComputedStyle(document.querySelector(".window-controls")).display,
    tabCount: document.querySelectorAll(".document-tab").length,
  }));
  assert.equal(shell.desktop, false, `Harmony should not enable the custom desktop titlebar: ${JSON.stringify(shell)}`);
  assert.equal(shell.nativeFrame, true, `Harmony should use the native-framed shell: ${JSON.stringify(shell)}`);
  assert.equal(shell.browser, false, `Harmony should not use the browser shell: ${JSON.stringify(shell)}`);
  assert.equal(shell.titlebarDisplay, "flex", `Harmony document tab strip is not visible: ${JSON.stringify(shell)}`);
  assert.equal(shell.tabbarDisplay, "flex", `Harmony document tabs are not visible: ${JSON.stringify(shell)}`);
  assert.equal(shell.brandDisplay, "flex", `Harmony should show the custom top-bar brand: ${JSON.stringify(shell)}`);
  assert.equal(shell.controlsDisplay, "none", `Harmony should not show custom window controls: ${JSON.stringify(shell)}`);
  assert.equal(shell.tabCount, 1, `Initial document tab count is wrong: ${JSON.stringify(shell)}`);
}

async function verifyNativeFrameNewDocument(page) {
  await page.keyboard.press("Control+N");
  await page.waitForFunction(() => document.querySelectorAll(".document-tab").length === 2);
  const shell = await page.evaluate(() => ({
    tabbarDisplay: getComputedStyle(document.querySelector(".document-tabbar")).display,
    tabs: [...document.querySelectorAll(".document-tab")].map((tab) => ({
      title: tab.textContent.trim(),
      active: tab.classList.contains("is-active"),
    })),
    fileName: window.__chemcoreDebug?.state?.currentFileName || null,
  }));
  assert.equal(shell.tabbarDisplay, "flex", `Harmony new document hid the tabbar: ${JSON.stringify(shell)}`);
  assert.equal(shell.tabs.length, 2, `Harmony new document did not create a visible tab: ${JSON.stringify(shell)}`);
  assert.equal(shell.tabs[1].active, true, `Harmony new document did not activate the new tab: ${JSON.stringify(shell)}`);
  assert.equal(shell.fileName, null, `Harmony new document did not reset the active document: ${JSON.stringify(shell)}`);
}

async function verifySaveOpenClipboard(page) {
  await drawBondWithMouse(page);
  await page.evaluate(() => window.__chemcoreHarmonyQueueSave(["harmony-save.ccjs"]));
  await page.keyboard.press("Control+Shift+S");
  await page.waitForFunction(() => window.__chemcoreHarmonyMock.writes.length >= 1);
  const saved = await harmonyMock(page);
  assert.equal(saved.writes[0].path, "harmony-save.ccjs", `Save did not use Harmony path: ${JSON.stringify(saved.writes[0])}`);
  assert(saved.writes[0].content.includes('"objects"'), "Harmony save did not write ChemCore JSON.");

  await page.keyboard.press("Control+A");
  await page.keyboard.press("Control+C");
  await page.waitForFunction(() => !!window.__chemcoreHarmonyMock.clipboard?.chemcoreFragmentJson);
  await page.keyboard.press("Control+X");
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length === 0);
  await page.keyboard.press("Control+V");
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);

  await page.keyboard.press("Control+S");
  await page.waitForFunction(() => window.__chemcoreHarmonyMock.writes.length >= 2);

  await page.evaluate((text) => {
    window.__chemcoreHarmonySetFile("harmony-open.ccjs", text);
    window.__chemcoreHarmonyQueueOpen(["harmony-open.ccjs"]);
  }, saved.writes[0].content);
  await page.keyboard.press("Control+O");
  await page.waitForFunction(() => window.__chemcoreDebug?.state?.currentFileName === "harmony-open.ccjs");
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);

  const opened = await page.evaluate(() => ({
    fileName: window.__chemcoreDebug?.state?.currentFileName || null,
    tabbarDisplay: getComputedStyle(document.querySelector(".document-tabbar")).display,
    tabTitles: [...document.querySelectorAll(".document-tab-title")].map((tab) => tab.textContent.trim()),
    title: document.title,
    windowTitles: window.__chemcoreHarmonyMock.titles,
  }));
  assert.equal(opened.fileName, "harmony-open.ccjs", `Open did not preserve Harmony file name: ${JSON.stringify(opened)}`);
  assert.equal(opened.tabbarDisplay, "flex", `Harmony open hid the tabbar: ${JSON.stringify(opened)}`);
  assert(opened.tabTitles.some((title) => title.includes("harmony-open.ccjs")), `Tab title did not update after Harmony open: ${JSON.stringify(opened)}`);
  assert(opened.windowTitles.length > 0, `Window title was not sent through Harmony bridge: ${JSON.stringify(opened)}`);
}

let server = null;
let browser = null;
try {
  mkdirSync(join(rootDir, "tmp"), { recursive: true });
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const context = await browser.newContext();
  await installHarmonyBridgeMock(context);
  const errors = [];
  const page = await openViewer(context, errors);
  await verifyHarmonyShell(page);
  await verifyNativeFrameNewDocument(page);
  await verifySaveOpenClipboard(page);
  await page.close();
  assert.equal(errors.length, 0, `Harmony bridge regression saw console/page errors:\n${errors.join("\n")}`);
  console.log("[harmony-bridge-regression] ok (native frame tabs, bridge file open/save, clipboard, title)");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
