# 原子属性编辑与装饰规则

本文定义 `Isotope`、`IsotopicAbundance`、`Radical`、`AtomNumber`、
`ShowAtomNumber`、`AS` 和 `ShowAtomStereo` 在 ChemSema 内核、右键菜单、
点工具及 CDX/CDXML 往返中的统一行为。

## 语义模型

原子使用 CCJS `node.atomProperties` 保存来源无关的明确字段：

- `isotopeMass`：正整数质量数；
- `isotopicAbundance`：`unspecified`、`any`、`natural`、`enriched`、
  `deficient`、`nonnatural`；
- `radical`：`none`、`singlet`、`doublet`、`triplet`；
- `atomNumber` 与 `showAtomNumber`；
- `cipStereo` 与 `showAtomStereo`；
- `atomNumberPosition` 与 `stereoPosition` 保存可选的自动、偏移、角度或
  绝对定位意图。

普通原子编号不等于反应原子映射号，二者不得复用字段。CDXML 的缓存
`objecttag/t` 文本框是显示对象，不是原子属性的唯一语义来源。

## 编辑入口

选择一个或多个原子后，右键菜单的 **Atom Properties** 提供：

- 常用质量数和任意正整数质量数；
- 全部六种同位素丰度；
- 无、单重态、双重态、三重态自由基；
- 原子编号的设置、清除和显隐；
- CIP 描述符的设置、清除和显隐。

全部入口调用同一个可撤销的
`set-atom-property-for-selection` 内核命令。非法质量数、未知枚举或非法
布尔值必须拒绝，不能静默清空原值。

对话框的标题、字段、当前值、输入类型、约束和清空规则由内核
`atomPropertyDialogJson` 提供；Web、桌面与 Harmony 只共用一个输入法宿主，
不得在前端复制字段规则。

## 点工具与自由基

电子点是一个可选择、可拖动的 `symbol` 对象，同时通过
`attachedAtomId` 和 `radicalDelta` 贡献目标原子的有效自由基状态。

- 点工具点击原子：插入电子点、同步 `atomProperties.radical`，并立刻刷新
  该原子的隐式氢、价态和标签退让；
- 将点拖离或删除：原子的有效自由基状态立刻回退；
- 右键直接设置自由基：修改原子的基础自由基属性；
- 原子同时有基础自由基和附着电子点时，有效电子数为二者之和；
- 导出 CDX/CDXML 时必须写出有效 `Radical`，不能只保留可见点而丢失
  化学语义。

## ChemDraw 实测规则

2026-07-23 使用 `scripts/chemdraw-oracle.mjs` 后台 COM 对 ChemDraw
21.0.0.28 运行组合探针
`tmp/atom-properties-rule-probe.cdxml`，得到以下稳定行为：

1. 原子装饰字号为原子标签字号的 `0.75`；
2. 非 `unspecified` 的 `IsotopicAbundance` 在 `ShowAtomQuery=yes` 时显示
   查询标记 `I`；
3. 自动查询标记和普通原子编号与主标签的水平空隙约为
   `0.1875 × 主标签字号`；
4. 原子编号通常位于主标签右侧；右侧已有立体标记时，编号移到左侧；
5. CIP 标记显示为斜体括号文本，例如 `(R)`，而不是裸 `R`；
6. ChemDraw 将编号、查询和立体标记保存为附属于节点的
   `objecttag` 文本；导入后 ChemSema 保留其节点关联，语义编辑会同步
   已有显示对象，新建语义则由统一原子装饰渲染器绘制；
7. 直接原子自由基与点工具生成的符号不得重复绘点。

这些是按字段和对象关系实现的通用规则，不按文件名、样例 ID 或 CDXML
版本写特例。

## 往返与回归

- CDXML 和 CDX 均解析、写出上述字段；
- CCJS 只保存明确字段，不保存 `face` 或不透明位掩码；
- SVG、PNG、EMF 和 GUI 共用 `RenderPrimitive` 原子装饰；
- `command_engine` 覆盖 CCJS、CDXML、CDX 往返及装饰文字；
- `bond_tool` 覆盖点工具点击原子后有效自由基和 CDXML 导出；
- 右键菜单回归覆盖 Atom Properties 入口及 Radical 子菜单。
