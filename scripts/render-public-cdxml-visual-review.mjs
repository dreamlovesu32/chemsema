import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { launchBrowser } from "./playwright-browser.mjs";

const execFileAsync = promisify(execFile);

function parseArgs(argv) {
  const args = {
    root: "tmp/public-corpus-pilot",
    outDir: "tmp/public-cdxml-visual-review",
    cli: process.platform === "win32"
      ? "target/debug/chemsema-cli.exe"
      : "target/debug/chemsema-cli",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--root") args.root = argv[++index];
    else if (arg === "--out") args.outDir = argv[++index];
    else if (arg === "--cli") args.cli = argv[++index];
    else if (arg === "--help" || arg === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return args;
}

async function walk(directory) {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await walk(absolute));
    else files.push(absolute);
  }
  return files;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function safeName(value) {
  return value
    .replace(/\.cdxml$/i, "")
    .replace(/[^a-z0-9._-]+/gi, "_")
    .replace(/^_+|_+$/g, "");
}

function dataUrl(buffer, mimeType) {
  return `data:${mimeType};base64,${buffer.toString("base64")}`;
}

function reviewScreenshotHtml(item, referenceDataUrl, chemsemaDataUrl) {
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<style>
  * { box-sizing: border-box; }
  html, body { margin: 0; width: 2520px; min-height: 1080px; background: #f4f5f7; color: #15171a; font-family: "Segoe UI", "Microsoft YaHei", Arial, sans-serif; }
  body { padding: 24px 30px 30px; }
  header { height: 76px; display: flex; flex-direction: column; justify-content: center; }
  h1 { margin: 0; font-size: 23px; line-height: 32px; }
  .source { color: #60656d; font-size: 14px; line-height: 22px; }
  main { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; }
  section { min-width: 0; }
  h2 { margin: 0 0 8px; font-size: 18px; line-height: 26px; }
  .panel { width: 1218px; height: 920px; display: flex; align-items: center; justify-content: center; overflow: hidden; background: #fff; border: 1px solid #cfd3d8; border-radius: 6px; }
  img { display: block; width: calc(100% - 32px); height: calc(100% - 32px); object-fit: contain; }
</style>
</head>
<body>
  <header>
    <h1>${escapeHtml(item.number)} / ${escapeHtml(item.total)} · ${escapeHtml(item.title)}</h1>
    <div class="source">${escapeHtml(item.relativeCdxml)}</div>
  </header>
  <main>
    <section><h2>公共参考图（原始 PNG）</h2><div class="panel"><img src="${referenceDataUrl}"></div></section>
    <section><h2>ChemSema 导入结果（完整 SVG 渲染）</h2><div class="panel"><img src="${chemsemaDataUrl}"></div></section>
  </main>
</body>
</html>`;
}

function viewerHtml(items) {
  const data = JSON.stringify(items).replaceAll("<", "\\u003c");
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>公共 CDXML 肉眼审查</title>
<style>
  :root { color-scheme: light; font-family: "Segoe UI", "Microsoft YaHei", Arial, sans-serif; }
  * { box-sizing: border-box; }
  html, body { margin: 0; height: 100%; color: #17191c; background: #f2f3f5; }
  body { display: grid; grid-template-columns: 310px minmax(0, 1fr); }
  aside { overflow: auto; border-right: 1px solid #ccd0d5; background: #fff; padding: 16px 12px; }
  aside h1 { margin: 0 6px 4px; font-size: 18px; }
  .summary { margin: 0 6px 14px; color: #676c74; font-size: 13px; }
  .item-button { width: 100%; margin: 0 0 5px; padding: 8px 10px; border: 1px solid transparent; border-radius: 5px; background: transparent; text-align: left; cursor: pointer; font-size: 13px; }
  .item-button:hover { background: #f1f4f8; }
  .item-button.active { border-color: #5591d4; background: #eaf3ff; }
  .item-button[data-state="pass"]::before { content: "✓ "; color: #198754; font-weight: 700; }
  .item-button[data-state="issue"]::before { content: "! "; color: #c33; font-weight: 700; }
  main { min-width: 0; display: grid; grid-template-rows: auto minmax(0, 1fr) auto; gap: 12px; padding: 16px 18px; }
  header { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 12px; }
  button, textarea, input { font: inherit; }
  .nav { padding: 7px 12px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .title { min-width: 0; }
  .title strong, .title span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title span { margin-top: 2px; color: #676c74; font-size: 12px; }
  .mode { display: flex; gap: 6px; }
  .mode button { padding: 7px 10px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .mode button.active { color: #fff; border-color: #2869b3; background: #2869b3; }
  .stage { min-height: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .stage.overlay { display: block; position: relative; }
  figure { min-width: 0; min-height: 0; margin: 0; display: grid; grid-template-rows: auto minmax(0, 1fr); }
  figcaption { padding: 0 0 7px; font-weight: 600; }
  .canvas { min-height: 0; display: flex; align-items: center; justify-content: center; overflow: auto; border: 1px solid #cbd0d6; border-radius: 6px; background: #fff; }
  .canvas img { display: block; width: calc(100% - 24px); height: calc(100% - 24px); object-fit: contain; }
  .stage.overlay figure { position: absolute; inset: 0; grid-template-rows: 0 minmax(0, 1fr); }
  .stage.overlay figure figcaption { visibility: hidden; }
  .stage.overlay figure:last-child { pointer-events: none; }
  .stage.overlay .canvas { border: 0; background: transparent; overflow: hidden; }
  .stage.overlay figure:first-child .canvas { border: 1px solid #cbd0d6; background: #fff; }
  .stage.overlay figure:last-child img { opacity: var(--overlay-opacity, .5); }
  .review { display: grid; grid-template-columns: auto auto minmax(240px, 1fr); gap: 10px; align-items: center; }
  .review button { padding: 8px 13px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .review button.pass.active { color: #fff; border-color: #198754; background: #198754; }
  .review button.issue.active { color: #fff; border-color: #c33; background: #c33; }
  textarea { width: 100%; height: 42px; resize: vertical; border: 1px solid #bfc4ca; border-radius: 5px; padding: 8px; }
  .overlay-controls { display: none; align-items: center; gap: 8px; font-size: 13px; }
  .overlay-controls.visible { display: flex; }
  @media (max-width: 1000px) { body { grid-template-columns: 230px minmax(0, 1fr); } .stage { grid-template-columns: 1fr; overflow: auto; } figure { min-height: 55vh; } }
</style>
</head>
<body>
<aside>
  <h1>公共 CDXML 肉眼审查</h1>
  <p class="summary"><span id="progress"></span> · 共 ${items.length} 组</p>
  <div id="list"></div>
</aside>
<main>
  <header>
    <button class="nav" id="previous">← 上一张</button>
    <div class="title"><strong id="title"></strong><span id="source"></span></div>
    <div class="mode">
      <button id="sideMode" class="active">左右对照</button>
      <button id="overlayMode">透明叠加</button>
      <span class="overlay-controls" id="overlayControls">参考图<input id="opacity" type="range" min="0" max="100" value="50">ChemSema</span>
      <button class="nav" id="next">下一张 →</button>
    </div>
  </header>
  <div class="stage" id="stage">
    <figure><figcaption>公共参考图（原始 PNG）</figcaption><div class="canvas"><img id="reference"></div></figure>
    <figure><figcaption>ChemSema 导入结果（完整 SVG 渲染）</figcaption><div class="canvas"><img id="chemsema"></div></figure>
  </div>
  <div class="review">
    <button class="pass" id="pass">✓ 通过</button>
    <button class="issue" id="issue">! 有问题</button>
    <textarea id="notes" placeholder="可选：记录标签、位置、分子或其他问题；内容会保存在当前浏览器。"></textarea>
  </div>
</main>
<script>
  const items = ${data};
  const storageKey = "chemsema-public-cdxml-visual-review-v1";
  const saved = JSON.parse(localStorage.getItem(storageKey) || "{}");
  let index = Math.max(0, Math.min(items.length - 1, Number(location.hash.slice(1)) - 1 || 0));
  const list = document.getElementById("list");
  const buttons = items.map((item, itemIndex) => {
    const button = document.createElement("button");
    button.className = "item-button";
    button.textContent = item.label;
    button.onclick = () => show(itemIndex);
    list.append(button);
    return button;
  });
  function persist() { localStorage.setItem(storageKey, JSON.stringify(saved)); }
  function updateProgress() {
    const reviewed = items.filter((item) => saved[item.id]?.state).length;
    document.getElementById("progress").textContent = "已审 " + reviewed;
  }
  function show(nextIndex) {
    index = (nextIndex + items.length) % items.length;
    const item = items[index];
    location.hash = String(index + 1);
    document.getElementById("title").textContent = item.label;
    document.getElementById("source").textContent = item.relativeCdxml;
    document.getElementById("reference").src = item.reference;
    document.getElementById("chemsema").src = item.chemsema;
    document.getElementById("notes").value = saved[item.id]?.notes || "";
    document.getElementById("pass").classList.toggle("active", saved[item.id]?.state === "pass");
    document.getElementById("issue").classList.toggle("active", saved[item.id]?.state === "issue");
    buttons.forEach((button, buttonIndex) => {
      button.classList.toggle("active", buttonIndex === index);
      button.dataset.state = saved[items[buttonIndex].id]?.state || "";
    });
    buttons[index].scrollIntoView({ block: "nearest" });
    updateProgress();
  }
  function setState(state) {
    const id = items[index].id;
    saved[id] = { ...(saved[id] || {}), state: saved[id]?.state === state ? "" : state };
    persist();
    show(index);
  }
  document.getElementById("previous").onclick = () => show(index - 1);
  document.getElementById("next").onclick = () => show(index + 1);
  document.getElementById("pass").onclick = () => setState("pass");
  document.getElementById("issue").onclick = () => setState("issue");
  document.getElementById("notes").oninput = (event) => {
    const id = items[index].id;
    saved[id] = { ...(saved[id] || {}), notes: event.target.value };
    persist();
  };
  document.getElementById("sideMode").onclick = () => {
    document.getElementById("stage").classList.remove("overlay");
    document.getElementById("sideMode").classList.add("active");
    document.getElementById("overlayMode").classList.remove("active");
    document.getElementById("overlayControls").classList.remove("visible");
  };
  document.getElementById("overlayMode").onclick = () => {
    document.getElementById("stage").classList.add("overlay");
    document.getElementById("sideMode").classList.remove("active");
    document.getElementById("overlayMode").classList.add("active");
    document.getElementById("overlayControls").classList.add("visible");
  };
  document.getElementById("opacity").oninput = (event) => {
    document.getElementById("stage").style.setProperty("--overlay-opacity", Number(event.target.value) / 100);
  };
  addEventListener("keydown", (event) => {
    if (event.target.matches("textarea, input")) return;
    if (event.key === "ArrowLeft") show(index - 1);
    if (event.key === "ArrowRight") show(index + 1);
    if (event.key.toLowerCase() === "p") setState("pass");
    if (event.key.toLowerCase() === "i") setState("issue");
  });
  show(index);
</script>
</body>
</html>`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/render-public-cdxml-visual-review.mjs [--root corpus] [--out directory] [--cli chemsema-cli]");
    return;
  }

  const root = path.resolve(args.root);
  const outDir = path.resolve(args.outDir);
  const cli = path.resolve(args.cli);
  const allFiles = await walk(root);
  const pairs = allFiles
    .filter((file) => file.toLowerCase().endsWith(".cdxml"))
    .map((cdxml) => ({ cdxml, png: cdxml.replace(/\.cdxml$/i, ".png") }))
    .filter(({ png }) => allFiles.includes(png))
    .sort((left, right) => left.cdxml.localeCompare(right.cdxml, "en"));

  if (pairs.length === 0) throw new Error(`No matching CDXML/PNG pairs found under ${root}`);
  await fs.access(cli);
  await fs.mkdir(path.join(outDir, "items"), { recursive: true });

  const browser = await launchBrowser({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 2520, height: 1080 },
    deviceScaleFactor: 1.5,
  });
  const page = await context.newPage();
  const manifestItems = [];

  try {
    for (let index = 0; index < pairs.length; index += 1) {
      const pair = pairs[index];
      const relativeCdxml = path.relative(root, pair.cdxml).replaceAll("\\", "/");
      const prefix = String(index + 1).padStart(3, "0");
      const id = `${prefix}_${safeName(relativeCdxml)}`;
      const itemDir = path.join(outDir, "items", id);
      const referencePath = path.join(itemDir, "reference.png");
      const chemsemaPath = path.join(itemDir, "chemsema.svg");
      const comparisonPath = path.join(itemDir, "comparison.png");
      await fs.mkdir(itemDir, { recursive: true });
      await fs.copyFile(pair.png, referencePath);
      await execFileAsync(cli, ["convert", pair.cdxml, chemsemaPath], {
        cwd: process.cwd(),
        maxBuffer: 16 * 1024 * 1024,
      });

      const [referenceBuffer, chemsemaBuffer] = await Promise.all([
        fs.readFile(referencePath),
        fs.readFile(chemsemaPath),
      ]);
      const screenshotItem = {
        number: String(index + 1),
        total: String(pairs.length),
        title: path.basename(pair.cdxml, path.extname(pair.cdxml)),
        relativeCdxml,
      };
      await page.setContent(
        reviewScreenshotHtml(
          screenshotItem,
          dataUrl(referenceBuffer, "image/png"),
          dataUrl(chemsemaBuffer, "image/svg+xml"),
        ),
        { waitUntil: "networkidle" },
      );
      await page.screenshot({ path: comparisonPath, fullPage: true });

      manifestItems.push({
        id,
        label: `${String(index + 1).padStart(2, "0")} · ${screenshotItem.title}`,
        relativeCdxml,
        sourceCdxml: pair.cdxml,
        sourceReference: pair.png,
        reference: `items/${id}/reference.png`,
        chemsema: `items/${id}/chemsema.svg`,
        comparison: `items/${id}/comparison.png`,
      });
      console.log(`[${index + 1}/${pairs.length}] ${relativeCdxml}`);
    }
  } finally {
    await browser.close();
  }

  await Promise.all([
    fs.writeFile(path.join(outDir, "index.html"), viewerHtml(manifestItems)),
    fs.writeFile(
      path.join(outDir, "manifest.json"),
      `${JSON.stringify({ generatedAt: new Date().toISOString(), count: manifestItems.length, items: manifestItems }, null, 2)}\n`,
    ),
  ]);
  console.log(`Review gallery: ${pathToFileURL(path.join(outDir, "index.html")).href}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack || error.message : String(error));
    process.exit(1);
  });
}
