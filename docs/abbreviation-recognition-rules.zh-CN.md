# Chemcore 缩写识别与基团展开计划

本文档记录下一步“化学标签缩写识别引擎”的支持范围。目标是先明确哪些标签是合法缩写、它们对应什么基团、以及将来在 engine 中如何存储和展开。

## 参考来源

本次阅读了两份本地开源源码：

- Ketcher：`/home/jiajun/ketcher-src`
  - Abbreviation Lookup 的候选来自元素、functional groups、templates、salts/solvents。
  - functional groups 由 `packages/ketcher-react/src/templates/fg.sdf` 载入。
  - 相关入口：
    - `packages/ketcher-react/src/script/ui/state/functionalGroups/index.ts`
    - `packages/ketcher-react/src/script/ui/dialog/AbbreviationLookup/hooks/useOptions.tsx`
    - `packages/ketcher-core/src/domain/entities/functionalGroup.ts`
- ChemCanvas：`/home/jiajun/chemcanvas-src`
  - 工具栏中有一组小型 functional group 清单。
  - 相关入口：
    - `chemcanvas/tools.py`
    - `grouptools_template`
    - `group_smiles_dict`

结论：Ketcher 是“模板/SUP SGroup 驱动”，缩写对应完整结构；ChemCanvas 是“小型 group 列表 + SMILES 映射”。Chemcore 不应只把缩写当纯文本，也不应把整个 Ketcher template library 全部视为端点缩写。

## Engine 目标模型

缩写标签应分成三类：

- `element`：元素标签，例如 `N`、`Cl`。
- `abbreviation`：合法缩写但暂不展开，至少参与显示、编辑和命中。
- `functional-group`：有结构定义的缩写，可展开为原子和键。

`functional-group` 还应继续分成两种来源：

- `terminal-fragment`：完整末端基团，例如 `CN`、`NO2`、`Et`、`Ts`、`Boc`。
- `composite-fragment`：由开放连接片段和末端片段拼出的基团，例如 `CO2Et = CO2 + Et`、`COOSO2Me = COO + SO2 + Me`。

建议每个可展开缩写记录：

- `label`：用户输入和显示的缩写，例如 `NO2`、`Boc`。
- `aliases`：等价输入，例如 `OMe` 和 `OCH3`。
- `name`：化学名或常用名。
- `formula`：人读的缩写结构。
- `anchorAtom`：外部键连接的原子。
- `rightAttachment`：组合解析时下一个片段连接的位置；末端片段为空。
- `expansion`：内部结构，使用 engine 原生的局部 atoms/bonds/attachments 表示；这是附加语义层，不替换用户原始绘制层。
- `components`：组合缩写的解析结果，例如 `["CO2", "Et"]`。
- `source`：来自 Ketcher、ChemCanvas 或 Chemcore 自定义确认。

## 第一阶段末端清单

第一阶段支持 Ketcher `fg.sdf` 中的 functional groups，再补入 ChemCanvas 已明确给出 SMILES 的小型 group。重复语义用 alias 归一，例如 `C2H5` 归到 `Et`，`Tos` 归到 `Ts`。其中能拆成组合片段的标签，不应只作为整词特例保存；整词表只作为兼容和校验来源。

