function normalizeSharedGlyphProfile(profile) {
  return {
    shape: String(profile?.shape || "rect"),
    advanceEm: Number(profile?.advanceEm || 0),
    inkLeftEm: Number(profile?.inkLeftEm || 0),
    inkTopEm: Number(profile?.inkTopEm || 0),
    inkRightEm: Number(profile?.inkRightEm || 0),
    inkBottomEm: Number(profile?.inkBottomEm || 0),
    padXEm: Number(profile?.padXEm || 0),
    padYEm: Number(profile?.padYEm || 0),
    visible: profile?.visible !== false,
  };
}

const DEFAULT_GLYPH_LAYOUT = {
  trackingEm: 0,
  subscriptScale: 0.75,
  superscriptScale: 0.75,
  subscriptShiftDownEm: 0.22,
  superscriptShiftUpEm: 0.392,
};
const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM = 0.215;

function editorGlyphLayoutConfig(sharedGlyphProfiles) {
  if (!sharedGlyphProfiles) {
    return DEFAULT_GLYPH_LAYOUT;
  }
  return sharedGlyphProfiles.layout;
}

export function textCodePoints(text) {
  return Array.from(String(text || ""));
}

export function textLength(text) {
  return textCodePoints(text).length;
}

export function sliceTextByOffset(text, start = 0, end = undefined) {
  return textCodePoints(text).slice(start, end).join("");
}

export function normalizeSharedGlyphProfiles(manifest) {
  const specials = Object.create(null);
  for (const [key, value] of Object.entries(manifest?.specials || {})) {
    if (Array.from(key).length !== 1) {
      throw new Error(`Glyph profile key must be exactly one character: ${JSON.stringify(key)}`);
    }
    specials[key] = normalizeSharedGlyphProfile(value);
  }
  return {
    layout: {
      trackingEm: Number(manifest?.layout?.trackingEm || 0),
      subscriptScale: Number(manifest?.layout?.subscriptScale || 0.75),
      superscriptScale: Number(manifest?.layout?.superscriptScale || 0.75),
      subscriptShiftDownEm: Number(manifest?.layout?.subscriptShiftDownEm || 0.22),
      superscriptShiftUpEm: Number(manifest?.layout?.superscriptShiftUpEm || 0.392),
    },
    defaults: {
      upper: normalizeSharedGlyphProfile(manifest?.defaults?.upper),
      lower: normalizeSharedGlyphProfile(manifest?.defaults?.lower),
      digit: normalizeSharedGlyphProfile(manifest?.defaults?.digit),
      punctuation: normalizeSharedGlyphProfile(manifest?.defaults?.punctuation),
    },
    specials,
  };
}

export function editorScriptScale(sharedGlyphProfiles, script) {
  const layout = editorGlyphLayoutConfig(sharedGlyphProfiles);
  if (script === "subscript") {
    return layout.subscriptScale;
  }
  if (script === "superscript") {
    return layout.superscriptScale;
  }
  return 1;
}

export function editorScriptBaselineShift(sharedGlyphProfiles, baseFontSize, script) {
  return editorScriptBaselineShiftEm(sharedGlyphProfiles, script) * baseFontSize;
}

export function editorScriptBaselineShiftEm(sharedGlyphProfiles, script, fontWeight = 400) {
  const layout = editorGlyphLayoutConfig(sharedGlyphProfiles);
  if (script === "subscript") {
    return Number(fontWeight) >= 600
      ? CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM
      : layout.subscriptShiftDownEm;
  }
  if (script === "superscript") {
    return -layout.superscriptShiftUpEm;
  }
  return 0;
}

export function editorSvgScriptBaselineShift(sharedGlyphProfiles, runFontSize, script, fontWeight = 400) {
  return -editorScriptBaselineShiftEm(sharedGlyphProfiles, script, fontWeight) * runFontSize;
}
