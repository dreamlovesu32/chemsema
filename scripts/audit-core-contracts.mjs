import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
const jsonOnly = args.includes("--json");
const reportArg = args.indexOf("--report");
const reportPath = reportArg >= 0 ? resolve(rootDir, args[reportArg + 1]) : null;
const failOnError = args.includes("--fail-on-error");
const findings = [];
const architectureReviewLedger = JSON.parse(
  readFileSync(join(rootDir, "docs", "architecture-review-ledger.json"), "utf8"),
);
const verifiedArchitectureReviews = [];

const SOURCE_ROOTS = [
  "crates/chemsema-engine/src",
  "crates/chemsema-cli/src",
  "apps/chemsema-desktop/src-tauri/src",
  "viewer",
];
const EXCLUDED_PARTS = [
  "/viewer/engine/",
  "/viewer/inchi/",
  "/node_modules/",
  "/target/",
];
const SOURCE_EXTENSIONS = new Set([".rs", ".js", ".mjs", ".ts"]);
const OBJECT_TYPES = [
  "molecule",
  "text",
  "line",
  "curve",
  "bracket",
  "symbol",
  "shape",
  "image",
  "group",
];
const OBJECT_VARIANTS = Object.fromEntries(OBJECT_TYPES.map((type) => [
  type,
  type[0].toUpperCase() + type.slice(1),
]));

function normalizePath(path) {
  return path.replaceAll("\\", "/");
}

function repoPath(path) {
  return normalizePath(relative(rootDir, path));
}

function lineAt(text, index) {
  return text.slice(0, Math.max(0, index)).split("\n").length;
}

function addFinding(severity, category, rule, file, index, message, evidence = "") {
  findings.push({
    severity,
    category,
    rule,
    file: normalizePath(file),
    line: index == null ? null : lineAt(read(file), index),
    message,
    evidence: String(evidence || "").trim(),
  });
}

const fileCache = new Map();
function read(path) {
  const absolute = resolve(rootDir, path);
  if (!fileCache.has(absolute)) {
    fileCache.set(absolute, readFileSync(absolute, "utf8"));
  }
  return fileCache.get(absolute);
}

function walk(path) {
  const normalized = normalizePath(path);
  if (EXCLUDED_PARTS.some((part) => normalized.includes(part))) {
    return [];
  }
  const stat = statSync(path);
  if (stat.isFile()) {
    return SOURCE_EXTENSIONS.has(extname(path)) ? [path] : [];
  }
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => (
    walk(join(path, entry.name))
  ));
}

const sourceFiles = SOURCE_ROOTS
  .flatMap((path) => walk(join(rootDir, path)))
  .filter((path) => !normalizePath(path).includes("/tests/"));

function balancedBlock(text, openingBrace) {
  let depth = 0;
  let string = null;
  let escaped = false;
  let lineComment = false;
  let blockComment = false;
  for (let index = openingBrace; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];
    if (lineComment) {
      if (char === "\n") lineComment = false;
      continue;
    }
    if (blockComment) {
      if (char === "*" && next === "/") {
        blockComment = false;
        index += 1;
      }
      continue;
    }
    if (string) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === string) {
        string = null;
      }
      continue;
    }
    if (char === "/" && next === "/") {
      lineComment = true;
      index += 1;
      continue;
    }
    if (char === "/" && next === "*") {
      blockComment = true;
      index += 1;
      continue;
    }
    if (char === '"' || char === "'" || char === "`") {
      string = char;
      continue;
    }
    if (char === "{") depth += 1;
    if (char === "}") {
      depth -= 1;
      if (depth === 0) return text.slice(openingBrace, index + 1);
    }
  }
  return "";
}

