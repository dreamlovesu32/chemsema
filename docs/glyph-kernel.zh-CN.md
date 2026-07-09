# Glyph Kernel

## 目标

`chemcore` 需要与宿主无关的化学标签文本几何。

Rust engine 负责：

- 用于键裁剪的逐字形 label 几何
- glyph advance 估算
- 下标/上标缩放和 baseline shift
- 用于 knockout 和 label-aware bond retreat 的 background padding

如果各宿主独立推导这些细节，Web 和桌面渲染器会逐渐产生偏差。

## 当前模型

当前活跃的 glyph geometry 实现在 Rust 中：

- [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)

Rust engine 现在消费三个共享 manifest：

- [shared/glyph_profiles.json](../shared/glyph_profiles.json)
- [shared/glyph_clip_polygons.json](../shared/glyph_clip_polygons.json)
- [shared/text_symbols.json](../shared/text_symbols.json) 列出 viewer 符号表和 profile 生成脚本使用的文本符号 catalog

`glyph_profiles.json` 仍是归一化文本排版 metrics 的来源：

- 归一化 glyph advance
- 归一化 ink bounds
- 用于 label layout metrics 的保守 background padding
- normal / subscript / superscript layout
- 对共享 profile manifest 中缺失字符的保守 Unicode 类别兜底

`glyph_clip_polygons.json` 是运行时 label clipping geometry 的唯一来源。Rust kernel 不再在运行时按 `rect / ellipse / cut-corner / petal` 合成多边形。

输出用于 attached-label layout、label anchor geometry、label-aware bond clipping 和 text edit preview geometry。

## 固定裁剪规则

当前裁剪方案是数据驱动且确定性的：

1. Layout 仍从 `glyph_profiles.json` 中的归一化 ink box 开始。
2. 实际裁剪多边形从 `glyph_clip_polygons.json` 读取。
3. ASCII 大写字母使用预计算多边形，构成包括：
   - 基准自然轮廓外扩：`10pt` 参考字号下为 `1.0pt`
   - 内向锚点偏移：`0.22 * glyph height`
   - 基准锚点圆半径：`10pt` 参考字号下为 `2.0pt`
4. 非大写符号只使用自然轮廓外扩：
   - 基准自然轮廓外扩：`10pt` 参考字号下为 `1.0pt`
5. 运行时把轮廓外扩量重新映射到文档源 margin 的绝对 pt。CDXML 导入时，自然外扩等于
   `MarginWidth`，锚点圆半径等于 `2 * MarginWidth`；二者都不随实际 label 字号缩放。
6. 缺失于 clip manifest 的可见字符属于 manifest 生成失败；运行时不再即时合成替代形状。

详细的大写字母锚点规则见：

- [docs/glyph-clip-polygons.zh-CN.md](./glyph-clip-polygons.zh-CN.md)

## Manifest 生成

裁剪 manifest 由以下命令生成：

```bash
python scripts/generate-glyph-profiles.py
python scripts/generate-glyph-clip-polygons.py
```

当前 clip manifest 来自 `Arial` 轮廓几何，并固定为上面的基准 pt 值。运行时渲染器消费这些预计算的 petal/corner 规则，并把外扩量从基准源 margin 映射到文档源 margin。

## 消费链路

同一套 glyph polygons 现在贯穿整个栈：

- `chemcore-engine` Rust kernel 构造 glyph polygons：
  - [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)
- label-aware bond clipping 直接使用这些 polygons：
  - [crates/chemcore-engine/src/render/labels.rs](../crates/chemcore-engine/src/render/labels.rs)
- document knockouts 使用同一套 polygons：
  - [crates/chemcore-engine/src/render_objects.rs](../crates/chemcore-engine/src/render_objects.rs)
- Office / EMF preview 通过同一 glyph clipping algorithm 重放 engine polygons：
  - [apps/chemcore-office/src/windows_office/emf_preview/renderer.rs](../apps/chemcore-office/src/windows_office/emf_preview/renderer.rs)

这意味着 kernel clipping、SVG/document knockouts 和 EMF preview 共享同一个几何来源。

## Web 状态

Web viewer 通过 WASM 消费 Rust engine state 和 render primitives：

- [crates/chemcore-engine/src/wasm.rs](../crates/chemcore-engine/src/wasm.rs)
- [viewer/app.js](../viewer/app.js)

旧的 C++ glyph kernel 和 standalone glyph WASM path 已移除。当前验证应通过 Rust engine tests 和 viewer engine WASM build。
