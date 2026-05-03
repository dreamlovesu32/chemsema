# Chemcore 开发者日志 - 2026-05-03

作者：张家骏

时间范围：2026-05-03 00:00 至 2026-05-03 23:59，Asia/Shanghai

对比提交：`7f406c9 docs: expand label recognition developer log`

## 总结

本轮工作把括号、重复单元、电荷/自由基符号和 formula-like 标签识别推进到 Rust engine 内核。viewer 新增括号工具和符号工具，Wasm 暴露对应工具选项与双击组件选择；CDXML 导入/导出开始支持 ChemDraw 的 bracket 和 symbol graphic；渲染层新增圆括号、方括号、花括号以及 8 类电荷/电子符号的原生 primitive。

化学语义方面，符号不再只是独立装饰图形。正负电荷、自由基阳离子/阴离子、单电子和孤对符号可以归属到端点或 attached label 的重原子，并刷新节点电荷、自由基、有效隐式氢、非法状态和 repeating unit expansion。标签识别也从固定缩写组合继续扩展，新增价键驱动 parser，用于识别 `CN`、`CO2Cl`、`CH2COOCH2SO2NHCl` 这类 formula-like terminal label。

## 括号和符号工具

编辑内核新增 `Tool::Bracket` 和 `Tool::Symbol`，并在 `ToolState` 中保存 `bracket_kind` 和 `symbol_kind`。

- Bracket 工具支持拖拽创建圆括号、方括号和花括号对象。
- Symbol 工具支持创建 `circle-plus`、`plus`、`radical-cation`、`lone-pair`、`circle-minus`、`minus`、`radical-anion` 和 `electron`。
- 点击端点、attached label 或普通文本附近时，符号会自动选择合理插入位置。
- 从端点或 label glyph 拖拽符号时，符号按端点/label 的椭圆轨道定位。
- Bracket 工具保留 bond center hover，避免误聚焦端点；Symbol 工具聚焦端点和 label，但不聚焦 bond center。
- Select 工具可以点选、框选、移动、排列和删除 bracket/symbol 对象。
- 双击分子组件时会选择整块 connected component，并把包围该组件的 bracket 一起纳入选择。

viewer 侧新增主工具按钮和二级 toolbar。括号按钮提供 3 种 bracket kind，符号按钮提供 8 种电子/电荷符号；工具状态通过 `setBracketOptions()` 和 `setSymbolOptions()` 同步到 Wasm engine。Bracket 创建完成后，viewer 会在右下角打开文本编辑器，便于直接输入重复单元计数。

## 渲染和 CDXML

渲染层新增 bracket/symbol 对象路径：

- 圆括号用椭圆弧近似，方括号按 lip 比例绘制，花括号用 cubic path 绘制。
- Dagger / double dagger 用填充路径绘制。
- 带圈正负使用外圈 path 加内部符号；普通正负使用填充矩形组合。
- Radical cation / radical anion / lone pair / electron 使用圆点和正负号组合。
- ACS 和 default 两套 symbol metrics 分开处理，保证线宽和尺寸接近 ChemDraw 输出。
- 对归属后非法的 charge/radical 状态，渲染层会在节点附近画红色圆形 invalid marker。

CDXML 路径同步补齐：

- 导入 `GraphicType="Bracket"`，把左右 bracket graphic 配对成一个 `SceneObject { type: "bracket" }`。
- 导入 `GraphicType="Symbol"`，映射 ChemDraw 的 `SymbolType` 到内核 symbol kind。
- 按 CDXML 默认线宽推断 symbol style 和 metrics，保留 anchor width/height、line width 和原始 bbox。
- 导出 bracket/symbol 时重新写回 CDXML graphic，包括 bracket pair、symbol type、bbox 和 z-order。
- CDXML root defaults 导出时优先使用导入 defaults、文档 style、实际 bond 或 symbol line width，避免 round-trip 后绘图参数回落到内核默认值。

新增 `crates/chemcore-engine/examples/cdxml_render_metrics.rs` 和 `scripts/compare-cdxml-symbol-pixels.mjs`，用于量测 CDXML 渲染 metrics 和对比 ChemDraw SVG 符号像素差异。

## 符号化学语义

新增 `crates/chemcore-engine/src/symbols.rs`，集中处理电荷/自由基符号归属和节点语义刷新。

- 符号中心距离端点或 label anchor 在 10pt 内时，会归属到最近候选原子。
- symbol 对象写入 `chemicalRole`、`chargeDelta`、`radicalDelta`、`attachedAtomId`、`attachmentSource` 和 `attachmentDistance`。
- 节点 meta 写入 `attachedElectronSymbols`、`radicalCount`、`effectiveNumHydrogens` 和 `chargeSymbolInvalid`。
- 普通正负号改变形式电荷；radical cation/anion 同时改变电荷和自由基数；electron 增加自由基数；lone pair 第一阶段只保存显示语义。
- 归属符号后会刷新 attached label geometry、隐式氢和 repeating unit meta。
- 删除符号、移动排列符号或删除连接键后，会重新计算归属和合法性。
- 修正了无符号文档加载时不应写入 symbol bookkeeping meta 的问题，避免普通 label 被无关刷新误当作隐式氢标签重排。

