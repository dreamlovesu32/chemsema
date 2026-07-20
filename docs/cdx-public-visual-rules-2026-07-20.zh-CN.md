# CDX/CDXML 公共视觉回归规则（2026-07-20）

本轮修复遵守“字段定义优先，不按文件名、对象 ID 或单张图片坐标分支”的原则。

1. CDX `NodeType` 覆盖官方 0–13 全枚举；`BracketType` 按官方枚举值解码，不能把二进制整数直接当作 CDXML 文本。
2. `SupersededBy` 规范标签为 `0x0012`。读取兼容公共 ChemDraw 文件使用的生产者别名 `0x0013`，写出统一使用规范标签。该兼容只解决同义标签，不改变对象可见性规则。
3. 旧 CDX styled text 先按 UTF-8 解码；无效 UTF-8 才回退 Windows-1252。这样既不破坏现代文件，也能读取旧专利文件中的 `°`、`±` 等字符。
4. CDXML 键 `Order` 可以包含多个官方键级。没有显式可见 `query` 对象标签时，按键级集合合成 ChemDraw 查询助记文本，例如 `1 2` 显示为 `S/D`；位置由键中点、键方向和字号共同决定。
5. bracket graphic 的显式 `LineWidth` 优先；缺省时继承文档图形线宽，不能硬编码成固定 1 pt。
6. `BondSpacingAbs` 是多重键线段中心距的绝对值；它与 `BondSpacing` 同时存在时按官方规定优先，并按该键的实际端点距离换算成内核百分比表示。

每条规则都需要字段级测试和公共 ChemDraw 像素对照。像素门禁中的旧“通过”若依赖过宽的拓扑兜底，而新结果的实际 IoU 和覆盖率更高，应修门禁分类规则，不能为保留旧 verdict 恢复错误绘制。

规范依据：

- IUPAC FAIRSpec CDX SDK：`Node_Type`、`Bracket_Type`、`Bond_Order`、`BondSpacingAbs`、`SupersededBy`。
- 真实 ChemDraw 21 保存的 CDX/CDXML 对照。
- 公共语料的 ChemDraw SVG oracle 与细节无关尺寸门禁。
