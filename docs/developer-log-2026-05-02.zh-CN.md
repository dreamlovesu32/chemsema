# Chemcore 开发者日志 - 2026-05-02

作者：张家骏

时间范围：2026-05-02 00:00 至 2026-05-02 23:59，Asia/Shanghai

对比提交：`0836b15 feat: add shape tools and engine workflow rules`

## 总结

本轮工作把 CDXML 从“外部转换器/前端兼容路径”推进到 Rust 内核的一等输入输出能力。导入侧现在直接在 `chemcore-engine` 中解析 CDXML，生成内核原生 molecule fragment、text、line、arrow 和 shape 对象；导出侧新增内核 CDXML writer，可以把当前 `ChemcoreDocument` 写成 ChemDraw 可识别的 CDXML 文档。

另一条主线是 ChemDraw 绘图风格收敛：键长、线宽、粗键宽、hash spacing、双键间距和 ACS Document 1996 preset 都进入 engine 选项和渲染公式。CDXML 导入后继续画键会继承源文件格式；如果源文件匹配 ACS preset，viewer 右上角样式下拉会同步到 ACS，也可以正常切回 Default。

同时补齐了端点标签的化学识别：隐式氢、元素标签、terminal/bridge abbreviation、组合缩写、`N3`、`CF3`、`t-Bu/tBu` 这一类 whole-label 规则都进入 Rust engine。viewer 只消费 engine 的状态和 render primitive，不再自己定义这些化学行为。

## 内核边界

本轮继续强化项目规则中的内核边界：

- 新增 `crates/chemcore-engine/src/cdxml.rs`，CDXML parse/export 都在 Rust 内核中实现。
- 新增 `crates/chemcore-engine/src/abbreviation.rs`，缩写识别、alias、组合解析和展开元数据都在 Rust 内核中实现。
- `quick-xml` 加入 `chemcore-engine` 依赖，用于 CDXML XML 解析。
- `lib.rs` 导出 `cdxml` 和 `abbreviation` 模块，供 tests、wasm 和 engine 使用。
- viewer 只负责打开/保存文件、toolbar 状态同步和 SVG/DOM 显示。

这避免了 CDXML、标签识别和继续画键的行为散落在前端转换层。

## CDXML 导入

导入路径新增 `parse_cdxml_document()`，覆盖 ChemDraw 常见文档结构：

- 读取 CDXML root defaults：`BondLength`、`LineWidth`、`BoldWidth`、`HashSpacing`、`BondSpacing`。
- 解析 color table 和 font table，并兼容 ChemDraw legacy palette id。
- 将 display fragment 转为 `molecule_fragment2d` resource。
- 将 `n` 节点转为内核 `Node`，保留元素、占位/缩写节点、charge、hydrogen count 和 CDXML source meta。
- 将 `b` 键转为内核 `Bond`，保留 order、double placement、stereo、line style、line weight、bond spacing、hash spacing 和 bold width。
- 将 CDXML `arrow` / `graphic Line` 转为 `line` 对象。
- 将 rectangle/oval 转为 `shape` 对象，保留 fill、stroke、dash、shadow、shaded 等样式信息。
- 将自由文本框转为 `text` 对象，保留文本、bbox、alignment、font size、runs 和颜色。

结构标签不再按 CDXML 原始文本框直接画。导入后会调用内核 attached-label 排版，确保 `NH`、`O`、`CF3`、`t-Bu` 等节点标签走同一套标签引擎，避免 ChemDraw 文本框裁剪和我们的 internal label clipping 互相冲突。

## 编辑态 CDXML 归一

底层 `parse_cdxml_document()` 保留 CDXML display fragment 的原始对象划分，方便测试和后续 round-trip 分析。但 `Engine::load_cdxml_document()` 会额外执行编辑态归一：

- 多个 CDXML molecule fragment 会合并为一个 editable fragment。
- 合并时把每个 fragment 的节点、键、标签 bbox 和 glyph polygon 转成统一局部坐标。
- 原始 parser 行为不变；只有进入编辑器的文档会合并。