| 缩写 | 别名 | 名称 | 展开基团 | 外部锚点 | 来源 |
| --- | --- | --- | --- | --- | --- |
| `R` | - | R group / generic substituent | 泛基团，不展开 | label 本身 | ChemCanvas |
| `Me` | `CH3` | methyl | `-CH3` | C | Ketcher |
| `Et` | `C2H5` | ethyl | `-CH2CH3` | first C | Ketcher |
| `Pr` | - | propyl | `-CH2CH2CH3` | first C | Ketcher |
| `iPr` | - | isopropyl | `-CH(CH3)2` | central C | Ketcher |
| `Bu` | - | butyl | `-CH2CH2CH2CH3` | first C | Ketcher |
| `iBu` | - | isobutyl | `-CH2CH(CH3)2` | first C | Ketcher |
| `sBu` | - | sec-butyl | `-CH(CH3)CH2CH3` | substituted C | Ketcher |
| `tBu` | - | tert-butyl | `-C(CH3)3` | central C | Ketcher |
| `Ph` | - | phenyl | `-C6H5` | ipso C | Ketcher + ChemCanvas |
| `PhCOOH` | - | benzoic acid substituent | Ketcher structure | ring/template atom | Ketcher |
| `Bn` | - | benzyl | `-CH2Ph` | benzylic C | Ketcher |
| `Bz` | - | benzoyl | `-C(=O)Ph` | carbonyl C | Ketcher |
| `Ac` | - | acetyl | `-C(=O)CH3` | carbonyl C | Ketcher |
| `Piv` | - | pivaloyl | `-C(=O)tBu` | carbonyl C | Ketcher |
| `CHO` | - | formyl | `-C(=O)H` | carbonyl C | ChemCanvas |
| `CN` | - | cyano | `-C#N` | C | Ketcher + ChemCanvas |
| `NCO` | - | isocyanato | `-N=C=O` | N | Ketcher |
| `NCS` | - | isothiocyanato | `-N=C=S` | N | Ketcher |
| `SCN` | - | thiocyanato | `-S-C#N` | S | Ketcher |
| `NO2` | - | nitro | `-N(=O)O` | N | Ketcher + ChemCanvas |
| `Ts` | `Tos` | tosyl | `-S(=O)2-p-Tol` | S | Ketcher |
| `Bs` | - | brosyl | `-S(=O)2-p-BrPh` | S | ChemCanvas derived from `OBs` |
| `Ms` | - | mesyl | `-S(=O)2CH3` | S | Ketcher |
| `Tf` | - | triflyl | `-S(=O)2CF3` | S | Ketcher |
| `SO3H` | - | sulfonic acid | `-S(=O)2OH` | S | Ketcher + ChemCanvas |
| `SO2H` | - | sulfinic acid style label | `-S(=O)OH` or Ketcher structure | S | Ketcher |
| `SO3` | - | sulfonate fragment | `-S(=O)3-` | S | Ketcher |
| `SO4H` | - | sulfate monoacid | `-O/SO3H` per Ketcher structure | S/O per template | Ketcher |
| `SO4` | - | sulfate fragment | Ketcher structure | S/O per template | Ketcher |
| `PO2` | - | phosphinyl/phosphoryl fragment | Ketcher structure | P | Ketcher |
| `PO3` | - | phosphate fragment | Ketcher structure | P | Ketcher |
| `PO3H2` | - | phosphonic acid | `-P(=O)(OH)2` | P | Ketcher |
| `PO4` | - | phosphate | Ketcher structure | P/O per template | Ketcher |
| `PO4H2` | - | phosphate acid form | Ketcher structure | P/O per template | Ketcher |
| `Boc` | - | tert-butyloxycarbonyl | `-C(=O)O-tBu` | carbonyl C | Ketcher |
| `Cbz` | `Z` later maybe | benzyloxycarbonyl | `-C(=O)OCH2Ph` | carbonyl C | Ketcher |
| `FMOC` | `Fmoc` | fluorenylmethoxycarbonyl | `-C(=O)OCH2-fluorenyl` | carbonyl C | Ketcher |
| `TMS` | - | trimethylsilyl | `-Si(CH3)3` | Si | Ketcher |
| `TBDMS` | - | tert-butyldimethylsilyl | `-Si(CH3)2tBu` | Si | Ketcher |
| `TBDPS` | - | tert-butyldiphenylsilyl | `-Si(Ph)2tBu` | Si | Ketcher |
| `CCl3` | - | trichloromethyl | `-CCl3` | C | Ketcher |
| `CF3` | - | trifluoromethyl | `-CF3` | C | Ketcher |
| `CPh3` | - | trityl | `-CPh3` | central C | Ketcher |
| `Cp` | - | cyclopentadienyl | Ketcher structure | ring atom / haptic later | Ketcher |
| `Cy` | - | cyclohexyl | `-C6H11` | ring C | Ketcher |
| `Mes` | - | mesityl | 2,4,6-trimethylphenyl | ipso C | Ketcher |
| `NHPh` | - | anilino | `-NHPh` | N | Ketcher |
| `Indole` | - | indolyl / indole template | Ketcher structure | template atom | Ketcher |
| `ster` | - | generic steric label | 不展开；保留标签 | label 本身 | Ketcher |

## 组合缩写语法

除了上面的末端表，Chemcore 应支持一组双连接或开放连接片段。它们本身不是完整末端基团；只有右侧继续接入末端片段，整个标签才算合法 functional group。