function findFunction(path, name) {
  const text = read(path);
  const patterns = [
    new RegExp(`(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${name}\\s*\\(`),
    new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`),
    new RegExp(`^\\s*(?:async\\s+)?${name}\\s*\\([^)]*\\)\\s*\\{`, "m"),
  ];
  for (const pattern of patterns) {
    const match = pattern.exec(text);
    if (!match) continue;
    const openingBrace = text.indexOf("{", match.index + match[0].length - 1);
    const body = balancedBlock(text, openingBrace);
    if (body) return { body, index: match.index, line: lineAt(text, match.index) };
  }
  return null;
}

function hasType(body, type) {
  return new RegExp(`["']${type}["']`).test(body)
    || body.includes(`SceneObjectKind::${OBJECT_VARIANTS[type]}`)
    || body.includes("SceneObjectKind::ALL");
}

function auditObjectCapabilities() {
  const registryPath = "crates/chemsema-engine/src/object_kind.rs";
  const registryText = read(registryPath);
  for (const type of OBJECT_TYPES) {
    const variant = OBJECT_VARIANTS[type];
    if (!registryText.includes(`Self::${variant}`)) {
      addFinding("error", "object model", "OBJECT-REGISTRY-INCOMPLETE", registryPath, 0,
        `Scene object registry does not include '${type}'.`);
    }
  }
  const surfaces = [
    ["render", "crates/chemsema-engine/src/render.rs", "render_scene_object"],
    ["selection coverage", "crates/chemsema-engine/src/engine/select/render.rs", "scene_object_selection_coverage"],
    ["selectable", "crates/chemsema-engine/src/engine/select/render.rs", "scene_object_is_selectable"],
    ["select all", "crates/chemsema-engine/src/engine/select.rs", "select_all"],
    ["clipboard completeness", "crates/chemsema-engine/src/engine/clipboard.rs", "visible_root_object_is_selected_for_clipboard"],
    ["rotation", "crates/chemsema-engine/src/engine/select/drag.rs", "rotated_scene_object"],
    ["transform policy", "crates/chemsema-engine/src/engine/select/drag.rs", "object_transform_participates_in_render"],
    ["CDXML export", "crates/chemsema-engine/src/cdxml/export.rs", "write_scene_object"],
  ].map(([label, file, name]) => ({ label, file, name, fn: findFunction(file, name) }));

  for (const surface of surfaces) {
    if (!surface.fn) {
      addFinding("error", "object completeness", "OBJECT-SURFACE-MISSING", surface.file, null,
        `Cannot inspect required object surface '${surface.label}' (${surface.name}).`);
    }
  }

  const matrix = {};
  for (const type of OBJECT_TYPES) {
    matrix[type] = Object.fromEntries(surfaces.map((surface) => [
      surface.label,
      Boolean(surface.fn && hasType(surface.fn.body, type)),
    ]));
  }

  const curveRequired = [
    "render",
    "selection coverage",
    "selectable",
    "select all",
    "clipboard completeness",
    "rotation",
    "transform policy",
    "CDXML export",
  ];
  for (const label of curveRequired) {
    const surface = surfaces.find((candidate) => candidate.label === label);
    if (surface?.fn && !matrix.curve[label]) {
      addFinding("error", "object completeness", "OBJECT-CURVE-PARTIAL", surface.file, surface.fn.index,
        `The imported/rendered 'curve' object has no explicit '${label}' rule.`,
        `${surface.name} does not mention "curve".`);
    }
  }

  const dragPath = "crates/chemsema-engine/src/engine/select/drag.rs";
  const dragText = read(dragPath);
  for (const functionName of ["rotated_scene_object", "resized_scene_object"]) {
    const fn = findFunction(dragPath, functionName);
    const helpers = [
      fn?.body || "",
      findFunction(dragPath, "rotate_payload_points_to_next_local")?.body || "",
      findFunction(dragPath, "resize_payload_named_points")?.body || "",
    ].join("\n");
    if (!helpers.includes("curvePoints")) {
      addFinding("error", "object completeness", "OBJECT-CURVE-GEOMETRY", dragPath, fn?.index ?? 0,
        `${functionName} does not transform the CDXML curvePoints geometry.`,
        "Curve rendering/export use curvePoints, so translation-only behavior cannot preserve rotation/resize semantics.");
    }
  }

  const formatPath = "docs/format-v0.1.md";
  const formatText = read(formatPath);
  const supportedTypeSection = formatText.match(
    /### Supported Object Types in v0\.1([\s\S]*?)(?=\n##|\n###)/,
  )?.[1] || "";
  for (const type of ["curve", "symbol"]) {
    if (!new RegExp(`-\\s+\`${type}\``).test(supportedTypeSection)) {
      const importPath = "crates/chemsema-engine/src/cdxml/import_objects.rs";
      const importIndex = read(importPath).indexOf(`"${type}".to_string()`);
      addFinding("error", "object model", "OBJECT-TYPE-DRIFT", importPath, importIndex,
        `CDXML imports a first-class '${type}' object, but CCJS v0.1 does not define that object type.`,
        type === "curve"
          ? "The native model defines curved strokes under line; import must normalize to line or the format must explicitly add curve."
          : "The object must be documented as first-class or normalized to a documented bracket/shape/line semantic.");
    }
  }
  return matrix;
}

function auditSilentDispatch() {
  const dispatches = [
    ["crates/chemsema-engine/src/render.rs", "render_scene_object"],
    ["crates/chemsema-engine/src/cdxml/export.rs", "write_scene_object"],
    ["crates/chemsema-engine/src/engine/select/render.rs", "scene_object_selection_coverage"],
    ["crates/chemsema-engine/src/engine/clipboard.rs", "visible_root_object_is_selected_for_clipboard"],
    ["crates/chemsema-engine/src/engine/select/drag.rs", "rotated_scene_object"],
    ["crates/chemsema-engine/src/engine/select/drag.rs", "object_transform_participates_in_render"],
  ];
  for (const [path, name] of dispatches) {
    const fn = findFunction(path, name);
    if (!fn) continue;
    const wildcard = /_\s*=>\s*(?:\{\s*\}|true|false|None|Default::default\(\))/m.exec(fn.body);
    if (wildcard) {
      addFinding("error", "fallback", "FALLBACK-UNKNOWN-OBJECT", path,
        fn.index + fn.body.indexOf(wildcard[0]),
        `${name} silently accepts an unknown object type.`,
        wildcard[0]);
    }
  }
}

function auditForbiddenCommands() {
  const forbidden = ["mutation", "pointer-up", "toolbar-click", "legacy-mutation"];
  for (const path of sourceFiles) {
    const text = read(path);
    for (const command of forbidden) {
      const pattern = new RegExp(`["']${command}["']`, "g");
      for (const match of text.matchAll(pattern)) {
        addFinding("error", "core rules", "CORE-FORBIDDEN-COMMAND", repoPath(path), match.index,
          `Forbidden generic command name '${command}' bypasses semantic command ownership.`,
          match[0]);
      }
    }
  }
}

function auditFallbacks() {
  const rules = [
    {
      severity: "error",
      rule: "FALLBACK-EMPTY-ASYNC-ERROR",
      pattern: /\.catch\(\(\)\s*=>\s*\{\s*\}\)/g,
      message: "An asynchronous failure is discarded without an explicit state transition or error report.",
    },
    {
      severity: "error",
      rule: "FALLBACK-EMPTY-CATCH",
      pattern: /catch\s*\{\s*\}/g,
      message: "An exception is discarded by an empty catch block.",
    },
  ];
  for (const path of sourceFiles) {
    const text = read(path);
    for (const rule of rules) {
      for (const match of text.matchAll(rule.pattern)) {
        addFinding(rule.severity, "fallback", rule.rule, repoPath(path), match.index,
          rule.message, text.slice(match.index, text.indexOf("\n", match.index)).trim());
      }
    }
  }

  const namedFallbacks = new Map();
  for (const path of sourceFiles) {
    const text = read(path);
    for (const match of text.matchAll(/\bfallback[A-Za-z0-9_]*/gi)) {
      const key = `${repoPath(path)}\0${match[0]}`;
      if (!namedFallbacks.has(key)) namedFallbacks.set(key, { path, text, match });
    }
  }
  for (const { path, text, match } of namedFallbacks.values()) {
    addFinding("review", "fallback", "FALLBACK-NAMED", repoPath(path), match.index,
      "A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.",
      text.slice(match.index, text.indexOf("\n", match.index)).trim());
  }

  const hardFallbacks = [
    ["crates/chemsema-engine/src/cdxml.rs", "fallback_cdxml_topology_positions",
      "CDXML without authoritative coordinates is silently laid out by an invented topology algorithm."],
    ["crates/chemsema-engine/src/render/labels.rs", "clip_point_out_of_box",
      "Bond retreat falls back from authoritative glyph polygons to a label rectangle."],
    ["viewer/text_metrics.js", "inferredGlyphProfile",
      "The viewer invents glyph geometry when the shared kernel profile has no character rule."],
    ["viewer/text_metrics.js", "estimatedEditorCharWidth",
      "The viewer owns a second text-width rule instead of consuming kernel layout geometry."],
  ];
  for (const [path, token, message] of hardFallbacks) {
    const index = read(path).indexOf(token);
    if (index >= 0) addFinding("error", "fallback", "FALLBACK-SEMANTIC", path, index, message, token);
  }
}

function auditViewerAuthority() {
  const patterns = [
    [/\b(valence|bondRetreat|glyphClip|doubleBondPlacement)\b/gi,
      "VIEWER-CHEMISTRY-RULE", "Viewer code appears to own chemistry or rendering-rule vocabulary."],
    [/\b(snapAngle|nearestAngle|bondAngle)\b/gi,
      "VIEWER-GEOMETRY-RULE", "Viewer code appears to calculate an editor geometry rule."],
  ];
  for (const path of sourceFiles.filter((file) => repoPath(file).startsWith("viewer/"))) {
    const text = read(path);
    for (const [pattern, rule, message] of patterns) {
      for (const match of text.matchAll(pattern)) {
        addFinding("review", "frontend authority", rule, repoPath(path), match.index, message,
          text.slice(match.index, text.indexOf("\n", match.index)).trim());
      }
    }
  }
}

function auditAsyncEventHandlers() {
  for (const path of sourceFiles.filter((file) => repoPath(file).startsWith("viewer/"))) {
    const text = read(path);
    const pattern = /addEventListener\([^,\n]+,\s*async\s*\([^)]*\)\s*=>\s*\{/g;
    for (const match of text.matchAll(pattern)) {
      const openingBrace = text.indexOf("{", match.index + match[0].length - 1);
      const body = balancedBlock(text, openingBrace);
      if (!body || /\btry\s*\{|\brunSafe\s*\(/.test(body)) continue;
      addFinding("warning", "frontend errors", "FRONTEND-UNGUARDED-ASYNC-EVENT",
        repoPath(path), match.index,
        "Async DOM event handler can reject without a user-visible error or controlled state recovery.",
        match[0]);
    }
  }
}

function auditHybridGetters() {
  const path = "viewer/engine_host.js";
  const methods = [
    ["documentColorsJson", "documentColorsJson"],
    ["documentStylePreset", "documentStylePreset"],
    ["canUndo", "canUndo"],
    ["canRedo", "canRedo"],
  ];
  for (const [name, layoutMethod] of methods) {
    const fn = findFunction(path, name);
    if (fn && !fn.body.includes(`layoutEngine?.${layoutMethod}`)) {
      addFinding("warning", "frontend state", "FRONTEND-HYBRID-STALE-GETTER", path, fn.index,
        `${name} reads only the native cache even when the local layout engine is authoritative.`,
        fn.body.split("\n").slice(0, 6).join("\n"));
    }
  }
  for (const name of [
    "hasClipboard",
    "clipboardSelectionJson",
    "clipboardDocumentJson",
    "clipboardCdxml",
  ]) {
    const fn = findFunction(path, name);
    if (fn && !fn.body.includes("awaitNativeReadBarrier")
      && !fn.body.includes("nativeBackgroundOperation")) {
      addFinding("error", "frontend state", "FRONTEND-HYBRID-READ-BARRIER", path, fn.index,
        `${name} can read native clipboard/selection state before queued local selection mutations reach native.`,
        "The full GUI copy/paste/cut sequence reproduces this race.");
    }
  }
}

function auditOperationTests() {
  const regressionFiles = readdirSync(join(rootDir, "scripts"), { withFileTypes: true })
    .filter((entry) => entry.isFile() && /(?:regression|smoke)\.mjs$/.test(entry.name))
    .map((entry) => join(rootDir, "scripts", entry.name));
  const integrationDir = join(rootDir, "crates", "chemsema-engine", "tests");
  const testTexts = [
    ...regressionFiles.map((path) => read(path)),
    ...walk(integrationDir).map((path) => read(path)),
    ...sourceFiles
      .filter((path) => extname(path) === ".rs")
      .map((path) => {
        const text = read(path);
        const marker = text.indexOf("#[cfg(test)]");
        return marker >= 0 ? text.slice(marker) : "";
      }),
  ].join("\n");
  const curveOperationTest = /fn\s+[A-Za-z0-9_]*(?:bezier_)?curve_object[A-Za-z0-9_]*(?:select|move|rotate|resize|copy|cut|delete)|fn\s+[A-Za-z0-9_]*(?:select|move|rotate|resize|copy|cut|delete)[A-Za-z0-9_]*(?:bezier_)?curve_object/i;
  if (!curveOperationTest.test(testTexts)) {
    addFinding("warning", "object completeness", "OBJECT-OPERATION-TEST-GAP",
      "scripts", null,
      "No regression test exercises select/move/rotate/resize/copy/cut/delete for first-class CDXML curve objects.");
  }

  const regressionText = regressionFiles.map((path) => read(path)).join("\n");
  const frontendScenarios = [
    ["FRONTEND-IMAGE-IMPORT-TEST-GAP", /insertDroppedImage|openImageFilePicker|paste image|verifyImageDropAndPaste/i,
      "No browser regression covers image insertion by drop, paste, and blank-canvas context menu."],
    ["FRONTEND-CROSS-TAB-CLIPBOARD-TEST-GAP", /cross.?tab|another tab|between tabs/i,
      "No browser regression covers structured copy/paste between tabs or browser/desktop surfaces."],
    ["FRONTEND-DETACHED-TAB-TEST-GAP", /detach.*tab|tab.*detach|new window.*tab/i,
      "No end-to-end regression covers dragging a desktop tab into a new window."],
    ["FRONTEND-UI-ACTION-RUNNER-TEST-GAP", /ui-action-runner-regression|createUiActionRunner/i,
      "No regression verifies UI action error reporting, recovery, cancellation, and event-boundary settlement."],
  ];
  for (const [rule, pattern, message] of frontendScenarios) {
    if (!pattern.test(regressionText)) {
      addFinding("warning", "frontend tests", rule, "scripts", null, message);
    }
  }
}

function architectureReviewMatches(path, name, lines) {
  const key = `${repoPath(path)}:${name}`;
  const review = architectureReviewLedger.entries?.[key];
  if (!review || review.lines !== lines || !review.owner || !review.reason) {
    return false;
  }
  verifiedArchitectureReviews.push({
    key,
    lines,
    owner: review.owner,
    reason: review.reason,
  });
  return true;
}

function auditArchitecture() {
  const functions = [];
  for (const path of sourceFiles) {
    const text = read(path);
    const extension = extname(path);
    const pattern = extension === ".rs"
      ? /(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*?\)\s*(?:->[^{]+)?\{/g
      : /(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*\([^)]*\)\s*\{|^\s*(?:async\s+)?([A-Za-z_$][A-Za-z0-9_$]*)\s*\([^)]*\)\s*\{/gm;
    for (const match of text.matchAll(pattern)) {
      const name = match[1] || match[2];
      if (["if", "for", "while", "switch", "catch", "with"].includes(name)) continue;
      const brace = text.indexOf("{", match.index + match[0].length - 1);
      const body = balancedBlock(text, brace);
      if (!body) continue;
      // JavaScript host factories deliberately own named nested rules. Counting every
      // nested rule again as part of the factory reports lexical containment as one
      // giant function even though each rule is audited independently below.
      const ownedBody = extension === ".rs" ? body : withoutNestedNamedFunctions(body);
      const lines = ownedBody.split("\n").filter((line) => line.trim()).length;
      functions.push({ path: repoPath(path), name, index: match.index, lines, body });
      if (lines >= 350) {
        addFinding("error", "architecture", "ARCH-LARGE-FUNCTION", repoPath(path), match.index,
          `${name} is ${lines} lines; split ownership and behavior into named rules.`);
      } else if (lines >= 200 && !architectureReviewMatches(path, name, lines)) {
        addFinding("warning", "architecture", "ARCH-LARGE-FUNCTION", repoPath(path), match.index,
          `${name} is ${lines} lines and needs a focused decomposition review.`);
      }
    }
    const lineCount = productionLogicalLineCount(text, extension);
    if (lineCount >= 4000) {
      addFinding("error", "architecture", "ARCH-LARGE-FILE", repoPath(path), 0,
        `Source file has ${lineCount} production logic lines and mixes too many responsibilities.`);
    } else if (
      lineCount >= 2000
      && !architectureReviewMatches(path, "$file", lineCount)
    ) {
      addFinding("warning", "architecture", "ARCH-LARGE-FILE", repoPath(path), 0,
        `Source file has ${lineCount} production logic lines and needs an ownership review.`);
    }
  }
  const exactBodies = new Map();
  for (const fn of functions.filter((candidate) => candidate.lines >= 12)) {
    const normalized = fn.body
      .replace(/\/\/.*$/gm, "")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/\s+/g, " ")
      .trim();
    if (normalized.length < 240) continue;
    const entries = exactBodies.get(normalized) || [];
    entries.push(fn);
    exactBodies.set(normalized, entries);
  }
  for (const entries of exactBodies.values()) {
    const files = new Set(entries.map((entry) => entry.path));
    if (files.size < 2) continue;
    const first = entries[0];
    addFinding("warning", "architecture", "ARCH-EXACT-DUPLICATE", first.path, first.index,
      `Exact function body is duplicated across ${files.size} files.`,
      entries.map((entry) => `${entry.path}:${entry.name}`).join(", "));
  }
  const verifiedKeys = new Set(verifiedArchitectureReviews.map((review) => review.key));
  for (const key of Object.keys(architectureReviewLedger.entries || {})) {
    if (!verifiedKeys.has(key)) {
      addFinding(
        "warning",
        "architecture review ledger",
        "ARCH-REVIEW-STALE",
        "docs/architecture-review-ledger.json",
        null,
        `Reviewed architecture fingerprint no longer matches source: ${key}`,
      );
    }
  }
  return functions;
}

function productionLogicalLineCount(text, extension) {
  let productionText = text;
  if (extension === ".rs") {
    const testModule = productionText.search(/^#\[cfg\(test\)\]\s*\nmod\s+tests\s*\{/m);
    if (testModule >= 0) {
      productionText = productionText.slice(0, testModule);
    }
  }
  return productionText
    .split("\n")
    .filter((line) => {
      const trimmed = line.trim();
      return trimmed
        && trimmed !== "{"
        && trimmed !== "}"
        && !trimmed.startsWith("//")
        && !trimmed.startsWith("#[")
        && !(extension === ".rs" && trimmed.startsWith("use "));
    })
    .length;
}

function withoutNestedNamedFunctions(body) {
  const chars = [...body];
  const pattern = /(?:async\s+)?function\s+[A-Za-z_$][A-Za-z0-9_$]*\s*\([^)]*\)\s*\{/g;
  pattern.lastIndex = 1;
  for (const match of body.matchAll(pattern)) {
    const brace = body.indexOf("{", match.index + match[0].length - 1);
    const nestedBody = balancedBlock(body, brace);
    if (!nestedBody) continue;
    const end = brace + nestedBody.length;
    for (let index = match.index; index < end; index += 1) {
      if (chars[index] !== "\n") chars[index] = " ";
    }
  }
  return chars.join("");
}

function summarize(findingsList) {
  const counts = { error: 0, warning: 0, review: 0 };
  for (const finding of findingsList) counts[finding.severity] += 1;
  return counts;
}

function markdown(report) {
  const lines = [
    "# ChemSema 核心契约自动审查",
    "",
    `生成时间：\`${report.generatedAt}\``,
    "",
    "本报告只把可机械证明的问题列为 error；需要结合设计文档判断的候选项列为 review。",
    "文档规定的默认值不是 fallback；未知类型静默跳过、失败后改走另一套语义、吞异常才是禁止的 fallback。",
    "",
    "## 摘要",
    "",
    `- Error: ${report.summary.error}`,
    `- Warning: ${report.summary.warning}`,
    `- Review: ${report.summary.review}`,
    `- Verified architecture reviews: ${report.architecture.verifiedReviews}`,
    "",
    "## 对象能力矩阵",
    "",
    `| Object | ${report.surfaces.join(" | ")} |`,
    `| --- | ${report.surfaces.map(() => "---").join(" | ")} |`,
  ];
  for (const [type, capabilities] of Object.entries(report.objectMatrix)) {
    lines.push(`| ${type} | ${report.surfaces.map((surface) => capabilities[surface] ? "explicit" : "—").join(" | ")} |`);
  }
  for (const severity of ["error", "warning", "review"]) {
    const selected = report.findings.filter((finding) => finding.severity === severity);
    lines.push("", `## ${severity.toUpperCase()} (${selected.length})`, "");
    for (const finding of selected) {
      lines.push(`- \`${finding.rule}\` ${finding.file}${finding.line ? `:${finding.line}` : ""} — ${finding.message}`);
      if (finding.evidence) lines.push(`  - Evidence: \`${finding.evidence.replace(/\s+/g, " ").slice(0, 240)}\``);
    }
  }
  while (lines.at(-1) === "") lines.pop();
  return `${lines.join("\n")}\n`;
}

const objectMatrix = auditObjectCapabilities();
auditSilentDispatch();
auditForbiddenCommands();
auditFallbacks();
auditViewerAuthority();
auditAsyncEventHandlers();
auditHybridGetters();
auditOperationTests();
const functions = auditArchitecture();
findings.sort((left, right) => (
  ["error", "warning", "review"].indexOf(left.severity)
  - ["error", "warning", "review"].indexOf(right.severity)
  || left.file.localeCompare(right.file)
  || (left.line || 0) - (right.line || 0)
));

const report = {
  generatedAt: new Date().toISOString(),
  summary: summarize(findings),
  surfaces: Object.keys(objectMatrix[OBJECT_TYPES[0]]),
  objectMatrix,
  architecture: {
    sourceFiles: sourceFiles.length,
    functions: functions.length,
    verifiedReviews: verifiedArchitectureReviews.length,
  },
  findings,
};

if (reportPath) {
  writeFileSync(reportPath, markdown(report), "utf8");
}
if (jsonOnly) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(`[core-contract-audit] files=${report.architecture.sourceFiles} functions=${report.architecture.functions}`);
  console.log(`[core-contract-audit] errors=${report.summary.error} warnings=${report.summary.warning} review=${report.summary.review}`);
  for (const finding of findings.filter((item) => item.severity !== "review")) {
    console.log(`${finding.severity.toUpperCase()} ${finding.rule} ${finding.file}:${finding.line || "-"} ${finding.message}`);
  }
  if (reportPath) console.log(`[core-contract-audit] report=${repoPath(reportPath)}`);
}
if (failOnError && report.summary.error > 0) process.exit(1);
