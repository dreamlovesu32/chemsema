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
assert.match(html, /cc-bond-icon-fallback/, "bond toolbar should use explicit fallback icons before kernel icons are ready");

console.log("[toolbar-regression] ok");
