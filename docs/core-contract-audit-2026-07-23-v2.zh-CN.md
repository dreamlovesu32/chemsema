# ChemSema 核心契约自动审查

生成时间：`2026-07-23T08:13:42.814Z`

本报告只把可机械证明的问题列为 error；需要结合设计文档判断的候选项列为 review。
文档规定的默认值不是 fallback；未知类型静默跳过、失败后改走另一套语义、吞异常才是禁止的 fallback。

## 摘要

- Error: 0
- Warning: 30
- Review: 0

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

## ERROR (0)


## WARNING (30)

- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/agent/capture.rs:3 — capture_command is 319 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/agent/context.rs:3 — context_command is 279 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-cli/src/main.rs:1 — Source file has 2293 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-cli/src/main.rs:1467 — run_command_script is 201 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdx.rs:1 — Source file has 2060 production logic lines and needs an ownership review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml.rs:1 — Source file has 2343 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml.rs:264 — parse_cdxml_document is 225 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml/export.rs:1 — Source file has 2415 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/export.rs:1108 — write_line_object is 219 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/export.rs:1411 — write_shape_object is 260 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/cdxml/import_objects.rs:1 — Source file has 2458 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/import_objects.rs:362 — append_line_objects is 214 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/cdxml/import_objects.rs:1425 — append_bracket_objects is 286 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine.rs:1 — Source file has 3638 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine.rs:2309 — execute_command is 347 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/chemistry.rs:16 — insert_smiles_untracked is 204 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/chemistry.rs:298 — chemical_molecule_for_targets is 227 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/engine/context_menu.rs:141 — context_menu_items is 209 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/engine/text_edit/labels.rs:1 — Source file has 2047 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/legacy_mol.rs:40 — parse_molblock is 241 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_objects.rs:283 — render_fragment_atom_properties is 237 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` crates/chemsema-engine/src/render_objects/graphics.rs:1 — Source file has 2399 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_objects/graphics.rs:188 — render_orbital_shape_object is 205 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render_svg.rs:333 — write_primitive_svg is 272 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` crates/chemsema-engine/src/render/labels.rs:572 — render_fragment_line_with_profiles is 237 lines and needs a focused decomposition review.
- `ARCH-LARGE-FILE` viewer/app.js:1 — Source file has 3226 production logic lines and needs an ownership review.
- `ARCH-LARGE-FUNCTION` viewer/editor_overlay.js:549 — renderEditorOverlay is 228 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/editor_pointer_controller.js:884 — handleEditorPointerDown is 276 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/editor_pointer_controller.js:1320 — handleEditorPointerUp is 218 lines and needs a focused decomposition review.
- `ARCH-LARGE-FUNCTION` viewer/primitive_dom_renderer.js:87 — renderCorePrimitive is 243 lines and needs a focused decomposition review.

## REVIEW (0)
