# ChemSema 核心契约自动审查

生成时间：`2026-07-23T07:16:02.720Z`

本报告只把可机械证明的问题列为 error；需要结合设计文档判断的候选项列为 review。
文档规定的默认值不是 fallback；未知类型静默跳过、失败后改走另一套语义、吞异常才是禁止的 fallback。

## 摘要

- Error: 12
- Warning: 68
- Review: 55

## 对象能力矩阵

| Object | render | selection coverage | selectable | select all | clipboard completeness | rotation | transform policy | CDXML export |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| molecule | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| text | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| line | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| curve | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| bracket | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| symbol | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| shape | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| image | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |
| group | explicit | explicit | explicit | explicit | explicit | explicit | explicit | explicit |

## ERROR (12)

- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine.rs:1 — Source file has 4920 lines and mixes too many responsibilities.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine.rs:2066 — execute_command is 583 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/app_window_lifecycle.js:8 — createAppWindowLifecycleHost is 461 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FILE` viewer/app.js:1 — Source file has 4074 lines and mixes too many responsibilities.
- `ARCH-LARGE-FUNCTION` viewer/browser_document_tabs.js:19 — createBrowserDocumentTabs is 557 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/document_flow.js:24 — createDocumentFlow is 702 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/editor_context_menu.js:1 — createCanvasContextMenuHost is 588 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/editor_overlay.js:19 — createEditorOverlayRenderer is 769 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/editor_pointer_controller.js:15 — createEditorPointerController is 1582 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/editor_pointer_controller.js:1161 — handleEditorPointerUp is 380 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/editor_viewport_host.js:22 — createEditorViewportHost is 739 lines; split ownership and behavior into named rules.
- `ARCH-LARGE-FUNCTION` viewer/text_editor_controller.js:3 — createTextEditorController is 678 lines; split ownership and behavior into named rules.

## WARNING (68)

- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/agent/capture.rs:3 — capture_command is 320 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/agent/context.rs:3 — context_command is 281 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-cli/src/main.rs:1 — Source file has 3749 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/main.rs:1467 — run_command_script is 204 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdx.rs:1 — Source file has 3389 lines and needs an ownership review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/cdx.rs:221 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/cdx.rs:decode_hex_bytes, crates/chemsema-engine/src/cdxml/import_objects.rs:decode_cdxml_hex_bytes`
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml.rs:1 — Source file has 2929 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml.rs:264 — parse_cdxml_document is 226 lines and needs a focused decomposition review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/cdxml.rs:1759 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/cdxml.rs:fragment_connected_components, crates/chemsema-engine/src/document.rs:molecule_fragment_connected_components`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/cdxml.rs:1829 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/cdxml.rs:component_local_bounds, crates/chemsema-engine/src/document.rs:molecule_component_bounds`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/cdxml.rs:1880 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/cdxml.rs:translate_node_label_geometry, crates/chemsema-engine/src/document.rs:translate_node_label_geometry`
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml/export.rs:1 — Source file has 2975 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/export.rs:1108 — write_line_object is 219 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/export.rs:1411 — write_shape_object is 260 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml/import_objects.rs:1 — Source file has 2781 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/import_objects.rs:376 — append_line_objects is 214 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/import_objects.rs:1439 — append_bracket_objects is 290 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/document.rs:1 — Source file has 3140 lines and needs an ownership review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/editing/arrows.rs:265 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/editing/arrows.rs:point_at_distance_from_start, crates/chemsema-engine/src/render_objects/arrows.rs:point_at_distance_from_start`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/editing/geometry.rs:154 — Exact function body is duplicated across 3 files.
  - Evidence: `crates/chemsema-engine/src/editing/geometry.rs:polygon_bounds, crates/chemsema-engine/src/engine/text_edit/geometry.rs:polygon_bounds, crates/chemsema-engine/src/engine/text_edit/labels.rs:label_polygon_bounds`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/editing/geometry.rs:171 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/editing/geometry.rs:polygon_anchor_point, crates/chemsema-engine/src/engine/text_edit/geometry.rs:polygon_anchor_point`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/editing/geometry.rs:188 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/editing/geometry.rs:label_glyph_anchor_point, crates/chemsema-engine/src/engine/text_edit/geometry.rs:label_editor_anchor_point`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/engine.rs:119 — Exact function body is duplicated across 3 files.
  - Evidence: `crates/chemsema-engine/src/engine.rs:render_primitive_role, crates/chemsema-engine/src/render.rs:render_primitive_role, crates/chemsema-engine/src/render_svg.rs:render_primitive_role`
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/chemistry.rs:16 — insert_smiles_untracked is 207 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/chemistry.rs:298 — chemical_molecule_for_targets is 227 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine/context_menu.rs:1 — Source file has 2023 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/context_menu.rs:141 — context_menu_items is 214 lines and needs a focused decomposition review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/engine/context_menu.rs:772 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/engine/context_menu.rs:selected_bonds, crates/chemsema-engine/src/engine/presets.rs:selected_object_settings_bonds`
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine/context_styles.rs:1 — Source file has 2213 lines and needs an ownership review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/engine/links.rs:323 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/engine/links.rs:set_meta_object_field, crates/chemsema-engine/src/repeating_units.rs:set_meta_object_field`
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine/select.rs:1 — Source file has 2544 lines and needs an ownership review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/engine/select/drag.rs:626 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/engine/select/drag.rs:selection_resize_handle_point, crates/chemsema-engine/src/engine/select/geometry.rs:selection_resize_handle_center`
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/engine/select/geometry.rs:230 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/engine/select/geometry.rs:point_in_polygon, crates/chemsema-engine/src/engine.rs:point_in_polygon`
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine/text_edit/labels.rs:1 — Source file has 2522 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/legacy_mol.rs:40 — parse_molblock is 254 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_objects.rs:283 — render_fragment_atom_properties is 243 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/render_objects/arrows.rs:1 — Source file has 2051 lines and needs an ownership review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/render_objects/graphics.rs:1 — Source file has 2747 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_objects/graphics.rs:188 — render_orbital_shape_object is 207 lines and needs a focused decomposition review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/render_objects/graphics.rs:2116 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/render_objects/graphics.rs:rotate_point_around, crates/chemsema-engine/src/render_svg.rs:rotate_point_around`
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_svg.rs:361 — write_primitive_svg is 272 lines and needs a focused decomposition review.
- `ARCH-EXACT-DUPLICATE` crates/chemsema-engine/src/render/bounds.rs:281 — Exact function body is duplicated across 2 files.
  - Evidence: `crates/chemsema-engine/src/render/bounds.rs:estimate_text_width, crates/chemsema-engine/src/render_svg.rs:estimate_text_width`
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render/labels.rs:572 — render_fragment_line_with_profiles is 237 lines and needs a focused decomposition review.
- `FRONTEND-DETACHED-TAB-TEST-GAP` scripts — No end-to-end regression covers dragging a desktop tab into a new window.
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app_window_lifecycle.js:54 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async () => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app_window_lifecycle.js:68 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("dblclick", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app_window_lifecycle.js:73 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("pointerdown", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:300 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("change", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:306 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async () => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:894 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:913 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("keydown", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:962 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("pointerup", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/app.js:3943 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("pointercancel", async () => {`
- `ARCH-EXACT-DUPLICATE` viewer/color_host.js:340 — Exact function body is duplicated across 2 files.
  - Evidence: `viewer/color_host.js:normalizeHexColor, viewer/editor_bindings.js:normalizeHexColor`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/editor_bindings.js:481 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("keydown", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/editor_bindings.js:705 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async () => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/editor_bindings.js:747 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async (event) => {`
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/editor_bindings.js:796 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async (event) => {`
- `ARCH-LARGE-FUNCTION` viewer/editor_command_controller.js:37 — createEditorCommandController is 238 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/editor_context_menu.js:267 — runCanvasContextMenuCommand is 204 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` viewer/editor_document_renderer.js:1 — Source file has 2123 lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` viewer/editor_overlay.js:549 — renderEditorOverlay is 228 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/editor_pointer_controller.js:884 — handleEditorPointerDown is 276 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/editor_toolbar_host.js:17 — createEditorToolbarHost is 220 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/primitive_dom_renderer.js:87 — renderCorePrimitive is 243 lines and needs a focused decomposition review.
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/text_editor_controller.js:637 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("pointerdown", async (event) => {`
- `ARCH-LARGE-FUNCTION` viewer/text_symbol_palette.js:1 — createTextSymbolPalette is 256 lines and needs a focused decomposition review.
- `FRONTEND-UNGUARDED-ASYNC-EVENT` viewer/text_symbol_palette.js:136 — Async DOM event handler can reject without a user-visible error or controlled state recovery.
  - Evidence: `addEventListener("click", async (event) => {`

## REVIEW (55)

- `FALLBACK-NAMED` crates/chemsema-cli/src/agent/output.rs:253 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback unresolved. Bind the generic families to a face that`
- `FALLBACK-NAMED` crates/chemsema-cli/src/main.rs:385 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = match connection_angles.len() {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml.rs:146 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml.rs:1027 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = CdxmlDefaults::default();`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml/export.rs:2030 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: &str,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml/export.rs:2031 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_size: f64,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml/export.rs:2053 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_color: &str,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml/export.rs:2054 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_family: &str,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/cdxml/import_objects.rs:772 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/document.rs:1398 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size = label`
- `FALLBACK-NAMED` crates/chemsema-engine/src/editing/arrows.rs:52 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/bond_tools.rs:64 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size = runs`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/brackets.rs:389 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = Point::new(endpoint.point.x + 6.0, endpoint.point.y - 6.0);`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/templates.rs:1238 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_side_length: f64,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/templates.rs:1616 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = engine.options.bond_stroke_world_pt().value();`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit.rs:855 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_family = session`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit.rs:859 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size = session`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit.rs:863 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_fill = session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL);`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/labels.rs:372 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_geometry = || {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/labels.rs:980 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_index: usize) -> usize {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/labels.rs:1017 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/layout.rs:88 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/runs.rs:124 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_family: &str,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/runs.rs:125 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/engine/text_edit/runs.rs:126 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_fill: &str,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/glyph_kernel.rs:245 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/glyph_kernel.rs:947 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback:`
- `FALLBACK-NAMED` crates/chemsema-engine/src/glyph_kernel.rs:1540 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_rect_profile(1.0, -0.86, 1.0, 0.14);`
- `FALLBACK-NAMED` crates/chemsema-engine/src/label_rules.rs:620 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = decide_label_layout(&[90.0], true, true);`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render_objects/arrows.rs:1014 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: RenderArrowEndpointStyle,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render_primitives.rs:571 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render_primitives.rs:573 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_fill: Option<&str>,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render_svg.rs:21 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_size: Option<(f64, f64)>,`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render_svg.rs:634 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render/bond_geometry.rs:730 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: f64) -> f64 {`
- `FALLBACK-NAMED` crates/chemsema-engine/src/render/bounds.rs:281 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback_font_size: f64) -> f64 {`
- `FALLBACK-NAMED` viewer/app.js:3292 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackStyle) {`
- `FALLBACK-NAMED` viewer/app.js:3564 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = "") {`
- `FALLBACK-NAMED` viewer/browser_document_tabs.js:219 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback: droppedFiles };`
- `FALLBACK-NAMED` viewer/color_host.js:23 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = fallbackColorDialogPalette(initialColor, customColors);`
- `FALLBACK-NAMED` viewer/color_host.js:23 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackColorDialogPalette(initialColor, customColors);`
- `FALLBACK-NAMED` viewer/document_flow.js:480 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackFormat = null) {`
- `FALLBACK-NAMED` viewer/editor_context_menu.js:284 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackChanged = !!(await apply());`
- `FALLBACK-NAMED` viewer/editor_pointer_controller.js:1493 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackAt = performance.now();`
- `FALLBACK-NAMED` viewer/editor_pointer_controller.js:1512 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackMs: fallbackAt - executedAt,`
- `FALLBACK-NAMED` viewer/engine_bridge.js:1 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = null) {`
- `FALLBACK-NAMED` viewer/engine_host.js:1572 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = null) {`
- `FALLBACK-NAMED` viewer/primitive_dom_renderer.js:20 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = 0) {`
- `FALLBACK-NAMED` viewer/primitive_dom_renderer.js:414 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackCenter = null) {`
- `FALLBACK-NAMED` viewer/render_support.js:9 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = CHEMDRAW_INK) {`
- `FALLBACK-NAMED` viewer/text_editor_model.js:34 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackStyle, normalizeColor) {`
- `FALLBACK-NAMED` viewer/text_editor_render.js:20 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackStyle = baseStyle(root);`
- `FALLBACK-NAMED` viewer/text_symbol_palette.js:287 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback = { symbol: "P", atomicNumber: 15, name: "Phosphorus", column: 15, row: 3, color: null };`
- `FALLBACK-NAMED` viewer/toolbar.js:667 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallbackBondToolIconSvg(type = "single") {`
- `FALLBACK-NAMED` viewer/toolbar.js:685 — A fallback-named path requires proof that it is a documented default or an explicit compatibility branch.
  - Evidence: `fallback");`