这个改动修复了“导入后很多键无法聚焦”的问题。旧的 hit-test 和编辑链路只看 `document.editable_fragment()`，也就是第一个 molecule object；多 fragment CDXML 中后面的键不会参与聚焦。合并后，导入文件中的所有分子键都进入同一个可编辑图。

## CDXML 导出

新增 `document_to_cdxml()` 和 `Engine::document_cdxml()`，wasm 暴露为 `documentCdxml()`。导出目标不是复制 ChemDraw 文件里的所有脏字段，而是从 `ChemcoreDocument` 写出核心、干净、可读的 CDXML：

- 写出标准 `CDXML` root、DOCTYPE、page、color table 和 font table。
- molecule object 写为 `<fragment>`，节点写为 `<n>`，键写为 `<b>`。
- 普通碳节点保持简洁；元素节点写 `Element`。
- 缩写/占位标签写为 `NodeType="Nickname"`，并生成 `<t><s>...</s></t>` label。
- 双键写出 `Order`、`DoublePosition`、`BondSpacing`、`LineWidth`、`BoldWidth`、`HashSpacing`。
- 楔键、虚楔、dash、bold double line 转回 CDXML display 属性。
- 自由文本写为 `<t>`；线/箭头写为 `graphic` 或 `arrow`；矩形/椭圆写为 `graphic`。
- 颜色从文档 style 和 label runs 收集进 color table，run 级 fallback 会继承标签颜色。

viewer 新增 “Save CDXML” 按钮，并接入浏览器 save picker；同时打开文件路径支持 `.cdxml` 和常见 CDXML MIME type。

## ChemDraw / ACS 绘图格式

本轮重新校准了默认和 ACS Document 1996 的绘图参数：

- ACS preset：键长 `14.4`、线宽 `0.6`、粗键宽 `2.0`、hash spacing `2.5`、图形线宽 `0.6`。
- 新画键、模板键、删除降级后的新键、粘贴/模板生成键都会继承当前 `EditorOptions`。
- CDXML 导入时从 root defaults 和实际键数据推断当前绘图选项。
- 如果导入文件匹配 ACS preset，`Engine::document_style_preset()` 返回 `acs-document-1996`。
- viewer 在 load JSON/CDXML 后从 engine 反向同步 preset 下拉，避免旧 UI 状态覆盖导入格式。
- 切换 ACS 后可以再切回 Default，并会按键长比例缩放现有文档。

这样导入 ACS 样式文件后继续画键不会回到默认样式。

## 双键和键绘制

双键间距不再使用固定视觉比例，而是按 ChemDraw 的 `BondSpacing` 和实际键长计算：

```text
inner_gap = max(bond_length * BondSpacing / 100 - line_width, line_width * 0.5)
center_distance = inner_gap + (width_a + width_b) / 2
```

这里 `width_a` 和 `width_b` 会受普通线宽、粗线宽和 double line weight 影响。hash wedge 的间距也会读取 bond-level `HashSpacing`。三键和侧双键继续随键长变化，避免用户拉长末端键后间距仍像静态量出来的值。

相关渲染路径同时补齐：

- bond-level `bold_width`、`hash_spacing`、`bond_spacing` 字段。
- 粗键接触和 join 计算使用 bond-level bold width。
- dash/hash knockout 使用当前线宽和 spacing。
- 导入的 dashed double、bold double、side double 和 ACS fixtures 有回归测试。

## 标签、隐式氢和缩写识别

新增缩写识别模块后，端点标签不再只是普通文本：

- 简单元素标签会进入元素识别，并根据连接数刷新隐式氢。
- `N`、`O`、`P`、`S`、卤素、`B`、`Si` 等隐式氢规则写入 `docs/implicit-hydrogen-rules.zh-CN.md`。
- terminal abbreviation 支持 `Me`、`Et`、`Pr`、`iPr`、`Bu`、`iBu`、`sBu`、`tBu`、`Ph`、`Bn`、`Ac`、`Boc`、`Cbz`、`Fmoc`、`TMS` 等。
- 组合缩写支持 `CO2Et`、`COOEt`、`OAc`、`SO2Me` 等由 linker + terminal 组成的标签。
- 两键桥接标签支持 `NH`、`CO`、`CO2/COO`、`OCO`、`SO/SO2`、`CH2` 和部分 `NMe/NTs`。
- `N3` 识别为叠氮基。
- `CF3` 走正常缩写识别；右侧连接时显示为 `F3C`，锚点仍在 `C` 上。
- `t-Bu` 和 `tBu` 作为同一个合法标签识别，`nBu/iPr` 等同类 alias 同样进入合法标签系统。
- 已识别 whole-label 缩写和未知非法标签在靠左连接时都按整体处理，锚点落在最右侧字母组。

