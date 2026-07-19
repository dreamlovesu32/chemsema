import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const tmpDir = join(rootDir, "tmp", "gui-regression");
const exactTieOnly = process.env.CHEMSEMA_GUI_CASE === "exact-tie-double";

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

async function installBrowserMocks(context) {
  await context.addInitScript(() => {
    const encoder = new TextEncoder();
    window.__chemsemaSavePickerWrites = [];
    window.__chemsemaSavePickerQueue = [];
    window.__chemsemaSetSavePickerQueue = (names) => {
      window.__chemsemaSavePickerQueue = Array.from(names || []);
    };
    window.confirm = () => true;

    try {
      Object.defineProperty(window, "showOpenFilePicker", {
        value: undefined,
        configurable: true,
      });
    } catch {
      window.showOpenFilePicker = undefined;
    }

    Object.defineProperty(window, "showSaveFilePicker", {
      configurable: true,
      value: async (options = {}) => {
        const name = window.__chemsemaSavePickerQueue.shift()
          || options.suggestedName
          || "chemsema-document.ccjz";
        const record = {
          name,
          suggestedName: options.suggestedName || null,
          byteLength: 0,
          text: "",
          chunkTypes: [],
          closed: false,
        };
        window.__chemsemaSavePickerWrites.push(record);
        return {
          name,
          async createWritable() {
            return {
              async write(chunk) {
                record.chunkTypes.push(chunk?.constructor?.name || typeof chunk);
                if (typeof chunk === "string") {
                  record.text += chunk;
                  record.byteLength += encoder.encode(chunk).byteLength;
                  return;
                }
                if (chunk instanceof Blob) {
                  record.byteLength += chunk.size;
                  try {
                    record.text += await chunk.text();
                  } catch {
                    // Binary blobs are still covered by byteLength.
                  }
                  return;
                }
                if (chunk instanceof ArrayBuffer) {
                  record.byteLength += chunk.byteLength;
                  return;
                }
                if (ArrayBuffer.isView(chunk)) {
                  record.byteLength += chunk.byteLength;
                  return;
                }
                if (chunk != null) {
                  const text = String(chunk);
                  record.text += text;
                  record.byteLength += encoder.encode(text).byteLength;
                }
              },
              async close() {
                record.closed = true;
              },
            };
          },
        };
      },
    });
  });
}

async function openViewer(context, errors, viewport = { width: 1400, height: 1000 }) {
  const page = await context.newPage({ viewport });
  page.setDefaultTimeout(12000);
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?v=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await waitForReady(page);
  return page;
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => !!window.__chemsemaDebug?.state?.editorEngine && !!window.__chemsemaDebug?.document,
    null,
    { timeout: 30000 },
  );
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

async function documentSummary(page) {
  return page.evaluate(() => {
    const doc = window.__chemsemaDebug?.document || null;
    const resources = Object.values(doc?.resources || {});
    const resourceBonds = resources.reduce((sum, resource) => sum + (resource?.data?.bonds?.length || 0), 0);
    const resourceNodes = resources.reduce((sum, resource) => sum + (resource?.data?.nodes?.length || 0), 0);
    const selectionBounds = JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.selectionBoundsJson?.() || "null");
    return {
      title: doc?.document?.title || null,
      currentFileName: window.__chemsemaDebug?.state?.currentFileName || null,
      objects: doc?.objects?.length || 0,
      resourceBonds,
      resourceNodes,
      renderedBonds: document.querySelectorAll("[data-bond-id]").length,
      renderedNodes: document.querySelectorAll("[data-node-id]").length,
      selectionBounds,
      activeTool: window.__chemsemaDebug?.editorState?.activeTool || null,
      bondToolbarButtons: document.querySelectorAll('#secondary-toolbar [data-secondary-value^="bond-"]').length,
      blankToolbarIcons: [...document.querySelectorAll("#secondary-toolbar button")]
        .filter((button) => !button.querySelector("svg")).length,
    };
  });
}

async function setSaveQueue(page, names) {
  await page.evaluate((nextNames) => {
    window.__chemsemaSetSavePickerQueue(nextNames);
  }, names);
}

async function saveWrites(page) {
  return page.evaluate(() => window.__chemsemaSavePickerWrites);
}

async function waitForSaveWrite(page, index) {
  await page.waitForFunction(
    (targetIndex) => window.__chemsemaSavePickerWrites?.[targetIndex]?.closed === true,
    index,
  );
  return (await saveWrites(page))[index];
}

