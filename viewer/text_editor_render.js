import { textLength } from "./text_metrics.js";
import {
  displayLabelFontFamily,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";

export function editorSourceRunsFromSession(session, root, options) {
  const {
    defaultFontFamily,
    defaultFontSize,
    defaultTextColor,
    normalizeRuns,
    baseStyle,
  } = options;
  const fallbackStyle = baseStyle(root);
  const rawRuns = Array.isArray(session.sourceRuns) && session.sourceRuns.length
    ? session.sourceRuns.map((run) => ({ ...run }))
    : session.text
      ? [{
        text: String(session.text || ""),
        fontFamily: session.fontFamily || fallbackStyle.fontFamily || defaultFontFamily,
        fontSize: Number(session.fontSize || root.dataset.baseFontSize || defaultFontSize),
        fill: session.fill || root.style.color || defaultTextColor,
        fontWeight: 400,
        fontStyle: "normal",
        underline: false,
        script: session.defaultChemical ? "chemical" : "normal",
      }]
      : [];
  return normalizeRuns(rawRuns, fallbackStyle);
}

export function previewTextRunsFromKernel(sourceRuns, root, options) {
  const {
    engine,
    parseJson,
    baseStyle,
    normalizeRuns,
    runsPlainText,
    defaultTextAlign,
    defaultLineHeight,
    target,
  } = options;
  if (!engine?.previewTextRuns || !root) {
    return null;
  }
  const fallbackStyle = baseStyle(root);
  const baseLineHeight = Number.parseFloat(root.dataset.baseLineHeight || `${defaultLineHeight(fallbackStyle.fontSize)}`)
    || defaultLineHeight(fallbackStyle.fontSize);
  const preview = parseJson(engine.previewTextRuns(JSON.stringify({
    target: target || {
      kind: "text-object",
      objectId: null,
      x: 0,
      y: 0,
    },
    text: runsPlainText(sourceRuns || []),
    sourceRuns: sourceRuns || [],
    fontFamily: fallbackStyle.fontFamily,
    fontSize: fallbackStyle.fontSize,
    fill: fallbackStyle.fill,
    align: root.style.textAlign || defaultTextAlign,
    lineHeight: baseLineHeight,
    defaultChemical: root.dataset.defaultChemical === "true",
  })), null);
  if (!preview) {
    return null;
  }
  return {
    sourceRuns: normalizeRuns(preview.sourceRuns || sourceRuns || [], fallbackStyle),
    displayRuns: normalizeRuns(preview.displayRuns || [], fallbackStyle),
  };
}

export function displayRunsForEditor(sourceRuns, root, options) {
  const preview = previewTextRunsFromKernel(sourceRuns, root, options);
  if (preview?.displayRuns) {
    return preview.displayRuns;
  }
  return options.normalizeRuns(sourceRuns || [], options.baseStyle(root));
}

function splitRunsIntoLines(runs) {
  const lines = [[]];
  for (const run of runs || []) {
    const text = String(run?.text || "");
    const parts = text.split("\n");
    for (let index = 0; index < parts.length; index += 1) {
      if (parts[index]) {
        lines[lines.length - 1].push({
          ...run,
          text: parts[index],
        });
      }
      if (index < parts.length - 1) {
        lines.push([]);
      }
    }
  }
  return lines;
}

function textAnchorForAlign(align) {
  if (align === "right") {
    return "end";
  }
  if (align === "center") {
    return "middle";
  }
  return "start";
}

function anchorXForAlign(align, width) {
  if (align === "right") {
    return width;
  }
  if (align === "center") {
    return width * 0.5;
  }
  return 0;
}

function normalizedSelectionRange(selectionOffsets) {
  if (!selectionOffsets) {
    return null;
  }
  const start = Number(selectionOffsets.start ?? Math.min(selectionOffsets.anchor, selectionOffsets.focus));
  const end = Number(selectionOffsets.end ?? Math.max(selectionOffsets.anchor, selectionOffsets.focus));
  if (!Number.isFinite(start) || !Number.isFinite(end) || start === end) {
    return null;
  }
  return {
    start: Math.min(start, end),
    end: Math.max(start, end),
  };
}

export function fillTextEditorContent(root, session, selectionOffsets, options) {
  const {
    resolveDisplayRuns,
    defaultLineHeight,
    scriptScale,
    scriptShiftEm,
  } = options;
  root.innerHTML = "";
  const runs = resolveDisplayRuns(session);
  const align = root.style.textAlign || session.align || "left";
  const baseFontSize = Number.parseFloat(root.dataset.baseFontSize || `${session.fontSize || 10}`) || 10;
  const fontSize = Number.parseFloat(root.style.fontSize || `${baseFontSize}`) || baseFontSize;
  const lineHeight = Number.parseFloat(root.style.lineHeight || `${defaultLineHeight(fontSize)}`)
    || defaultLineHeight(fontSize);
  const box = Array.isArray(session?.boxValue) ? session.boxValue : null;
  const initialWidth = Math.max(
    8,
    Number(root.dataset.renderWidth || 0)
      || (box
        ? Math.max(
          0,
          Number(box[2] || 0) - Number(box[0] || 0),
          Number(box[2] || 0),
        )
        : 0),
  );
  const lines = splitRunsIntoLines(runs);
  const selectionRange = normalizedSelectionRange(selectionOffsets);
  let textOffset = 0;
  const svg = makeSvgNode("svg", {
    class: "text-editor-svg",
    width: initialWidth,
    height: Math.max(lineHeight, lineHeight * Math.max(1, lines.length)),
    viewBox: `0 0 ${initialWidth} ${Math.max(lineHeight, lineHeight * Math.max(1, lines.length))}`,
    "data-editor-text-svg": "true",
  });
  const content = makeSvgNode("g", {
    "data-editor-text-content": "true",
  });
  const anchor = textAnchorForAlign(align);
  const x = anchorXForAlign(align, initialWidth);
  const baseline = fontSize * 0.82;

  for (let lineIndex = 0; lineIndex < Math.max(1, lines.length); lineIndex += 1) {
    const lineRuns = lines[lineIndex] || [];
    const textNode = makeSvgNode("text", {
      x,
      y: baseline + lineIndex * lineHeight,
      "data-editor-text-line": String(lineIndex),
      class: "chem-text",
      "font-size": fontSize,
      "dominant-baseline": "alphabetic",
      "text-anchor": anchor,
      fill: session.fill ? normalizeDisplayColor(session.fill) : undefined,
      "font-family": session.fontFamily ? displayLabelFontFamily(session.fontFamily) : undefined,
    });
    for (const run of lineRuns) {
      const runText = String(run.text || "");
      const runStart = textOffset;
      const runEnd = runStart + textLength(runText);
      const isSelected = selectionRange && runStart < selectionRange.end && runEnd > selectionRange.start;
      const runFontSize = Number(run.fontSize || fontSize);
      const isSub = isSubscriptRun(run);
      const isSuper = isSuperscriptRun(run);
      const scale = isSub ? scriptScale("subscript") : isSuper ? scriptScale("superscript") : 1;
      const tspan = makeSvgNode("tspan", {
        class: isSelected ? "text-editor-run is-selected" : "text-editor-run",
        "data-script": run.script || undefined,
        fill: run.fill ? normalizeDisplayColor(run.fill) : undefined,
        "font-size": (isSub || isSuper) ? Math.max(7, runFontSize * scale) : runFontSize,
        "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
        "font-weight": fontWeightForRun(run),
        "font-style": fontStyleForRun(run),
        "text-decoration": run.underline ? "underline" : undefined,
        "baseline-shift": isSub
          ? `-${scriptShiftEm("subscript")}em`
          : isSuper
            ? `${scriptShiftEm("superscript")}em`
            : undefined,
        dx: isSuper ? "-0.02em" : undefined,
      });
      tspan.textContent = runText;
      textNode.appendChild(tspan);
      textOffset = runEnd;
    }
    if (lineIndex < lines.length - 1) {
      textOffset += 1;
    }
    if (!lineRuns.length) {
      textNode.textContent = "\u00A0";
      textNode.setAttribute("fill", "transparent");
    }
    content.appendChild(textNode);
  }

  if (!runs.length && !textLength(session.text || "")) {
    const placeholder = makeSvgNode("text", {
      x: 0,
      y: baseline,
      "font-size": fontSize,
      "dominant-baseline": "alphabetic",
      fill: "transparent",
    });
    placeholder.textContent = "\u00A0";
    content.appendChild(placeholder);
  }

  svg.appendChild(content);
  root.appendChild(svg);
}
