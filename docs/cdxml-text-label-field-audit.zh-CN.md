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

- [Revvity 当前 CDXML DTD](https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd)
- [Object Tag 对象](https://chemapps.stolaf.edu/iupac/cdx/sdk/ObjectTag.htm)
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

## 节点立体标记与对象标签补充规则

本轮公共 CDXML/CDX 像素对照补充了以下导入与绘制规则。这些规则按字段语义执行，不依赖文件名、案例编号或分子结构特例。

1. `objecttag` 不是一律隐藏的元数据。`Name="stereo"`、`Name="enhancedstereo"` 和括号标签所包含的 `t/s` 文本都必须作为场景文字导入，并保留坐标、字体、字号、颜色和样式段。
2. 括号标签采用字段优先级，而不是 `CreationProgram` 版本分支：同一 `graphic` 只有 `bracketusage` 时绘制它；新增 `parameterizedBracketLabel` 时，由后者提供 ChemDraw 实际生成的文字和位置，旧 `bracketusage` 仅保留为隐藏的往返数据。ChemDraw 22.2/23.1 文件是在旧结构上增加这个字段，没有改变旧字段在老文件中的含义。
3. `parameterizedBracketLabel` 是一个生成字段：即使它或内部 `t` 标记为 `Visible="no"`，只要它与 `bracketusage` 同时存在，ChemDraw 仍用它生成可见括号标签。这是此专用字段的语义例外；普通对象标签仍严格遵守 `Visible`。
4. 自动定位（`PositioningType` 缺省或 `auto`）时，文字左边缘位于承载它的右括号外侧 `0.1875 × 字号`，纵向基线继续使用 `t.p.y`。不能把 `t.p.x` 当成文字左边缘，因为它会随首字符及字体度量变化。
5. 显式定位（`PositioningType="offset"`、`absolute` 或 `angle`）时，使用文件记录的 `t.p`；这类字段会有意覆盖自动括号间距。ChemDraw 23.1 的同一文件同时包含自动和 `offset` 标签，证明这里应按定位字段分支，而不是按文件版本分支。
6. 节点具有 `EnhancedStereoType`/`EnhancedStereoGroupNum`、但文件没有保存可见 `enhancedstereo` 对象标签时，ChemDraw 会生成 `abs`、`orN` 或 `&N`。ChemSema 同样生成标签；有显式楔键时，标签放在楔键的反方向，以落入立体中心周围的空白扇区。
7. 四面体节点的 `HDot="yes"` 绘制实心圆点；`HDash="yes"` 绘制位于节点下方的两条短横线。尺寸来自文档 `BoldWidth`、`LineWidth`，不能按截图像素或单个样例写死。
8. `NodeType="MultiAttachment"` 且当前没有实际连接键时，绘制 ChemDraw 的三线星号占位标记；一旦该节点被键连接（例如金属到芳环多中心键），不再绘制星号。星号直径约为文档 `BondLength` 的 30%，线宽使用 `LineWidth`。
9. 分裂不连通分子组件时，带 `MultiAttachment`、`HDot` 或 `HDash` 语义的孤立碳节点仍属于可见内容，不能按“无键普通碳”过滤。

对应回归测试覆盖显式/隐藏对象标签、缺省增强立体标签、`HDot`、`HDash` 和未连接 `MultiAttachment`；公共图像门禁继续负责验证这些语义的最终像素位置和尺寸。