async function verifyToolbarAndCursor(page) {
  const tools = [
    { tool: "bond", minSecondary: 11 },
    { tool: "arrow", minSecondary: 6 },
    { tool: "shape", minSecondary: 5 },
    { tool: "templates", minSecondary: 5 },
  ];
  for (const item of tools) {
    await page.locator(`button[data-tool="${item.tool}"]`).click();
    await page.waitForFunction((tool) => {
      return document.querySelector(`button[data-tool="${tool}"]`)?.classList.contains("is-active");
    }, item.tool);
    const state = await page.evaluate((tool) => {
      const container = document.querySelector("#viewer-container");
      const svg = document.querySelector("#viewer-svg");
      const active = document.querySelector(`button[data-tool="${tool}"]`);
      return {
        tool,
        active: active?.classList.contains("is-active") || false,
        activeHasSvg: !!active?.querySelector("svg"),
        secondaryButtons: document.querySelectorAll("#secondary-toolbar button").length,
        secondarySvgs: document.querySelectorAll("#secondary-toolbar button svg").length,
        containerCursor: getComputedStyle(container).cursor,
        svgCursor: getComputedStyle(svg).cursor,
      };
    }, item.tool);
    assert.equal(state.active, true, `${item.tool} did not become active: ${JSON.stringify(state)}`);
    assert.equal(state.activeHasSvg, true, `${item.tool} tool button lost its icon: ${JSON.stringify(state)}`);
    assert(
      state.secondaryButtons >= item.minSecondary && state.secondarySvgs >= item.minSecondary,
      `${item.tool} secondary toolbar has blank/missing icons: ${JSON.stringify(state)}`,
    );
    assert(
      state.containerCursor !== "" && state.svgCursor !== "",
      `${item.tool} cursor styles were not applied: ${JSON.stringify(state)}`,
    );
  }
}

async function firstBondScreenGeometry(page) {
  return page.evaluate(() => {
    const svg = document.querySelector("#viewer-svg");
    const bond = document.querySelector('[data-role="document-bond"][data-bond-id]');
    if (!svg || !bond) {
      return null;
    }
    const toScreen = (x, y) => {
      const point = svg.createSVGPoint();
      point.x = x;
      point.y = y;
      const transformed = point.matrixTransform(svg.getScreenCTM());
      return { x: transformed.x, y: transformed.y };
    };
    if (bond.tagName.toLowerCase() === "line") {
      const start = toScreen(Number(bond.getAttribute("x1")), Number(bond.getAttribute("y1")));
      const end = toScreen(Number(bond.getAttribute("x2")), Number(bond.getAttribute("y2")));
      return {
        start,
        end,
        center: { x: (start.x + end.x) * 0.5, y: (start.y + end.y) * 0.5 },
      };
    }
    const box = bond.getBBox();
    const start = toScreen(box.x, box.y + box.height * 0.5);
    const end = toScreen(box.x + box.width, box.y + box.height * 0.5);
    return {
      start,
      end,
      center: { x: (start.x + end.x) * 0.5, y: (start.y + end.y) * 0.5 },
    };
  });
}

async function selectionOverlayRoles(page) {
  return page.evaluate(() => [...document.querySelectorAll('#viewer-svg [data-role^="selection-"]')]
    .map((element) => ({
      role: element.getAttribute("data-role"),
      tag: element.tagName.toLowerCase(),
      fill: element.getAttribute("fill"),
      stroke: element.getAttribute("stroke"),
      strokeWidth: element.getAttribute("stroke-width"),
    })));
}

function assertBondSelectionDot(roles, label) {
  const dots = roles.filter((entry) => entry.role === "selection-bond-dot");
  assert.equal(dots.length, 1, `${label} should render exactly one selected bond center dot: ${JSON.stringify(roles)}`);
  assert(
    dots.every((dot) => dot.stroke !== "#ffffff" && dot.strokeWidth === "0"),
    `${label} selected bond dot should not paint a white stroke over the bond: ${JSON.stringify(dots)}`,
  );
}

