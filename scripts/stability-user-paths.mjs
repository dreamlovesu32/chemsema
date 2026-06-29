import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { makeSyntheticLargeDocument } from "./generate-stability-fixtures.mjs";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const syntheticNodeCount = Number(process.env.CHEMCORE_STABILITY_SYNTHETIC_BROWSER_NODES || 6500);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
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
  page.on("pageerror", (error) => errors.push(error.stack || error.message));
}

async function openViewer(browser) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?stability=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => !!window.__chemcoreDebug?.state?.editorEngine, null, { timeout: 20000 });
  return { page, errors };
}

async function activateTool(page, tool) {
  await page.locator(`button[data-tool="${tool}"]`).click();
  await page.waitForFunction((expectedTool) => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state?.activeTool === expectedTool
      && document.querySelector(`button[data-tool="${CSS.escape(expectedTool)}"]`)?.classList.contains("is-active");
  }, tool, { timeout: 2000 });
}

async function viewerBox(page) {
  const box = await page.locator("#viewer-container").boundingBox();
  assert(box, "Viewer container was not visible.");
  return box;
}

async function dragOnCanvas(page, tool, start, end, steps = 8) {
  await activateTool(page, tool);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps });
  await page.mouse.up();
  await page.waitForTimeout(160);
}

async function clickCanvas(page, tool, point) {
  await activateTool(page, tool);
  await page.mouse.click(point.x, point.y);
  await page.waitForTimeout(140);
}

async function documentSnapshot(page) {
  return page.evaluate(() => {
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const doc = parse(debug?.state?.editorEngine?.documentJson?.()) || debug?.document || null;
    const objects = [];
    const visit = (object, parentId = null) => {
      if (!object) {
        return;
      }
      const type = object.type || object.objectType || object.object_type || "";
      objects.push({
        id: object.id,
        type,
        parentId,
        text: object.payload?.text || object.payload?.extra?.text || "",
      });
      for (const child of object.children || []) {
        visit(child, object.id);
      }
    };
    for (const object of doc?.objects || []) {
      visit(object);
    }
    const typeCounts = {};
    for (const object of objects) {
      typeCounts[object.type] = (typeCounts[object.type] || 0) + 1;
    }
    let nodeCount = 0;
    let bondCount = 0;
    for (const resource of Object.values(doc?.resources || {})) {
      nodeCount += resource?.data?.nodes?.length || 0;
      bondCount += resource?.data?.bonds?.length || 0;
    }
    return {
      objectIds: objects.map((object) => object.id).filter(Boolean),
      objects,
      typeCounts,
      objectCount: objects.length,
      nodeCount,
      bondCount,
      activeTool: debug?.editorState?.activeTool || "",
      elementPlacementActive: !!debug?.editorState?.elementPlacementActive,
      engineTool: debug?.engineState?.tool?.activeTool || debug?.engineState?.tool?.active_tool || "",
      activeTextEditor: !!debug?.activeTextEditor,
    };
  });
}

function assertSnapshotContains(before, after, label) {
  const afterIds = new Set(after.objectIds);
  const missing = before.objectIds.filter((id) => !afterIds.has(id));
  assert(!missing.length, `${label} removed existing objects: ${JSON.stringify({ missing: missing.slice(0, 20), before, after })}`);
  assert(after.nodeCount >= before.nodeCount, `${label} reduced node count: ${JSON.stringify({ before, after })}`);
  assert(after.bondCount >= before.bondCount, `${label} reduced bond count: ${JSON.stringify({ before, after })}`);
}