| 片段 | 别名 | 结构含义 | 左侧外部连接 | 右侧继续连接 | 示例 |
| --- | --- | --- | --- | --- | --- |
| `O` | - | ether / oxy linker, `-O-` | O | O | `OMe`、`OEt`、`OPh`、`OAc`、`OTs` |
| `CH2` | - | methylene linker, `-CH2-` | C | C | `CH2CH3`、`CH2CH2CH3` |
| `NH` | - | imino linker, `-NH-` | N | N | `NHTs`、`NHMe`、两键桥接 `NH` |
| `CO` | - | carbonyl linker, `-C(=O)-` | carbonyl C | carbonyl C | `COMe`、`COPh`、`COCl`、`CONH2` |
| `CO2` | `COO` | ester/carboxyl linker, `-C(=O)O-` | carbonyl C | O | `CO2Et`、`COOEt`、`COOPh`、`COOCN` |
| `OCO` | - | reverse ester linker, `-O-C(=O)-` | O | carbonyl C | `OCOEt`、`OCOPh` |
| `SO` | - | sulfinyl linker, `-S(=O)-` | S | S | `SOMe`、两键桥接 `SO` |
| `SO2` | - | sulfonyl linker, `-S(=O)2-` | S | S | `SO2Me`、`SO2Cl`、`SO2Ph` |

组合解析的终止片段包括：

- 常见烃基/芳基：`Me`、`Et`、`Pr`、`iPr`、`Bu`、`iBu`、`sBu`、`tBu`、`Ph`、`Bn`、`Cy`、`Mes`。
- 公式链终止：`CH3`，以及一个或多个 `CH2` 后接 `CH3`，例如 `CH2CH2CH3`。
- 原子或小端基：`H`、`F`、`Cl`、`Br`、`I`、`OH`、`NH2`、`CN`、`NO2`、`NCO`、`NCS`、`SCN`。
- 保护基或复杂末端：`Ts`、`Ms`、`Tf`、`Boc`、`Cbz`、`Fmoc`、`TMS`、`TBDMS`、`TBDPS` 等。

组合示例：

```text
CO2Et          -> CO2 + Et              -> -C(=O)OCH2CH3
CO2Boc         -> CO2 + Boc             -> -C(=O)O-Boc
COOEt          -> CO2 + Et              -> -C(=O)OCH2CH3
COOCH2CH2CH3   -> CO2 + CH2 + CH2 + Me  -> -C(=O)OCH2CH2CH3
COOSO2Me       -> CO2 + SO2 + Me        -> -C(=O)O-S(=O)2-CH3
COOCN          -> CO2 + CN              -> -C(=O)O-C#N
OCOEt          -> OCO + Et              -> -O-C(=O)CH2CH3
OAc            -> O + Ac                -> -O-C(=O)CH3
OBs            -> O + Bs                -> -O-S(=O)2-p-BrPh
SO2Me          -> SO2 + Me              -> -S(=O)2CH3
SO2Cl          -> SO2 + Cl              -> -S(=O)2Cl
```

有效性规则：

- 末端缩写只在恰好 1 根外部键时合法；0 根键、2 根键或更多连接时，末端缩写不能作为 functional group 识别。
- 在末端上下文里，开放片段不能独立成为完整 functional group。`CO`、`CO2`、`COO`、`OCO`、`SO2`、`CH2` 单独输入时只作为普通文本或未完成缩写处理。
- 解析用最长匹配。`CO2Et` 必须先命中 `CO2`，不能拆成 `CO` + `2Et`；`OCOEt` 必须先尝试 `OCO`，再尝试 `O`。
- `COO` 结构上归一到 `CO2`，但显示保留用户输入。`COOH` 可解析为 `COO + H`，canonical 可归到 `CO2H`。
- 整词表中的 `CO2Et`、`CO2Me`、`CO2tBu`、`COCl`、`COBr`、`CONH2`、`OAc`、`OTs` 等应能通过组合语法得到同一结构；如果整词模板和组合模板冲突，以来源模板为测试基准，修正规则而不是保留两个不同结果。
- 整词命中只确认“这是合法缩写”，不表示跳过组合解析。像 `CO2Et` 这种已在 Ketcher 表里的词，也应保存 `components = ["CO2", "Et"]`。
- 组合解析不做任意化学式解析。第一阶段只支持本节列出的片段、终止片段和重复 `CH2` 链。