async function verifySelectionOverlayConsistency(page) {
  await drawBondWithMouse(page);
  const geometry = await firstBondScreenGeometry(page);
  assert(geometry, "Could not locate the drawn bond for selection overlay checks.");

  await page.locator('button[data-tool="select"]').click();
  await page.mouse.click(geometry.center.x, geometry.center.y);
  await page.waitForFunction(() => document.querySelector('[data-role="selection-bond-dot"]'));
  const clickRoles = await selectionOverlayRoles(page);
  assert(clickRoles.some((entry) => entry.role === "selection-bond"), `Click-select did not render the bond box: ${JSON.stringify(clickRoles)}`);
  assert(!clickRoles.some((entry) => entry.role === "selection-box"), `Click-select should not render an outer component box: ${JSON.stringify(clickRoles)}`);
  assertBondSelectionDot(clickRoles, "Click-select");

  const margin = 18;
  await page.mouse.move(geometry.start.x - margin, geometry.start.y - margin);
  await page.mouse.down();
  await page.mouse.move(geometry.end.x + margin, geometry.end.y + margin, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('[data-role="selection-box"]'));
  const boxRoles = await selectionOverlayRoles(page);
  assert(boxRoles.some((entry) => entry.role === "selection-box"), `Box-select did not render the component selection box: ${JSON.stringify(boxRoles)}`);
  assert(
    !boxRoles.some((entry) => entry.role === "selection-bond-dot"),
    `Box-selecting a complete molecule should suppress internal bond center dots: ${JSON.stringify(boxRoles)}`,
  );
}

function makeShortDoubleBondDocument() {
  return {
    format: { name: "chemsema", version: "0.1" },
    document: {
      id: "doc_short_double",
      title: "short double bond",
      page: { width: 220, height: 180, background: "#ffffff" },
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        strokeWidth: 1,
        fontFamily: "Arial",
        fontSize: 10,
      },
    },
    objects: [{
      id: "obj_molecule_001",
      type: "molecule",
      visible: true,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: { resourceRef: "mol_001" },
    }],
    resources: {
      mol_001: {
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: {
          schema: "chemsema.molecule.fragment2d",
          bbox: [95, 95, 20, 10],
          nodes: [
            { id: "n1", element: "C", atomicNumber: 6, position: [100, 100], charge: 0, numHydrogens: 0 },
            { id: "n2", element: "C", atomicNumber: 6, position: [110, 100], charge: 0, numHydrogens: 0 },
          ],
          bonds: [{ id: "b1", begin: "n1", end: "n2", order: 2 }],
        },
      },
    },
  };
}

function makeExactTieDoubleBondDocument() {
  return {
    format: { name: "chemsema", version: "0.1" },
    document: {
      id: "doc_exact_tie_double",
      title: "exact tie double bond",
      page: { width: 260, height: 220, background: "#ffffff" },
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        strokeWidth: 1,
        fontFamily: "Arial",
        fontSize: 10,
      },
    },
    objects: [{
      id: "obj_molecule_001",
      type: "molecule",
      visible: true,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: { resourceRef: "mol_001" },
    }],
    resources: {
      mol_001: {
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: {
          schema: "chemsema.molecule.fragment2d",
          bbox: [100, 70, 30, 60],
          nodes: [
            { id: "n1", element: "C", atomicNumber: 6, position: [100, 100], charge: 0, numHydrogens: 0 },
            { id: "n2", element: "C", atomicNumber: 6, position: [130, 100], charge: 0, numHydrogens: 0 },
            { id: "n3", element: "C", atomicNumber: 6, position: [100, 70], charge: 0, numHydrogens: 0 },
            { id: "n4", element: "C", atomicNumber: 6, position: [100, 130], charge: 0, numHydrogens: 0 },
            { id: "n5", element: "C", atomicNumber: 6, position: [130, 70], charge: 0, numHydrogens: 0 },
          ],
          bonds: [
            { id: "b1", begin: "n1", end: "n2", order: 2, double: { placement: "right", frozen: false } },
            { id: "b2", begin: "n1", end: "n3", order: 1 },
            { id: "b3", begin: "n1", end: "n4", order: 1 },
            { id: "b4", begin: "n2", end: "n5", order: 1 },
          ],
        },
      },
    },
  };
}

async function verifyExactTieDoubleBondIncrementalReplacement(page) {
  await page.evaluate((doc) => window.__chemsemaDebug.loadDocumentForTest(doc), makeExactTieDoubleBondDocument());
  await page.waitForFunction(() => document.querySelectorAll('[data-layer="document-content"] [data-bond-id="b1"]').length === 2);
  await page.locator('button[data-tool="bond"]').click();
  await page.locator('button[data-secondary-value="bond-single"]').click();
  const points = await page.evaluate(() => ({
    start: window.__chemsemaDebug.worldToClient(130, 100),
    end: window.__chemsemaDebug.worldToClient(130, 130),
  }));

  await page.mouse.move(points.start.x, points.start.y);
  await page.mouse.down();
  await page.mouse.move(points.end.x, points.end.y, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(300);

  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    const fragment = doc.resources?.mol_001?.data;
    return {
      command,
      bondCount: fragment?.bonds?.length || 0,
      placement: fragment?.bonds?.find((bond) => bond.id === "b1")?.double?.placement || null,
      centralPrimitiveCount: document.querySelectorAll(
        '[data-layer="document-content"] [data-bond-id="b1"]',
      ).length,
    };
  });
  const changedBonds = new Set([
    ...(result.command?.targets?.bonds || []),
    ...(result.command?.updated?.bonds || []),
  ]);
  assert.equal(result.bondCount, 5, `Fourth attachment was not committed: ${JSON.stringify(result)}`);
  assert.equal(result.placement, "left", `Exact tie did not follow the last attachment side: ${JSON.stringify(result)}`);
  assert(changedBonds.has("b1"), `Fourth attachment did not target the center double bond: ${JSON.stringify(result)}`);
  assert.equal(
    result.centralPrimitiveCount,
    2,
    `Incremental render left stale center-double primitives: ${JSON.stringify(result)}`,
  );
}