async function firstVisibleNodeTarget(page) {
  return page.evaluate(() => {
    const candidates = [...document.querySelectorAll('[data-layer="document-content"] [data-node-id]')]
      .filter((element) => !element.classList.contains("document-diagnostic-marker"))
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          id: element.getAttribute("data-node-id"),
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
          area: rect.width * rect.height,
          visible: rect.width > 1 && rect.height > 1
            && rect.right >= 0 && rect.bottom >= 0
            && rect.left <= window.innerWidth && rect.top <= window.innerHeight,
        };
      })
      .filter((entry) => entry.visible);
    candidates.sort((a, b) => b.area - a.area);
    if (candidates[0]) {
      return candidates[0];
    }
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const doc = parse(debug?.state?.editorEngine?.documentJson?.()) || debug?.document || null;
    if (!doc || typeof debug?.worldToClient !== "function") {
      return null;
    }
    const fallback = [];
    for (const resource of Object.values(doc.resources || {})) {
      for (const node of resource?.data?.nodes || []) {
        const position = node.position || [];
        const client = debug.worldToClient(Number(position[0]), Number(position[1]));
        if (!client) {
          continue;
        }
        fallback.push({
          id: node.id,
          x: client.x,
          y: client.y,
          area: 0,
          visible: client.x >= 0 && client.y >= 0 && client.x <= window.innerWidth && client.y <= window.innerHeight,
        });
      }
    }
    return fallback.find((entry) => entry.visible) || fallback[0] || null;
  });
}

function expectedEngineToolForUiTool(tool) {
  return tool === "chain" ? "templates" : tool;
}

async function verifyPaletteAccessibleUnderTools(page) {
  const tools = ["bond", "select", "text", "arrow", "bracket", "symbol", "shape", "orbital", "templates", "chain"];
  for (const tool of tools) {
    await activateTool(page, tool);
    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-element").click();
    await page.waitForFunction(() => {
      const state = window.__chemcoreDebug?.editorState || {};
      const engineTool = window.__chemcoreDebug?.engineState?.tool?.activeTool
        || window.__chemcoreDebug?.engineState?.tool?.active_tool
        || "";
      return document.querySelector(".quick-palette")?.classList.contains("is-open")
        && document.querySelector(".quick-palette")?.dataset.mode === "element"
        && state.elementPlacementActive
        && engineTool === "element";
    });
    const elementState = await page.evaluate((expectedTool) => {
      const editorState = window.__chemcoreDebug?.editorState || {};
      return {
        open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
        mode: document.querySelector(".quick-palette")?.dataset.mode || "",
        activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
        activeTool: editorState.activeTool,
        elementPlacementActive: !!editorState.elementPlacementActive,
        engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "",
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        expectedTool,
      };
    }, tool);
    assert(
      elementState.open
        && elementState.mode === "element"
        && elementState.activeTool === tool
        && elementState.activeButtons.includes(tool)
        && elementState.elementPlacementActive
        && elementState.engineTool === "element"
        && !elementState.shieldActive,
      `Element quick palette failed under ${tool}: ${JSON.stringify(elementState)}`,
    );

    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-symbol").click();
    const expectedEngineTool = expectedEngineToolForUiTool(tool);
    try {
      await page.waitForFunction(({ expectedTool, expectedEngineTool }) => {
        const state = window.__chemcoreDebug?.editorState || {};
        const engineTool = window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "";
        return document.querySelector(".quick-palette")?.dataset.mode === "symbol"
          && state.activeTool === expectedTool
          && !state.elementPlacementActive
          && engineTool === expectedEngineTool;
      }, { expectedTool: tool, expectedEngineTool }, { timeout: 2000 });
    } catch (error) {
      const diagnostic = await page.evaluate(({ expectedTool, expectedEngineTool }) => {
        const editorState = window.__chemcoreDebug?.editorState || {};
        return {
          open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
          mode: document.querySelector(".quick-palette")?.dataset.mode || "",
          activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
          activeTool: editorState.activeTool,
          elementPlacementActive: !!editorState.elementPlacementActive,
          engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
            || window.__chemcoreDebug?.engineState?.tool?.active_tool
            || "",
          shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
          expectedTool,
          expectedEngineTool,
        };
      }, { expectedTool: tool, expectedEngineTool });
      throw new Error(`Symbol quick palette did not stabilize under ${tool}: ${JSON.stringify(diagnostic)}\n${error.message}`);
    }
    const symbolState = await page.evaluate(({ expectedTool, expectedEngineTool }) => {
      const editorState = window.__chemcoreDebug?.editorState || {};
      return {
        open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
        mode: document.querySelector(".quick-palette")?.dataset.mode || "",
        activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
        activeTool: editorState.activeTool,
        elementPlacementActive: !!editorState.elementPlacementActive,
        engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "",
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        expectedTool,
        expectedEngineTool,
      };
    }, { expectedTool: tool, expectedEngineTool });
    assert(
      symbolState.open
        && symbolState.mode === "symbol"
        && symbolState.activeTool === tool
        && symbolState.activeButtons.includes(tool)
        && !symbolState.elementPlacementActive
        && symbolState.engineTool === expectedEngineToolForUiTool(tool)
        && !symbolState.shieldActive,
      `Symbol quick palette failed under ${tool}: ${JSON.stringify(symbolState)}`,
    );
    await page.mouse.click(18, 18);
    await page.waitForTimeout(80);
  }
}