识别结果写入 `meta.labelRecognition`，并在格式文档中补充了 `functionalGroupExpansion.v1` 的语义层说明。这个 expansion 是附加语义，不替换主分子图。

## 文本编辑和标签排版

端点标签编辑继续收敛到内核：

- text edit session 可以打开普通 text object，也可以打开 endpoint label。
- preview/apply 使用 Rust label kernel 生成 source runs、display runs、bbox、glyph polygons 和 caret geometry。
- 端点标签 hover 时优先显示整块 label box，而不是普通端点圆。
- 编辑中隐藏当前标签的 document text/knockout/hover primitive，避免 DOM 文本编辑器和 SVG 标签重叠。
- 重新打开 endpoint label 时保留稳定 anchor、bbox 和 source text。
- 自动生成的隐式氢进入编辑文本，但不能成为画键锚点；从生成氢处拖键会回到重原子锚点。

viewer 的 `text_editor_controller` 只负责 DOM 文本编辑器的交互和定位，几何仍以 engine 返回的 layout 为准。

## 选择、命中和交互

选择和聚焦行为配合 CDXML/native label 做了调整：

- `RenderPrimitive` 增加 `node_id`，让 hover/text primitive 能和 endpoint label 关联。
- text 工具可以 hover 已存在标签，并打开 endpoint label 编辑。
- select/delete/template 路径在结构变化后刷新标签几何。
- 多 fragment CDXML 在进入 engine load 后合并，保证 hit-test 覆盖所有导入键。
- bond center hover 和 cycle style 仍复用现有内核 hit-test，但数据源现在是合并后的完整 fragment。

## 文档和格式

文档更新包括：

- README 和中文 README 增加隐式氢规则、缩写识别规则链接。
- `docs/project-rules.zh-CN.md` 明确化学标签行为属于 Rust engine。
- `docs/format-v0.1.md` 和中文版本补充 `meta.labelRecognition`、`functionalGroupExpansion.v1`、source-format bit mask 不进入核心字段等规则。
- 新增 `docs/implicit-hydrogen-rules.zh-CN.md`。
- 新增 `docs/abbreviation-recognition-rules.zh-CN.md`。

## Viewer 和 Wasm

viewer 层更新：

- 打开文件支持 JSON 和 CDXML。
- 保存支持原有 JSON 和新增 CDXML。
- toolbar 增加 document style preset 下拉：Default / ACS Document 1996。
- load 后从 engine 读取 `documentStylePreset()`，避免 preset UI 和内核不同步。
- 渲染支持带 `nodeId` 的 primitive，用于隐藏正在编辑的 endpoint label。
- wasm 绑定新增 `loadDocumentCdxml()`、`documentCdxml()`、`documentStylePreset()` 和 document style setter。
- `viewer/engine` 的 JS、d.ts 和 wasm 二进制已重建。

## 测试和验证

测试覆盖明显扩展：

- CDXML assets/native molecule import。
- CDXML arrow、shape、free text、table line/text import。
- ChemDraw legacy color palette。
- CDXML node label 走内部 attached label layout。
- default / ACS 双键间距 fixture。
- 拉长键后的双键间距随实际键长变化。
- CDXML exporter round-trip。
- 多 CDXML fragment 进入 engine 后可编辑和可 hit-test。
- CDXML load 后继承 ACS drawing options。
- ACS preset 新画键、粗键、图形线宽和切回 Default。
- 标签缩写识别、`CF3`、`t-Bu/tBu`、非法 whole-label anchor。
- 隐式氢、生成氢锚点、端点标签重新打开和预览 geometry。

本轮提交前运行过：

- `cargo test -p chemcore-engine`
- `./scripts/build-engine-wasm.sh`