async function verifyDeleteToolFocusedBondCenter(page) {
  await page.evaluate((doc) => window.__chemsemaDebug.loadDocumentForTest(doc), makeShortDoubleBondDocument());
  await page.waitForFunction(() => document.querySelector('[data-bond-id="b1"]'));
  await page.locator('button[data-tool="delete"]').click();
  const center = await page.evaluate(() => window.__chemsemaDebug.worldToClient(105, 100));

  await page.mouse.move(center.x, center.y);
  await page.mouse.click(center.x, center.y);

  await page.waitForFunction(() => {
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return doc.resources?.mol_001?.data?.bonds?.[0]?.order === 1;
  });
  const after = await page.evaluate(() => {
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return doc.resources?.mol_001?.data || null;
  });
  assert.equal(after?.nodes?.length, 2, `Delete tool removed the short double bond endpoint: ${JSON.stringify(after)}`);
  assert.equal(after?.bonds?.length, 1, `Delete tool removed the short double bond instead of degrading it: ${JSON.stringify(after)}`);
  assert.equal(after?.bonds?.[0]?.order, 1, `Delete tool did not degrade the focused double bond: ${JSON.stringify(after)}`);
}

async function verifyCopyPasteCut(page) {
  await drawBondWithMouse(page);
  const before = await documentSummary(page);
  await page.keyboard.press("Control+A");
  await page.waitForFunction(() => JSON.parse(window.__chemsemaDebug.state.editorEngine.selectionBoundsJson() || "null"));
  const selected = await documentSummary(page);
  assert(selected.selectionBounds, `Ctrl+A did not create a selection: ${JSON.stringify(selected)}`);

  await page.locator('button[data-command="copy"]').click();
  const copied = await page.evaluate(() => {
    const engine = window.__chemsemaDebug.state.editorEngine;
    return {
      hasClipboard: engine.hasClipboard?.() || false,
      fragmentLength: engine.clipboardSelectionJson?.()?.length || 0,
      documentLength: engine.clipboardDocumentJson?.()?.length || 0,
    };
  });
  assert(copied.hasClipboard && copied.fragmentLength > 20, `Copy did not populate the internal clipboard: ${JSON.stringify(copied)}`);

  await page.locator('button[data-command="paste"]').click();
  await page.waitForFunction((count) => document.querySelectorAll("[data-bond-id]").length > count, before.renderedBonds);
  const pasted = await documentSummary(page);
  assert(pasted.renderedBonds > before.renderedBonds, `Paste did not duplicate the selected structure: ${JSON.stringify({ before, pasted })}`);

  await page.keyboard.press("Control+A");
  await page.locator('button[data-command="cut"]').click();
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length === 0);
  const cut = await documentSummary(page);
  assert.equal(cut.renderedBonds, 0, `Cut did not remove selected content: ${JSON.stringify(cut)}`);

  await page.locator('button[data-command="paste"]').click();
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
  const restored = await documentSummary(page);
  assert(restored.renderedBonds > 0, `Paste after cut did not restore content: ${JSON.stringify(restored)}`);
}

