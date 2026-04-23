const SCRIPT_NORMAL = 0;
const SCRIPT_SUBSCRIPT = 1;
const SCRIPT_SUPERSCRIPT = 2;

export const LABEL_ALIGN = {
  right: 0,
  left: 1,
  above: 2,
  below: 3,
};

const DEFAULT_ANCHOR_GLYPH_INDEX = 0xffffffff;
const GLYPH_INPUT_BYTES = 8;
const LAYOUT_CONFIG_BYTES = 24;
const GLYPH_PLACEMENT_BYTES = 80;

let kernelPromise = null;

function shapeKindName(shapeKind) {
  switch (shapeKind) {
    case 2:
      return "rect-cut-top-right";
    case 3:
      return "rect-cut-bottom-right";
    case 4:
      return "rect-cut-top-left";
    case 5:
      return "rect-cut-bottom-left";
    case 0:
    default:
      return "rect";
  }
}

function asScriptKind(face) {
  const value = Number(face) || 0;
  if ((value & 32) !== 0 && (value & 64) === 0) {
    return SCRIPT_SUBSCRIPT;
  }
  if ((value & 64) !== 0 && (value & 32) === 0) {
    return SCRIPT_SUPERSCRIPT;
  }
  return SCRIPT_NORMAL;
}

function writeLayoutConfig(module, ptr, fontSize) {
  const values = new Float32Array(module.HEAPU8.buffer, ptr, 6);
  values[0] = Number(fontSize) || 11;
  values[1] = 0.0;
  values[2] = 0.78;
  values[3] = 0.78;
  values[4] = 0.30;
  values[5] = 0.28;
}

function writeGlyphInputs(module, ptr, glyphs) {
  const view = new DataView(module.HEAPU8.buffer);
  glyphs.forEach((glyph, index) => {
    const offset = ptr + index * GLYPH_INPUT_BYTES;
    view.setUint32(offset, glyph.codepoint >>> 0, true);
    view.setInt32(offset + 4, glyph.scriptKind | 0, true);
  });
}

function readPlacement(module, ptr, index) {
  const view = new DataView(module.HEAPU8.buffer);
  const offset = ptr + index * GLYPH_PLACEMENT_BYTES;
  const codepoint = view.getUint32(offset, true);
  const shapeKind = view.getInt32(offset + 60, true);
  return {
    codepoint,
    char: String.fromCodePoint(codepoint),
    scriptKind: view.getInt32(offset + 4, true),
    visible: view.getInt32(offset + 8, true) !== 0,
    fontSize: view.getFloat32(offset + 12, true),
    originX: view.getFloat32(offset + 16, true),
    baselineY: view.getFloat32(offset + 20, true),
    advance: view.getFloat32(offset + 24, true),
    inkBox: {
      x1: view.getFloat32(offset + 28, true),
      y1: view.getFloat32(offset + 32, true),
      x2: view.getFloat32(offset + 36, true),
      y2: view.getFloat32(offset + 40, true),
    },
    backgroundBox: {
      x1: view.getFloat32(offset + 44, true),
      y1: view.getFloat32(offset + 48, true),
      x2: view.getFloat32(offset + 52, true),
      y2: view.getFloat32(offset + 56, true),
    },
    shape: shapeKind === 1
      ? {
          kind: "ellipse",
          cx: view.getFloat32(offset + 64, true),
          cy: view.getFloat32(offset + 68, true),
          rx: view.getFloat32(offset + 72, true),
          ry: view.getFloat32(offset + 76, true),
        }
      : {
          kind: shapeKindName(shapeKind),
          x1: view.getFloat32(offset + 44, true),
          y1: view.getFloat32(offset + 48, true),
          x2: view.getFloat32(offset + 52, true),
          y2: view.getFloat32(offset + 56, true),
        },
  };
}

function translateShape(shape, dx, dy) {
  if (shape.kind === "ellipse") {
    return {
      ...shape,
      cx: shape.cx + dx,
      cy: shape.cy + dy,
    };
  }
  return {
    ...shape,
    x1: shape.x1 + dx,
    y1: shape.y1 + dy,
    x2: shape.x2 + dx,
    y2: shape.y2 + dy,
  };
}