## 重复单元

新增 `crates/chemcore-engine/src/repeating_units.rs`，在文档层识别 bracket + 数字文本组成的 repeating unit。

- 只识别可明确归属的 bracket 对象和右下方数字 count。
- 通过 bracket bounds 找内部 atom/bond。
- 要求左右边界各有一根 crossing bond，且两侧边界键阶一致。
- 成功识别后，在 bracket/text object meta 上写 `repeatUnitId` 和 `repeatUnitRole`。
- 在 editable fragment meta 写入 `repeatingUnits`，包含 atom ids、internal bond ids、boundary bonds、repeat count 和 expansion。
- expansion 会复制重复单元内部 atoms/bonds，并保留节点 charge、effective hydrogens、radicalCount 和 attached electron symbols。
- 无数字 count 或边界不完整时不生成 expansion，避免给不完整结构制造错误语义。

## 标签识别和文本编辑

`abbreviation.rs` 新增价键驱动 terminal label parser：

- 标签先 token 化为元素和 terminal group 片段，支持元素数量展开，例如 `O2`、`H3`。
- 外部连接先消耗 attachment atom 的一个价键单位。
- 后续 token 按价态和可用连接数从左到右建立 parent/child 关系。
- C-N、C-O/C-S、S/P/As-O 等常见多键模式优先生成更合理的键级。
- 支持 `B`、`N`、`O` 的带电例外价态，并在 component 上记录 `formalCharge`。
- recognition meta 对 valence parser 结果写入 `source: "valence-parser"`，component 记录 `parentIndex`、`bondOrderToParent` 和可选 `formalCharge`。
- `COOH`、`COCH3`、`OCH3` 会归一为 `CO2H`、`COMe`、`OMe`。

文本编辑也补了“非化学 endpoint label”路径：

- `TextEditSession.default_chemical` 会根据 source runs 判断，而不是所有 endpoint label 都默认化学。
- 非 chemical runs 不进入 abbreviation/element recognition，也不画红框。
- 非化学右侧 label 保留原始文本顺序，并使用 whole-label anchor。
- text toolbar 的 chemical 按钮现在可以在 chemical 和 normal 之间切换。
- 正电荷下的隐式氢计算不再使用 `abs(charge)` 一刀切扣氢，避免正电荷 hetero atom 无法增加氢。

## Viewer 和 Wasm

Wasm 绑定新增：

- `setBracketOptions(kind)`
- `setSymbolOptions(kind)`
- `selectComponentAtPoint(x, y, additive)`

viewer 更新：

- 主 toolbar 增加 Bracket 和 Charge/Electron Symbol 工具。
- 二级 toolbar 支持 bracket kind 和 symbol kind 切换。
- 主 symbol 按钮会显示当前 symbol kind 图标。
- pointer routing 覆盖 bracket/symbol 工具。
- bracket 拖拽结束后自动打开 count 文本编辑器。
- select 工具双击调用 engine component selection。
- 渲染 bracket/symbol 时直接消费 engine render primitive。
- `viewer/engine` 的 JS、d.ts 和 wasm 产物已重建。

## 文档

新增和更新的文档：

- 新增 `docs/charge-radical-symbol-rules.zh-CN.md`，记录 8 类符号的归属、价态、隐式氢、非法状态和 expansion 设计。
- 新增 `docs/valence-label-recognition-rules.zh-CN.md`，记录 formula-like label 的价键解析计划。
- 更新 `docs/abbreviation-recognition-rules.zh-CN.md`，把下一阶段 formula-like parser 指向独立设计文档。
- 更新 `docs/project-rules.zh-CN.md`，把价键标签规则和电荷/自由基符号归属规则列入项目规则基线。

## 测试和验证

测试覆盖新增：

- bracket 拖拽创建、hover 策略、CDXML bracket/symbol import。
- symbol 创建、选择、端点/label 轨道定位、ACS metrics、default/ACS symbol 渲染尺寸。
- 电荷/自由基符号归属到碳和杂原子后的 charge、hydrogen、invalid marker。
- 四连接碳上的普通电荷/单电子 invalid，radical ion 允许。
- bracketed repeating unit 识别、计数文本匹配、expansion 和无 count 时不生成 expansion。
- 双击 component selection 自动包含包围 bracket。
- 非化学 endpoint label 不进入 recognition、不画红框、重新打开编辑器时保持 non-chemical 状态。
- valence parser 的 formula-like label、带电 B/N/O 例外和命名 terminal group。

本轮提交前运行过：

- `cargo fmt`
- `cargo test`：通过。
- `npm run build:engine-wasm`：通过，并重建 `viewer/engine` 产物。
- `node --check viewer/app.js`：通过。

说明：`npm run verify` 的测试和 wasm 构建阶段通过，但脚本最后会在 `viewer/engine` 存在未提交生成文件差异时退出。本轮正包含这些生成文件改动，因此提交前采用上述拆分验证命令记录结果。
