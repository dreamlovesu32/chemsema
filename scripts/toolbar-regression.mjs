import assert from "node:assert/strict";
import { renderSecondaryToolbarHtml } from "../viewer/toolbar.js";

const baseEditorState = {
  activeTool: "bond",
  bondType: "single",
  bondIconSvgs: {},
  arrowIconSvgs: {},
  shapeIconSvgs: {},
  symbolIconSvgs: {},
  orbitalIconSvgs: {},
  chainIconSvg: "",
  colorPalette: null,
  documentColors: [],
  elementPalette: null,
};

const html = renderSecondaryToolbarHtml(baseEditorState);
const bondButtons = [...html.matchAll(/data-secondary-value="bond-[^"]+"/g)];
const svgCount = [...html.matchAll(/<svg\b/g)];

assert.equal(bondButtons.length, 11, "bond toolbar should render every bond tool");
assert.equal(svgCount.length, 11, "bond toolbar buttons should not render blank icons when engine icons are unavailable");
assert.match(
  html,
  /cc-bond-icon-static/,
  "bond toolbar should use its declared static icons before kernel icons are ready",
);

const textHtml = renderSecondaryToolbarHtml({
  ...baseEditorState,
  activeTool: "text",
  textFontFamily: "Aptos Display",
  textFontSize: 16,
  textColor: "#000000",
  textAlign: "left",
  textBold: false,
  textItalic: false,
  textUnderline: false,
  textOutline: false,
  textShadow: false,
  textScript: "normal",
  textIconSvgs: {},
});
assert.match(textHtml, /<input[^>]+data-text-control="font"[^>]+list="text-font-options"/, "font family control should accept arbitrary names");
assert.match(textHtml, /value="Aptos Display"/, "font family control should retain an imported custom family");
assert.match(textHtml, /data-secondary-value="text-outline"/, "text toolbar should expose outline");
assert.match(textHtml, /data-secondary-value="text-shadow"/, "text toolbar should expose shadow");

console.log("[toolbar-regression] ok");