## 两键桥接标签

识别必须带连接数上下文。上面的末端缩写只适用于恰好 1 根外部键；如果同一个节点已经连了 2 根键，末端缩写要标红。例如 `Boc`、`Ts`、`CN`、`NO2`、`CO2Et` 在两键节点上都不是合法桥接标签。

桥接缩写只在恰好 2 根外部键时合法。两键节点允许单独使用开放片段作为桥接标签：

```text
CO      -> -C(=O)-
CO2/COO -> -C(=O)O-
OCO     -> -O-C(=O)-
NH      -> -NH-
SO      -> -S(=O)-
SO2     -> -S(=O)2-
CH2     -> -CH2-
```

两键节点还允许 `N` 加一个末端取代基，表示取代氮桥：

```text
NMe  -> -N(Me)-
NTs  -> -N(Ts)-
NTos -> -N(Ts)-
NCl  -> -N(Cl)-
```

`N` 自身仍按元素标签处理，不进入缩写识别；如果两根单键连接，隐式氢规则可以把显示更新为 `NH`。

## Ketcher 完整 functional group 列表

`fg.sdf` 当前包含 62 个 functional groups：

```text
Ac, Bn, Boc, Bu, Bz, Cbz, C2H5, CCl3, CF3, CN, CO2Et, CO2H,
CO2Me, CONH2, CO2Pr, CO2tBu, Cp, CPh3, Cy, Et, FMOC, iBu,
Indole, iPr, Me, Mes, Ms, NCO, NCS, NHPh, NO2, OAc, OCF3,
OCN, OEt, OMe, Ph, PhCOOH, Piv, PO2, PO3, PO3H2, PO4,
PO4H2, Pr, sBu, SCN, SO2, SO2Cl, SO2H, SO3, SO3H, SO4,
SO4H, ster, TBDMS, TBDPS, tBu, Tf, TMS, Tos, Ts
```

注意 `C2H5` 与 `Et` 语义重复；`Tos` 与 `Ts` 语义重复；`FMOC` 应接受用户常见输入 `Fmoc`，内部 canonical label 可统一成 `Fmoc` 或保持 Ketcher 原样 `FMOC`，需要 UI 决策。
（作者评：统一用Et，但可以识别C2H5.其余也一样，用Ts,Fmoc）

## ChemCanvas 小型 group 列表

ChemCanvas 工具栏直接列出的 group：

```text
R, Ph, NO2, CN, CHO, COOH, CONH2, COCH3, COCl, COBr,
OCH3, OEt, OAc, SO3H, OTs, OBs
```

其中 `group_smiles_dict` 给出明确结构：

```text
CHO      C=O
CN       C#N
COBr     C(=O)Br
COCH3    C(=O)C
COCl     C(=O)Cl
CONH2    C(=O)N
COOH     C(O=)O
NO2      N(=O)O
OAc      OC(=O)C
OBs      OS(=O)(=O)C1=CC=C(Br)C=C1
OCH3     OC
OEt      OCC
OTs      OS(=O)(=O)C1=CC=C(C)C=C1
Ph       C1=CC=CC=C1
SO3H     S(=O)(=O)O
```

ChemCanvas 的 SMILES 没有显式 attachment atom 字段；对 Chemcore 来说应按“字符串最左侧原子为外部连接锚点”解释，除非后续用专门模板覆盖。

## 不进入端点缩写的 Ketcher 数据

Ketcher Abbreviation Lookup 还会搜索：

- 元素周期表所有元素。
- `library.sdf` 模板库：糖、环、杂环、氨基酸、核苷酸、冠醚、金属有机模板等。
- `salts-and-solvents.sdf`：溶剂、盐、缓冲盐等，例如 `DMF`、`DMSO`、`THF`、`DCM`、`NaCl`。

这些不应在第一阶段自动作为端点缩写展开。理由：

- 模板库很多是多锚点或整分子模板，不是单一端点基团。
- 盐和溶剂通常是独立对象，不是从一个原子拖出的 substituent。
- 元素已经由元素标签系统处理，不应和 functional group 缩写混在同一个解析结果里。

后续如果需要，可把这些接入“模板搜索/插入”而不是“端点缩写识别”。

## 识别规则

初始规则建议：

