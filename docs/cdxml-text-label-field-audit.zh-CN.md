# CDXML/CDX 文本与原子标签字段审计

本文记录 ChemSema 对 CDXML/CDX 文本与原子标签字段的系统审计。审计依据 CambridgeSoft CDX SDK 的公开镜像、真实 ChemDraw 的受控对照输出，以及公开语料的连续三代往返结果。

## 审计结论

- 413 个公开文件中，407 个可导入案例完成连续三代保存与重开；404 个完全一致，1 个为预期安全清洗，2 个为预期无损归一化。
- 2 个文件按预期拒绝导入，4 个传输编码文件跳过。
- 在本审计覆盖的文本/标签语义和几何门禁内，未预期失败、语义漂移、非幂等和未分类计数漂移均为 **0**。
- 标签专项扫描覆盖 5,823 个原子标签、37 种相关字段组合；30 个受控 ChemDraw 对照案例全部符合预期。

这个“0”只表示下述字段族与当前公开语料的严格门禁已清零，不表示 CDXML/CDX 所有对象和所有私有扩展都已穷尽。

## 官方字段职责

| 字段 | 官方对象/类型 | 官方含义与默认 | ChemSema 处理原则 |
| --- | --- | --- | --- |
| `LabelDisplay` | Node / INT8 | 原子标签相对节点的显示位置；默认 `Auto` | 仅显式非 `Auto` 值固定标签锚点和保留作者文本，不由它合成新行 |
| `LabelAlignment` | Node / INT8 | 多行原子标签的对齐方式；默认 `Auto` | 只控制行对齐元数据，不改变字符顺序和自动化学布局方向 |
| `LabelJustification` | Node / INT8 | 原子标签文本的排版方式；默认 `Auto` | 只用于原子标签，不参与自由文本排版 |
| `Justification` | Text / INT8，已废弃 | 旧式自由文本对齐 | 仅作自由文本后备值，优先级低于 `CaptionJustification` |
| `CaptionJustification` | Text / INT8 | 自由文本对齐 | 自由文本的首选对齐字段 |
| `InterpretChemically` | Text / BOOL | 是否把自由文本按化学含义解释 | 保留字段语义，不把它混同为 Node 标签布局开关 |
| `LineHeight` | Text/Node / UINT16 | 旧式行高；`0` 为 variable，`1` 为 auto | 兼容读取；优先级低于对象专用的新字段 |
| `LabelLineHeight` | Node / INT16 | 原子标签行高 | 原子标签首选行高；再回退对象/根级旧字段 |
| `CaptionLineHeight` | Text / INT16 | 自由文本行高 | 自由文本首选行高；再回退对象/根级旧字段 |
| `WordWrapWidth` | Text/Node / INT16 | 自动换行宽度 | 原样保留并纳入往返门禁 |
| `LineStarts` | Text/Node / varies | 各行起始字符位置 | 保留作者行结构，不以自动布局重新分行 |

字段优先级按对象类型分开处理：Node 标签使用标签字段；Text 自由文本使用 caption 字段，旧 `Justification`/`LineHeight` 仅作为兼容后备。根级行高默认也按相同的“专用字段优先、旧字段后备”规则继承。

## ChemDraw 对照结论

受控文件包含 30 个左右、左、上、下连接方向以及各种对齐/显示组合，交由真实 ChemDraw 打开并导出 SVG。观察结果如下：

1. `LabelAlignment`、`LabelJustification` 和旧 `Justification` 不会覆盖随连接方向决定的化学字符顺序与堆叠。
2. 显式 `LabelDisplay` 会保留作者写入的字符顺序和行结构；`Above`/`Below` 不会凭空把单行标签拆成两行。
3. 使用 `BeginAttach`/`EndAttach` 字符索引的标签必须保留作者顺序，否则附着点会指向错误字符。
4. 方括号包围的查询标签（例如 `[C,N,P]`）是一个整体，不能按普通末端氢规则拆分或翻转。

## 本轮清理的系统性错误

1. 将 `LabelAlignment`/`LabelJustification` 错当成标签字符流方向。
2. 显式 `LabelDisplay` 的 `Above`/`Below` 被错误实现为自动造行。
3. 导入后丢失作者已有换行与样式段结构。
4. 带 `BeginAttach`/`EndAttach` 的标签被重新排序。
5. 方括号查询标签被拆成 `C,N,P]` 与 `[` 两部分。
6. 自由文本错误读取 `LabelJustification`，没有执行 `CaptionJustification > Justification > 文档默认` 的优先级。
7. `LabelLineHeight`、`CaptionLineHeight`、旧 `LineHeight` 及根级默认的继承和自动行高不一致。
8. CDX 二进制层缺少行高/换行宽度字段和 `LabelAlignment=Best` 的完整编解码。
9. 单节点分子往返时被误降级成自由文本。
10. 旧公开语料门禁只比较源标签，没有比较最终显示文本、行结构和文字几何，因而会漏报上述问题。

## 自动门禁

公开语料门禁现在逐代比较：

- 原子标签源文本、最终显示文本、行结构和样式段；
- 对齐、布局方向、锚点、换行宽度和解析后的行高；
- 自由文本内容、样式段、对齐、换行和几何；
- 标签/文本位置、边界框和行高（允许 0.5 pt 的数值舍入容差）；
- 分子、无头箭头、括号语义，以及对象/资源/样式计数。

详细运行结果写入 `tmp/public-cdxml-roundtrip-label-audit/report.json`。基准摘要和运行说明见 `benchmarks/public-cdxml/README.zh-CN.md`。

## 尚未声称解决的范围

- 公开 SDK 对部分现代 ChemDraw 的视觉实现细节没有完整说明，字体字形度量仍需真实 ChemDraw 和肉眼对照。
- 0.5 pt 几何容差用于吸收 CDXML/CDX 数值量化，不允许文本、行结构或样式段发生差异。
- 本报告没有宣称审计了所有 CDX 属性、所有图形对象或厂商私有扩展；后续字段族应采用同样的“官方定义 + ChemDraw 对照 + 多代语料门禁”流程。

## 规范来源

- [Node 对象](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/Node.htm)
- [Text 对象](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/Text.htm)
- [LabelDisplay](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/Node_LabelDisplay.htm)
- [LabelAlignment](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelAlignment.htm)
- [LabelJustification](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelJustification.htm)
- [Justification（旧字段）](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/Justification.htm)
- [InterpretChemically](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/InterpretChemically.htm)
- [LineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LineHeight.htm)
- [LabelLineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelLineHeight.htm)
- [CaptionLineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/CaptionLineHeight.htm)
- [WordWrapWidth](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/WordWrapWidth.htm)
- [LineStarts](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LineStarts.htm)