function translateBox(box, dx, dy) {
  return {
    x1: box.x1 + dx,
    y1: box.y1 + dy,
    x2: box.x2 + dx,
    y2: box.y2 + dy,
  };
}

function translatePlacement(placement, dx, dy) {
  return {
    ...placement,
    originX: placement.originX + dx,
    baselineY: placement.baselineY + dy,
    inkBox: translateBox(placement.inkBox, dx, dy),
    backgroundBox: translateBox(placement.backgroundBox, dx, dy),
    shape: translateShape(placement.shape, dx, dy),
  };
}

function resolveAnchorIndex(placements, requestedIndex) {
  if (
    Number.isInteger(requestedIndex)
    && requestedIndex >= 0
    && requestedIndex < placements.length
    && placements[requestedIndex].visible
  ) {
    return requestedIndex;
  }
  return placements.findIndex((placement) => placement.visible);
}

function standardAnchorY(placement, fontSize) {
  return placement.baselineY - 0.365 * fontSize;
}

function locateAnchor(placements, requestedIndex, fontSize) {
  const glyphIndex = resolveAnchorIndex(placements, requestedIndex);
  if (glyphIndex < 0) {
    return null;
  }
  const placement = placements[glyphIndex];
  return {
    glyphIndex,
    x: (placement.backgroundBox.x1 + placement.backgroundBox.x2) / 2,
    y: standardAnchorY(placement, fontSize),
  };
}

function normalizeGlyphs(glyphs) {
  return glyphs
    .map((glyph) => ({
      codepoint: Number(glyph.codepoint) >>> 0,
      scriptKind: glyph.scriptKind | 0,
      char: glyph.char || String.fromCodePoint(Number(glyph.codepoint) >>> 0),
      fill: glyph.fill || "#111111",
      fontFamily: glyph.fontFamily || "Arial",
    }))
    .filter((glyph) => glyph.codepoint > 0);
}

function callLayout(module, glyphs, fontSize, anchorGlyphIndex, align) {
  const count = glyphs.length;
  if (!count) {
    return [];
  }

  const inputPtr = module._malloc(count * GLYPH_INPUT_BYTES);
  const configPtr = module._malloc(LAYOUT_CONFIG_BYTES);
  const outputPtr = module._malloc(count * GLYPH_PLACEMENT_BYTES);
  try {
    writeGlyphInputs(module, inputPtr, glyphs);
    writeLayoutConfig(module, configPtr, fontSize);
    const written = module._chemcore_layout_glyph_run_aligned(
      inputPtr,
      count,
      configPtr,
      0,
      0,
      Number.isInteger(anchorGlyphIndex) ? anchorGlyphIndex : DEFAULT_ANCHOR_GLYPH_INDEX,
      align | 0,
      outputPtr,
      count,
    );
    const placementCount = Math.min(written >>> 0, count);
    const placements = [];
    for (let index = 0; index < placementCount; index += 1) {
      placements.push(readPlacement(module, outputPtr, index));
    }
    return placements;
  } finally {
    module._free(outputPtr);
    module._free(configPtr);
    module._free(inputPtr);
  }
}

export async function initializeGlyphKernel() {
  if (!kernelPromise) {
    kernelPromise = import("./chemcore_glyph_kernel.js").then(async ({ default: createModule }) => {
      const module = await createModule({
        locateFile: (path) => new URL(path, import.meta.url).href,
      });
      return {
        layoutAtAnchor({ glyphs, fontSize, anchorPoint, anchorGlyphIndex = null, align = LABEL_ALIGN.right }) {
          const normalized = normalizeGlyphs(glyphs);
          const placements = callLayout(module, normalized, fontSize, anchorGlyphIndex, align);
          const anchor = locateAnchor(placements, anchorGlyphIndex, fontSize);
          if (!anchor || !anchorPoint) {
            return { placements, anchor };
          }

          const dx = anchorPoint.x - anchor.x;
          const dy = anchorPoint.y - anchor.y;
          const translated = placements.map((placement) => translatePlacement(placement, dx, dy));
          const translatedAnchor = {
            ...anchor,
            x: anchor.x + dx,
            y: anchor.y + dy,
          };
          return {
            placements: translated,
            anchor: translatedAnchor,
          };
        },
        scriptKindForFace: asScriptKind,
      };
    });
  }
  return kernelPromise;
}