async function verifySelectAtomDragDoesNotCreateBonds(page) {
  const box = await viewerBox(page);
  const start = { x: box.x + box.width / 2 - 160, y: box.y + box.height / 2 - 40 };
  const end = { x: start.x + 120, y: start.y };
  await dragOnCanvas(page, "bond", start, end);
  const before = await documentSnapshot(page);
  const target = await firstVisibleNodeTarget(page);
  assert(target, `No atom target after creating a bond: ${JSON.stringify(before)}`);
  await activateTool(page, "select");
  for (const [dx, dy] of [[42, 12], [-24, 30], [36, -20], [-18, -16]]) {
    await page.mouse.move(target.x, target.y);
    await page.mouse.down();
    await page.mouse.move(target.x + dx, target.y + dy, { steps: 7 });
    await page.mouse.up();
    await page.waitForTimeout(130);
  }
  const after = await documentSnapshot(page);
  assert(after.bondCount === before.bondCount, `Select atom drags created bonds: ${JSON.stringify({ before, after })}`);
  assert(after.activeTool === "select" && after.engineTool === "select", `Select drag left wrong tool state: ${JSON.stringify(after)}`);
}

async function verifyBracketLabelCommitPreservesNewObjects(page) {
  const box = await viewerBox(page);
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  await dragOnCanvas(page, "bond", { x: cx - 260, y: cy - 140 }, { x: cx - 145, y: cy - 100 });
  await dragOnCanvas(page, "arrow", { x: cx - 60, y: cy - 150 }, { x: cx + 90, y: cy - 150 });
  await dragOnCanvas(page, "shape", { x: cx + 150, y: cy - 170 }, { x: cx + 245, y: cy - 90 });
  await clickCanvas(page, "symbol", { x: cx + 305, y: cy - 130 });
  await dragOnCanvas(page, "orbital", { x: cx - 240, y: cy + 45 }, { x: cx - 140, y: cy + 132 });
  await dragOnCanvas(page, "bracket", { x: cx + 20, y: cy + 20 }, { x: cx + 150, y: cy + 140 });
  await page.waitForFunction(() => !!window.__chemcoreDebug?.activeTextEditor, null, { timeout: 1500 });
  await page.locator(".text-editor-input").focus();
  await page.keyboard.type("3");
  await page.waitForFunction(() => window.__chemcoreDebug?.activeTextEditor?.plainText === "3", null, { timeout: 1000 });
  const beforeCommit = await documentSnapshot(page);
  await page.mouse.click(box.x + 28, box.y + 28);
  await page.waitForFunction(() => !window.__chemcoreDebug?.activeTextEditor, null, { timeout: 2000 });
  await page.waitForTimeout(220);
  const afterCommit = await documentSnapshot(page);
  assertSnapshotContains(beforeCommit, afterCommit, "Bracket label blank-click commit");
  assert(
    (afterCommit.typeCounts.text || 0) >= 1
      && afterCommit.objects.some((object) => object.type === "text" && object.text.includes("3")),
    `Bracket label text was not committed: ${JSON.stringify({ beforeCommit, afterCommit })}`,
  );
  for (const type of ["line", "shape", "bracket", "symbol"]) {
    assert((afterCommit.typeCounts[type] || 0) > 0, `Missing ${type} after mixed creation commit: ${JSON.stringify(afterCommit)}`);
  }
}