async function verifySaveAsFormats(page) {
  await drawBondWithMouse(page);
  await setSaveQueue(page, ["gui-save.ccjs", "gui-export.cdxml", "gui-export.svg", "gui-save-again.ccjz"]);

  await page.locator('button[data-command="save-as"]').click();
  const ccjs = await waitForSaveWrite(page, 0);
  assert.equal(ccjs.name, "gui-save.ccjs");
  assert(ccjs.text.includes('"objects"') && JSON.parse(ccjs.text).resources, "Save As .ccjs did not write a ChemSema JSON document.");

  await page.locator('button[data-command="save-as"]').click();
  const cdxml = await waitForSaveWrite(page, 1);
  assert.equal(cdxml.name, "gui-export.cdxml");
  assert(/<CDXML\b/i.test(cdxml.text), "Save As .cdxml did not write CDXML content.");

  await page.locator('button[data-command="save-as"]').click();
  const svg = await waitForSaveWrite(page, 2);
  assert.equal(svg.name, "gui-export.svg");
  assert(/<svg\b/i.test(svg.text), "Save As .svg did not write SVG content.");

  await page.keyboard.press("Control+S");
  const ccjz = await waitForSaveWrite(page, 3);
  assert.equal(ccjz.name, "gui-save-again.ccjz");
  assert(ccjz.byteLength > 100, `Ctrl+S .ccjz save was unexpectedly small: ${JSON.stringify(ccjz)}`);
}

async function createOpenFixture(context, errors) {
  const page = await openViewer(context, errors);
  await drawBondWithMouse(page);
  const documentJson = await page.evaluate(() => window.__chemsemaDebug.state.editorEngine.documentJson());
  await page.close();
  const fixturePath = join(tmpDir, "gui-open-source.ccjs");
  writeFileSync(fixturePath, `${JSON.stringify(JSON.parse(documentJson), null, 2)}\n`, "utf8");
  return fixturePath;
}

async function verifyOpenButton(context, errors, fixturePath) {
  const page = await openViewer(context, errors);
  const popupPromise = page.waitForEvent("popup", { timeout: 8000 }).catch(() => null);
  const chooserPromise = page.waitForEvent("filechooser");
  await page.locator('button[data-command="open"]').click();
  const chooser = await chooserPromise;
  await chooser.setFiles(fixturePath);
  const popup = await popupPromise;
  const openedPage = popup || page;
  if (popup) {
    capturePageErrors(popup, errors);
    popup.setDefaultTimeout(12000);
  }
  await waitForReady(openedPage);
  await openedPage.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
  const opened = await documentSummary(openedPage);
  assert.equal(opened.currentFileName, "gui-open-source.ccjs", `Open did not preserve the file name: ${JSON.stringify(opened)}`);
  assert(opened.renderedBonds > 0 && opened.resourceBonds > 0, `Open did not render the saved document: ${JSON.stringify(opened)}`);
  await openedPage.close();
  if (popup) {
    await page.close().catch(() => {});
  }
}

async function verifyZoomAndStyleMenu(page) {
  await page.locator("#zoom-input").selectOption("150");
  const zoom = await page.locator("#zoom-input").inputValue();
  assert.equal(zoom, "150", "Zoom select did not accept 150%.");

  await page.locator("#document-style-button").click();
  await page.locator('[data-document-style-preset="acs-document-1996"]').click();
  const styleState = await page.evaluate(() => ({
    menuHidden: document.querySelector("#document-style-menu")?.hidden,
    preset: window.__chemsemaDebug?.state?.editorEngine?.documentStylePreset?.() || null,
  }));
  assert.equal(styleState.menuHidden, true, `Style menu did not close after selection: ${JSON.stringify(styleState)}`);
  assert.equal(styleState.preset, "acs-document-1996", `Style preset was not applied: ${JSON.stringify(styleState)}`);
}

let server = null;
let browser = null;
try {
  mkdirSync(tmpDir, { recursive: true });
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const context = await browser.newContext({ acceptDownloads: true });
  await installBrowserMocks(context);
  const errors = [];

  const fixturePath = await createOpenFixture(context, errors);
  await verifyOpenButton(context, errors, fixturePath);

  const exactTiePage = await openViewer(context, errors);
  await verifyExactTieDoubleBondIncrementalReplacement(exactTiePage);
  await exactTiePage.close();

  if (!exactTieOnly) {
    const page = await openViewer(context, errors);
    await verifyToolbarAndCursor(page);
    await verifySelectionOverlayConsistency(page);
    await verifyDeleteToolFocusedBondCenter(page);
    await page.close();

    const editPage = await openViewer(context, errors);
    await verifyCopyPasteCut(editPage);
    await editPage.close();

    const savePage = await openViewer(context, errors);
    await verifySaveAsFormats(savePage);
    await verifyZoomAndStyleMenu(savePage);
    await savePage.close();
  }

  assert.equal(errors.length, 0, `GUI regression saw console/page errors:\n${errors.join("\n")}`);
  console.log(exactTieOnly
    ? "[gui-regression] ok (exact-tie double bond)"
    : "[gui-regression] ok (open, save-as ccjs/cdxml/svg, ctrl+s ccjz, copy/paste/cut, toolbar icons, cursors, selection overlay, delete tool, exact-tie double bond, zoom, style)");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