- 匹配大小写敏感，但为常见写法提供 alias：
  - `Fmoc` -> `FMOC`
  - `Tos` -> `Ts`
  - `OCH3` -> `OMe`
  - `COOH` -> `CO2H`
  - `C2H5` -> `Et`
- 优先级：
  1. 元素符号精确匹配。
  2. functional group canonical label 精确匹配，用于确认合法性和加载来源模板。
  3. alias 精确匹配，把用户输入归一到 canonical。
  4. 组合缩写解析；如果 canonical 或 alias 命中的标签可组合，仍记录 `components`。
  5. 普通文本或未知 superatom，不展开。
- 不做前缀匹配。`B` 不能误识别成 `Boc`，`C` 不能误识别成 `CN`。
- 不把带电荷、同位素、点号或括号表达式纳入第一阶段，例如 `NH4+`、`DMSO·H2O`。

## 展开行为

当用户输入合法 functional group 并确认：

- 文档中保留 collapsed label，显示用户输入的缩写。
- 同时在节点和 label 的 `meta.labelRecognition` 中保存识别结果，包括 `status`、`canonicalLabel`、`components`、`formula`、`anchorAtom` 和 `expansion`。
- `expansion` 使用 `chemcore.functionalGroupExpansion.v1`，只表达拓扑语义，不包含 label 坐标、glyph polygon、box 或字体样式。
- `expansion.atoms[].id` 是局部 id，只在 expansion 内有效，不污染主分子图的 `nodes[].id`。
- `expansion.attachments` 明确外部连接点：末端基团使用 `external`，两键桥接使用 `left`/`right`。
- `expansion.complete == false` 表示该标签已合法识别，但当前只有局部或占位拓扑；读取方仍可使用原始 label 和 `components`。
- 外部键连接到 `anchorAtom`。
- 如果用户继续从缩写标签拖键，锚点应落在 functional group 的 attachment atom，而不是标签文本中心。
- 用户执行 expand 时，把 collapsed label 替换成 expansion fragment。
- 用户执行 collapse 时，如果子图完整匹配某个定义，可以折回缩写。

第一阶段实现 collapsed label + metadata + expansion + anchor；真正把主图替换为展开子图的 expand/collapse 命令可作为下一步。

示例：

```json
{
  "status": "recognized",
  "label": "CO2Et",
  "canonicalLabel": "CO2Et",
  "groupKind": "composite-fragment",
  "components": [
    { "label": "CO2" },
    { "label": "Et" }
  ],
  "expansion": {
    "schema": "chemcore.functionalGroupExpansion.v1",
    "connectionKind": "terminal",
    "complete": true,
    "atoms": [
      { "id": "c1", "element": "C", "numHydrogens": 0 },
      { "id": "o1", "element": "O", "numHydrogens": 0 },
      { "id": "o2", "element": "O", "numHydrogens": 0 },
      { "id": "c2", "element": "C", "numHydrogens": 2 },
      { "id": "c3", "element": "C", "numHydrogens": 3 }
    ],
    "bonds": [
      { "begin": "c1", "end": "o1", "order": 2 },
      { "begin": "c1", "end": "o2", "order": 1 },
      { "begin": "o2", "end": "c2", "order": 1 },
      { "begin": "c2", "end": "c3", "order": 1 }
    ],
    "attachments": [
      { "role": "external", "atomId": "c1" }
    ]
  }
}
```

编辑已有端点标签时，不管提交后的标签因为连接方向显示为靠左、上下堆叠或反向，编辑中的文本框都应从锚点按普通左到右文本框展开；提交后再按当前连接方向重新排版。

当用户输入不能识别的端点标签：

- 仍保留用户输入的文本，不静默改写。
- `meta.labelRecognition.status` 标记为 `invalid`。
- 渲染时显示红色矩形文本框，使用 label bbox，而不是 glyph 裁剪轮廓。

## 和隐式氢的关系

functional group 缩写不走元素隐式氢规则。比如：

- `NO2` 是 nitro group，不是 N 原子加两个 O 文本。
- `CN` 是 cyano group，不是 C/N 两个普通字符。
- `Boc`、`Ts`、`Fmoc` 是保护基/取代基，不能按元素串计算隐式氢。
- `COOEt`、`COOSO2Me`、`CH2CH2CH3` 先按组合缩写解析，不按 C/O/S/H 的元素串逐字加氢。

因此缩写识别应该发生在简单元素隐式氢之前；一旦命中 functional group，就停止元素加氢。