async function verifySyntheticLargeMixedOperations(page) {
  const synthetic = makeSyntheticLargeDocument({ nodeCount: syntheticNodeCount, objectRepeats: 36 });
  await page.evaluate((doc) => window.__chemcoreDebug.loadDocumentForTest(doc), synthetic);
  await page.waitForFunction((expected) => {
    const doc = window.__chemcoreDebug?.document;
    return (doc?.resources?.mol_large?.data?.nodes?.length || 0) >= expected;
  }, syntheticNodeCount, { timeout: 20000 });
  const loaded = await documentSnapshot(page);
  assert(loaded.nodeCount >= syntheticNodeCount, `Synthetic large document did not load: ${JSON.stringify(loaded)}`);

  await activateTool(page, "shape");
  await page.locator(".quick-palette-toggle-element").click();
  await page.waitForFunction(() => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state.activeTool === "shape" && state.elementPlacementActive;
  });
  await page.locator(".quick-palette-toggle-symbol").click();
  await page.waitForFunction(() => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state.activeTool === "shape" && !state.elementPlacementActive
      && document.querySelector(".quick-palette")?.dataset.mode === "symbol";
  });
  await page.mouse.click(16, 16);

  const target = await firstVisibleNodeTarget(page);
  assert(target, "Synthetic large document exposed no visible atom target.");
  await activateTool(page, "select");
  await page.mouse.move(target.x, target.y);
  await page.mouse.down();
  await page.mouse.move(target.x + 34, target.y + 18, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(150);

  const box = await viewerBox(page);
  await dragOnCanvas(
    page,
    "bracket",
    { x: box.x + box.width - 320, y: box.y + box.height - 240 },
    { x: box.x + box.width - 180, y: box.y + box.height - 120 },
  );
  await page.waitForFunction(() => !!window.__chemcoreDebug?.activeTextEditor, null, { timeout: 1500 });
  await page.locator(".text-editor-input").focus();
  await page.keyboard.type("2");
  const beforeCommit = await documentSnapshot(page);
  await page.mouse.click(box.x + 24, box.y + 24);
  await page.waitForFunction(() => !window.__chemcoreDebug?.activeTextEditor, null, { timeout: 2000 });
  await page.waitForTimeout(220);
  const afterCommit = await documentSnapshot(page);
  assertSnapshotContains(beforeCommit, afterCommit, "Synthetic large bracket label commit");
  assert(afterCommit.nodeCount >= syntheticNodeCount, `Large document nodes disappeared: ${JSON.stringify(afterCommit)}`);
  assert(afterCommit.bondCount >= loaded.bondCount, `Large document bonds disappeared: ${JSON.stringify({ loaded, afterCommit })}`);
}

async function runScenario(browser, name, scenario) {
  const { page, errors } = await openViewer(browser);
  const started = performance.now();
  try {
    await scenario(page);
    assert(!errors.length, `Console errors:\n${errors.join("\n")}`);
    const elapsed = performance.now() - started;
    return { name, ok: true, elapsedMs: elapsed };
  } catch (error) {
    return {
      name,
      ok: false,
      elapsedMs: performance.now() - started,
      message: error?.stack || error?.message || String(error),
      consoleErrors: errors,
    };
  } finally {
    await page.close().catch(() => {});
  }
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const scenarios = [
    ["quick palettes under every tool", verifyPaletteAccessibleUnderTools],
    ["select atom drag does not draw bonds", verifySelectAtomDragDoesNotCreateBonds],
    ["bracket label commit preserves new objects", verifyBracketLabelCommitPreservesNewObjects],
    ["synthetic large mixed operations", verifySyntheticLargeMixedOperations],
  ];
  const results = [];
  for (const [name, scenario] of scenarios) {
    const result = await runScenario(browser, name, scenario);
    results.push(result);
    const label = result.ok ? "ok" : "fail";
    console.log(`[stability-user-paths] ${label} ${name} (${result.elapsedMs.toFixed(1)}ms)`);
  }
  const failures = results.filter((result) => !result.ok);
  if (failures.length) {
    console.error(JSON.stringify({ ok: false, failures }, null, 2));
    process.exit(1);
  }
  const totalMs = results.reduce((sum, result) => sum + result.elapsedMs, 0);
  console.log(`[stability-user-paths] ok (${results.length} scenarios, ${totalMs.toFixed(1)}ms)`);
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
