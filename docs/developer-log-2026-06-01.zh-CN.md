# Chemcore 开发者日志 / 阶段回顾 - 2026-06-01

作者：Codex

时间范围：2026-05-12 00:00 至 2026-06-01 23:59，Asia/Shanghai

说明：本日志覆盖当前仓库在上一份开发者日志之后的全部可见提交，不只记录单日工作，还额外补充阶段性项目评估、技术难度分析、项目意义分析，以及与主要竞品的对比判断。

基线提交：`970c4fc Add May 11 developer log`

工作目录：`D:\Projects\chemcore`

## 总结

这一阶段是 `chemcore` 从“已经有共享化学内核和桌面/浏览器宿主”继续向“真正能承受 Office / Word 场景、并具备可持续调优能力的产品底座”推进的关键窗口。可见历史里一共新增了 `204` 个提交，触达 `137` 个文件，总体差异约为 `33017` 行新增、`1260` 行删除。提交密度高度集中在 `2026-05-15` 到 `2026-05-18`，说明这段时间主要在进行高强度的 EMF / Office 预览攻坚与归因分析；`2026-06-01` 的 3 个提交则代表这一轮从“研究/试验”向“稳定化/正式工具链”收口。

如果要用一句话概括本阶段成果，那么就是：

- Office / Word 方向不再停留在“能粘贴、能显示”的表层成功，而是建立起了围绕 `Chemcore document -> render primitives -> EMF / OLE / Word` 的完整技术闭环。
- 围绕 EMF 文本、预览 frame、same-shell 偏差、attached-label replay、invalid marker、PPT/Word 同壳对比等问题，仓库内沉淀出一整套可复现、可批量分析、可回归的调研工具链。
- 到阶段末尾，项目已经不只是“研究为什么不像 ChemDraw”，而是开始系统清掉一部分历史试验分支，把浏览器对比工具和 Word roundtrip 验证正式收进仓库支持面。

这份日志会先按阶段回顾代码与方法论的变化，再给出全仓库进度、技术难度、项目意义和竞品对比的综合判断。

## 提交轮廓

当前范围内的提交按日期分布如下：

```text
2026-05-12  9
2026-05-13  2
2026-05-14 17
2026-05-15 29
2026-05-16 76
2026-05-17 43
2026-05-18 25
2026-06-01  3
```

这个分布有两个明显特征：

1. `2026-05-16` 是调研强度最高的一天，大量提交都围绕 packaged EMF 文本、预览 bounds、Word shell、same-shell frame 差异以及分析脚本展开。
2. `2026-05-19` 到 `2026-05-31` 在当前可见历史里没有新提交，因此本阶段可以自然切分为：
   - `05-12` 到 `05-18`：Office / EMF 攻坚与归因期
   - `06-01`：稳定化、清理和回归固化期

本阶段差异最大的文件和区域包括：

```text
apps/chemcore-office/src/windows_office.rs
apps/chemcore-office/src/windows_office/emf_preview.rs
apps/chemcore-office/src/windows_office/emf_preview/renderer.rs

crates/chemcore-engine/src/cdxml.rs
crates/chemcore-engine/src/cdxml/text_runs.rs
crates/chemcore-engine/src/engine/text_edit/labels.rs
crates/chemcore-engine/src/engine/text_edit/runs.rs
crates/chemcore-engine/tests/render_document.rs

viewer/app.js
viewer/document_flow.js
viewer/styles.css

docs/emf-text-investigation.zh-CN.md
scripts/*
```

其中最能说明阶段特征的不是某一个单独文件，而是三个层次同时演进：

- 共享内核：CDXML 导入、文本 run 语义、bond / label / render primitive 规则继续变厚。
- Office 适配层：EMF / OLE / Word 预览链路持续推深，并最终形成更稳的 roundtrip 验证。
- 调研工具层：大量脚本用于把“看起来不一样”拆成 record、region、text box、component family、phase bucket、policy sweep 等可分析对象。

## 阶段一：Office 预览、文本度量与 CDXML 几何收紧（2026-05-12 到 2026-05-14）

这一阶段的主题是把之前已经跑起来的 Office / EMF 预览链，从“基本能画”收紧到“线宽、文字、导入几何都更接近 ChemDraw / Word 实际显示”。

### 1. Office 预览与共享 glyph 处理

`bb14221 Improve Office preview and shared glyph handling` 是本段的开端。它说明 Office 预览与共享 glyph kernel 的边界被进一步打通：Office 路径不再只是临时拼装图形，而是更认真地消费内核已有的 glyph / label / primitive 结果。

在这之后，连续几条提交把几个关键问题逐个压实：

- `eb632bd Improve Office EMF antialiasing and hashed wedges`
- `71fc6ae Align Office EMF text run metrics`
- `344cf02 Align CDXML text metrics with ChemDraw`
- `5247b4b Align automatic double bond placement`
- `80d5b00 Preserve imported CDXML atom label geometry`
- `2eae21e Derive CDXML import clipping from file metrics`

这些改动组合起来，意味着项目开始同时追两件事：

1. 文本与键的“显示像不像”
2. 导入后几何与原始文件语义“保不保真”

这不是小修小补。因为一旦 CDXML 导入时的 label geometry、clipping、text metrics 和 bond placement 都开始被当作严格输入约束，那么后续不论是 viewer 画布、WASM、还是 Office EMF，都会被迫共享同一套更严格的几何前提。

### 2. Bond 预览缩放与 OLE 尺寸问题

`2026-05-14` 这一组提交的特征非常鲜明：大量工作都围绕 bond 线宽、EMF page transform、OOXML / OLE 原始尺寸和显示尺寸分离、以及预览 frame 来源而展开。

代表性提交包括：

- `74e0264 Refine Office bond preview scaling behavior`
- `aeceffd Restore EMF bond preview width to ChemDraw range`
- `5c1008e Align EMF bond page transform with ChemDraw scaling`
- `cd24905 Align preview bond width with kernel stroke width`
- `dba428b Extend side-double outer line on unoccupied side`
- `610401f Carry center-double extension into pen preview`
- `e8246ff Fit OOXML preview size to page body width`
- `5df96f2 Separate Word original and display OLE sizing`
- `41d82ef Use SVG canvas bounds for EMF preview frame`
- `7191f01 Add right-side preview padding for Word text`

这说明项目已经深入到一个很少有编辑器会真的碰的层面：不仅要决定“文档内容如何画”，还要决定“Word 里的嵌入对象原始自然尺寸”和“Word 最终显示尺寸”如何分离、如何与 page body width、preview frame、文本 padding 一起工作。

从产品角度看，这很重要，因为 Office 用户最终感知到的不是内部 JSON 是否优雅，而是：

- 贴进 Word 后对象看起来是否太瘦、太粗、太紧、太松
- 双键、中心双键、粗箭头的几何是否像熟悉的 ChemDraw
- 文本是否在右侧被截、被挤、或偏出预览框

## 阶段二：packaged EMF 文本、preview frame 和 same-shell 体系化调研（2026-05-15 到 2026-05-16）

这一阶段是本轮历史里技术密度最高、试验最密集、方法论最清晰的一段。核心不是“加功能”，而是建立一整套定位 EMF / Word 差异的科学工作流。

### 1. packaged EMF 文本上下文问题被正式拆开

从 `ccc8bd1 Align Office preview canvas and live text output` 开始，到

- `0b11408 Improve EMF packaged text alignment`
- `61340e7 Improve packaged EMF text rendering hint`
- `97abbce Add EMF text investigation tooling`
- `f243012 Add EMF object history analysis tooling`
- `65ab888 Add GDI+ text fallback harness`
- `4e37f64 Add EMF packaged text comparison tooling`
- `272b2bf Add packaged DrawString comparison tooling`
- `2522481 Add packaged EMF text experiment toggles`
- `50f7486 Add EMF rendering analysis toggles and tools`

为止，项目实际上建立了一个完整研究框架：

```text
真实 Word / ChemDraw 结果
  -> 提取 same-shell / packaged EMF / PNG
  -> 解析 EMF record 与对象历史
  -> 按文本行、文本框、文字 fallback、DrawString / driver-string / point-mode 细分
  -> 做实验开关与 sweep
  -> 把结论固化进 docs/emf-text-investigation.zh-CN.md
```

这段时间里有大量“experiment -> document -> revert”的提交，例如：

- packaged DrawString / driver-string / zero-layout / emSize clamp / top bias / text-state reset
- anti-alias mode / pixel-offset / width mode / zero-layout 等 toggle

它们表面上看像是试错，但从工程意义上说，这其实是在把“EMF 文本为什么不对”系统性剥离为：

- 文字内容本身是否错
- 行内 run 切分是否错
- baseline / top bounds 是否错
- GDI+ mode 是否错
- same-shell 文本上下文是否改变了 fallback record 链

这类调查成本很高，但它最终带来的价值也很大：后面的 Office 调优不再靠肉眼猜，而是靠固定脚本、固定样本、固定报告反复验证。

### 2. Preview bounds / frame / shell identity 被拉成独立问题

`2026-05-16` 的另一条大主线是 preview 几何与 same-shell frame 行为：

- `6642584 Add preview source padding investigation override`
- `c2b1eab Add preview source bounds mode override`
- `0845c23 Add extra preview source bounds experiment modes`
- `4213135 Add preview source side override for EMF analysis`
- `014d48c Add EMF frame override analysis hook`
- `497f552 Add Word frame comparison tooling`
- `80a5656 Add docx object size patch tooling`
- `7c6ddf3 Document same-shell EMF frame findings`
- `bde8bd6 Document Word preview shell interaction findings`
- `dc3e6b5 Document Word shell identity key findings`
- `20e16e8 Analyze region-wise and centered-frame behavior`
- `1bb910d Fit centered frame family pads`

这组提交说明：项目已经明确意识到，“Word 里看起来不一样”很多时候不是渲染 primitive 本身错，而是外层 preview source bounds、frame bounds、page shell、CopyAsPicture 壳层尺寸参与了误差。

这是一种很成熟的工程判断。因为它把问题从“是不是字体、是不是 SVG、是不是 Chemcore 画歪了”扩展到了：

- 当前 preview 用的是哪种 bounds
- 哪一侧来自 source，哪一侧来自 svg frame
- Word 当前的 shell / wrapper 是否影响最终 CopyAsPicture
- 当前对象是否处在 same-shell 比较环境里

如果没有这一层拆分，Office 集成很容易永远陷在“局部调整但整体不稳”的循环里。

## 阶段三：attached-label replay、invalid marker、PPT generalization（2026-05-17 到 2026-05-18）

这两天最核心的工作，是把最难对齐的一类对象集中拿下来：分子附近的 attached labels、knockout、same-shell 文字重放，以及这些差异如何抽象成 family、phase、policy。

### 1. 从“现象”升级到“family / predicate / atlas”

这一阶段的提交不再只是“试一个值”，而是明确地开始给现象命名、分类、建 predicate，并做搜索：

- `0f16f7d Analyze molecule label residuals in Word replay`
- `3c4023f Analyze single-label replay families`
- `e97d862 Identify attached-group replay family pattern`
- `9b793ad Analyze attached-label local geometry family`
- `d58d0e8 Model attached label replay families`
- `392ba52 Probe attached-label replay font-scale sensitivity`
- `48c5566 Analyze attached-label phase-sensitive replay buckets`
- `32e8322 Analyze attached label page-space phase`
- `213f9a4 Search attached-label phase-band policy`
- `c16bf6b Probe attached-label y subfamilies`
- `6cdf597 Formalize attached-label microfamily predicates`
- `edffe1a Formalize attached-label top predicates`
- `bd8a127 Formalize replay stack predicates`
- `aa070a7 Formalize residual attached-label y predicates`

配套脚本大量出现：

```text
scripts/probe-attached-actions-on-stack.py
scripts/run-attached-x-atlas.py
scripts/run-attached-y-atlas.py
scripts/run-attached-top-atlas.py
scripts/run-attached-fs-atlas.py
scripts/run-attached-hint-atlas.py
scripts/search-attached-phase-policy.py
scripts/search-attached-top-policy.py
scripts/search-attached-local-policy.py
scripts/fit-attached-replay-family.py
```

这说明团队已经不再满足于“把一个标签调对”，而是尝试把 attached-label replay 变成可以量化和批量决策的系统问题。对于长期维护来说，这是非常重要的跃迁。

### 2. Knockout、invalid marker、DocumentText 被正式纳入归因

`d691179 Analyze DocumentKnockout visibility in Office preview` 和 `4a42730 Analyze text and knockout isolation in Office preview` 把另一个关键点拆明白了：Word 里的偏差并不全来自文字本身，`DocumentKnockout` 的存在与否、同层图形的叠放顺序、invalid marker 是否参与预览，也会影响最终肉眼观感。

随后出现：

- `9a3d377 Hide DocumentKnockout in Office preview`
- `b7645df Hide invalid markers in Office preview`

这意味着项目已经做出更产品化的判断：某些内部调试/校验性图元不该进入正式 Office 预览，否则它们会污染用户视角和 same-shell 对比。

### 3. 从 Word 扩展到 PPT same-shell generalization

这两天的另一个亮点，是把同样的方法推到了 PPT：

- `7d0f6a9 Add PPT ChemDraw same-shell generalization harness`
- `022331a Document PPT same-shell frame generalization findings`
- `d8216f9 Analyze PPT same-shell EMF record families`
- `71084e9 Add attached-label family policy experiments`
- `127950f Add parallel PPT label policy runner`

这一步很有意义，因为它证明项目在做的不是“只为一个 Word 样例凑效果”，而是在尝试验证同一套 Office preview / attached-label / frame policy 能否跨宿主 generalize。

如果一个策略只能在某个 Word 样本上工作，那它价值有限；如果它能在 Word、PPT、same-shell / cross-shell 上都稳定，那就更像真正的渲染规则，而不是一次性的 patch。

## 阶段四：稳定化、清理和正式回归固化（2026-06-01）

这一阶段虽然只有 3 个提交，但重要性不低，因为它标志着本轮开始从“研究积累”向“稳定产物”转。

### 1. Word / OLE 工作流稳定化

`dfd14c7 Stabilize Word EMF workflow and prune investigation artifacts` 把一批之前在工作区里已经验证过的行为正式收进去，包括但不限于：

- OLE / clipboard 路径的稳定化
- Office payload fallback 的补全
- Word / Office 预览链的进一步收口
- viewer 与桌面端文档拖入 / 打开的一致性修复
- group children 渲染与选择范围问题修复
- 一批 CDXML 标签对齐、`face`、下标、单字符 label 锚点问题修复
- 浏览器脚本统一通过系统 Edge / Playwright helper 复用

这意味着 6 月 1 日的工作不再是单纯写分析脚本，而是在把分析阶段识别出来的关键稳定性问题正式编码。

### 2. 调试脚本进入支持面

`efab3c3 Promote browser comparison scripts to supported repo tools` 把一批浏览器截图 / 像素比较 / 回归脚本挂入 `package.json`，不再只是散落在 `scripts/` 的临时工具。这是个很小但很重要的信号：一旦脚本进入正式命令面，它们就从“个人调试资产”变成了“仓库支持的工程工具”。

### 3. 裁掉一批历史 EMF replay 试验开关

`f47c030 Prune EMF replay experiments and add Word roundtrip validation` 继续做了两件事：

- 清掉 `renderer.rs` 里一批已经不再参与默认行为的 attached-label replay 试验分支
- 新增 `scripts/validate-word-ole-roundtrip.ps1`，并挂到 `package.json` 的 `office:validate-roundtrip`

这条 roundtrip 脚本的价值在于：它把之前相对手工的 Word 长链验证固化成一条可复跑的脚本链：

```text
cdxml
  -> cdxml_to_clipboard_payload
  -> chemcore-office --write-word-docx-payload
  -> Word 打开
  -> Save
  -> 关闭
  -> 重开
  -> CopyAsPicture 导出 PNG
```

这一步说明项目已经不满足于“某次手工粘贴看起来不错”，而是开始把发行前真正需要的链路做成回归工具。

## 关键代码与工具演化

这段时间里，仓库不是只新增了几个脚本，而是形成了三个层次互相支撑的结构：

### 1. 内核层继续做厚

`crates/chemcore-engine` 在本阶段并没有停下，只做 Office 附庸。恰恰相反，它继续在几个方向上变厚：

- `cdxml.rs`、`cdxml/import_objects.rs`、`cdxml/text_runs.rs`：导入、标签 face、文本 run、几何保真
- `render.rs`、`render_bonds.rs`、`render_objects/*`、`render_svg.rs`：图元与边界的精修
- `engine/text_edit/labels.rs`、`engine/text_edit/runs.rs`：文本规则与 label display runs
- `engine/clipboard.rs`：Office / OLE 需要的文档剪贴板语义
- `tests/render_document.rs`：大量回归样本直接在内核层固化

这很关键，因为它保证了 Office / Word 不是绕过内核单独做一套“近似图”，而是始终被共享核心牵住。

### 2. Office 适配层变成真正的子系统

从文件规模和提交密度看，`apps/chemcore-office` 已经不是一个薄薄的“导出工具”，而是一个真正的 Windows Office / OLE 适配子系统。它负责：

- OLE object / storage / clipboard formats
- Word OOXML package 生成
- preview bounds / frame / extent 计算
- EMF / WMF / OlePres bytes 生成
- Word roundtrip 验证支持

更重要的是，它现在已经拥有一套成体系的方法来证明自己，而不是只能靠人工观察。

### 3. 工具层从零散脚本变成“研究基础设施”

本阶段新增和扩展的脚本数量非常多。它们共同构成了一个很不常见、但非常有价值的能力：可以对 Word / EMF / ChemDraw 的差异做系统化归因。

这类工具的价值往往被低估。实际上，很多复杂编辑器卡死在“没有好的对比工具”。而这里已经开始拥有：

- record-level 比较
- png / region / label-box / text-box / hotspot 比较
- same-shell / PPT / Word generalization 比较
- policy matrix / sweep / atlas / search
- docx patch / object size patch / CopyAsPicture 导出

这说明仓库正在长出一种很难得的能力：它不仅能开发功能，还能开发“理解自己偏差来源”的仪器。

## 阶段性全仓库开发进度评价

如果从整个仓库而不是单条 Office 线来看，我会把当前阶段结束时的项目完成度评为：

```text
整体完成度：70% - 75%
```

这个判断来自以下几个维度。

### 1. 共享内核：80% 以上

优点：

- 已经不是玩具级核心，而是有文档模型、编辑命令、render primitives、CDXML 路径、测试面的共享 Rust 内核。
- 文本、bond、label、abbreviation、glyph、CDXML import/export 都明显进入深水区。
- 大量行为开始以回归测试而不是“记忆”固化。

不足：

- 广谱兼容性和复杂文档覆盖面仍需继续压样本。
- 一些细节语义仍在通过实际文档不断修正。

### 2. 浏览器 / viewer：75% 左右

优点：

- viewer 已经是一个能承接真实编辑工作的宿主，而不是纯展示页。
- 它和 Wasm / engine 的边界比项目早期清晰得多。
- 拖入、组件选择、label 工具链、回归脚本都逐渐稳定。

不足：

- UI 产品化和异常体验还不算完成。
- 一些边缘交互仍然要靠实际文档推动修复。

### 3. Windows 桌面端：75% 左右

优点：

- 通过 Tauri 和 desktop service，项目已经能形成真正的桌面编辑体验。
- 本地文件链、会话链、剪贴板链逐渐闭合。

不足：

- 与 Office / chemcore-office 的进程关系、二进制锁竞争、构建时文件占用等工程问题还没有完全消失。
- 发行级安装、升级、错误恢复等还需要后续产品化。

### 4. Office / Word / OLE：85% 左右

这是当前最亮的部分之一。

优点：

- OLE 注册、嵌入、预览、Word 文档生成、粘贴与 roundtrip 已经不再只是概念验证。
- 项目已经具备一条可复跑、可诊断、可持续收敛的 Word / EMF 主链。

不足：

- 仍有 build / process lock / shell-specific 的工程毛刺。
- 还需要更多样本集和更系统的发行前回归。

### 5. 构建、发布与回归工程：55% - 65%

这是当前最明显的短板。

优点：

- 常用命令、Wasm 构建、桌面脚本、Office 自检、回归脚本都在增长。

不足：

- `npm run verify` 仍可能被外部进程占用 `chemcore-office.exe` 等问题打断。
- 说明“功能做出来了”和“稳定可交付”之间，还存在一层发行工程化工作没有完全收口。

## 技术难度评价

如果只从工程技术难度看，这个项目我会给出：

```text
技术难度：9 / 10
```

原因不在于代码量大，而在于这里叠加了多条本来就各自很难的线：

### 1. 化学编辑器内核本身就难

这不是通用绘图板，而是要处理：

- bond 语义
- attached label 语义
- abbreviations / formula-like labels
- CDXML import/export
- glyph / text / rendering rule
- 组件选择、剪贴板、命令历史

单做其中一项都不轻松，更不用说把它们放进共享核心。

### 2. 多宿主一致性非常难

项目目标不是“先做 Web demo，再重写桌面版”，而是让同一内核服务：

- 浏览器 / Wasm
- Windows 桌面
- Office / OLE / Word

这意味着很多系统只做一次的决定，这里都要在三种宿主里承受。

### 3. Word / OLE / EMF 是典型高摩擦领域

Office 集成最困难的点之一是：很多问题并不是“逻辑错了”，而是落在：

- shell / wrapper
- preview frame
- object sizing
- same-shell / cross-shell 行为
- GDI+ / EMF 文字 fallback

这些问题通常没有干净官方答案，只能靠实验、归因、对照和大量样本推进。

### 4. 你们做的是“兼容 + 自主实现”的双重难度

项目既要保有自己的共享内核设计，又要在 Word / ChemDraw 生态里表现得足够接近。这意味着它不是纯创新产品，也不是纯逆向兼容产品，而是两种难度叠在一起。

## 项目意义分析

这个项目的意义，不应该只理解为“又一个化学绘图软件”。

它真正有价值的地方在于，它在尝试把“化学文档能力”做成一个统一、可控、可复用的基础设施。

### 1. 统一核心带来的平台意义

一旦共享内核稳定，项目获得的不只是一个编辑器，而是一套可以复用到多个场景的底座：

- 浏览器编辑器
- Windows 桌面应用
- Office 可编辑对象
- 批处理 / 导入导出工具
- 后续可能的文档审阅、结构分析、结构服务接口

这和“做一个孤立桌面程序”是完全不同的战略价值。

### 2. 可控性很高

在化学文档这个领域，很多关键能力长期被封闭商业工具掌握。`chemcore` 的意义在于：

- 关键语义在自己手里
- 关键渲染路径在自己手里
- Word / OLE 输出链也开始在自己手里

对任何需要深定制或长期演化的团队来说，这种可控性非常重要。

### 3. 它在建立“化学文档工程学”

本阶段最让人印象深刻的一点，不是某个按钮，而是仓库开始形成一种方法：

- 把渲染偏差拆成可度量对象
- 把 Word / ChemDraw 差异写成可复跑脚本
- 把现象整理成 family / predicate / atlas / policy

这意味着项目不只是产出代码，也在产出一整套如何开发、验证、比较化学文档系统的方法论。

## 与主要竞品的对比优势

下面的判断不是“谁功能最多”的简单比较，而是从项目定位、技术架构和可持续性角度来看的。

### 1. 对 ChemDraw / Signals ChemDraw

ChemDraw 目前仍然是行业基准，尤其在：

- 成熟度
- Office 深度集成历史
- 用户习惯
- 样本覆盖面
- 生态位置

上都很强。

`chemcore` 当前还不能说在总功能量或成熟度上超过 ChemDraw，但它有几个非常明确的潜在优势：

- 统一共享内核，而不是历史产品堆叠
- Web / desktop / Office 三端共享同一套核心逻辑
- 可以直接掌控 Word / OLE / EMF 链路
- 更适合作为自研平台底座，而不是固定成品

换句话说，ChemDraw 更像“成熟终端产品”；`chemcore` 更像“下一代可控化学文档平台底座”。

### 2. 对 Marvin / MarvinSketch

Marvin 的强项在于老牌化学编辑、企业场景和既有化学能力积累。

`chemcore` 相比 Marvin 的优势不在“现成功能更全”，而在：

- 架构统一度更高
- 更适合深度定制
- Rust 核心更便于长期控制和跨宿主复用

但 Marvin 的产品成熟度和企业级稳定性，目前仍然更强。

### 3. 对 Ketcher

Ketcher 的优势非常鲜明：

- 开源
- Web-first
- 易嵌入
- 前端集成经验丰富

如果目标是“在网页里嵌一个结构编辑器”，Ketcher 依然很强。

`chemcore` 的差异化优势在于：

- 不是单纯 Web 编辑器，而是从一开始就把桌面和 Office 放进同一个架构目标
- 对 Word / OLE / EMF 的投入深很多
- CDXML / 化学文档兼容路径明显更重

所以二者不是简单替代关系，更像是不同战略方向。

### 4. 对 BIOVIA Draw

BIOVIA Draw 的强项在于企业平台整合、生物/化学工作流和大厂生态。

`chemcore` 当前的优势主要是：

- 轻量、可控
- 统一内核
- 更适合作为自有系统能力层

它的短板则在于企业平台宽度和成熟生态，目前还远不如 BIOVIA。

### 5. 综合优势总结

如果把所有竞品放在一起看，我认为 `chemcore` 最核心的潜在优势不是“功能更多”，而是：

```text
统一 Rust 内核
+ 多宿主一致性
+ 可控的 Word / OLE / EMF 输出链
+ 面向长期定制和平台化演化的结构
```

这套优势短期不一定转化成市场上最耀眼的产品，但一旦继续做成，会是非常强的战略资产。

## 当前发行距离判断

从本阶段结束时的状态看：

- 如果目标是 Windows 内部预览版、alpha、dogfood：已经很接近。
- 如果目标是对外稳定版：还差最后 1 到 3 轮系统性收口。

主要差距已经不是“有没有核心能力”，而是：

- build / verify / 打包链的稳定性
- Office / Word 长链的更大样本回归
- 剩余历史策略分支的进一步收口
- 边角错误处理、恢复路径和发行工程化

换句话说，项目最难的路线已经跑通了，但“稳定、可重复、可维护地交付出去”这件事还需要继续打磨。

## 当前阶段的结论

这是一个技术难度非常高、但方向也非常清晰的阶段。

从仓库视角看，`2026-05-12` 到 `2026-06-01` 不是平铺直叙地加功能，而是完成了三件更关键的事：

1. 把 Office / EMF / Word 这条最难链路做成了可分析、可比较、可验证的问题。
2. 把很多原本只能靠人工经验判断的差异，转化成 family、policy、sweep、roundtrip 这类可工程化对象。
3. 在阶段末尾开始真正清理和固化，把“研究资产”逐渐收进正式仓库支持面。

如果后续能继续沿着“稳定化、回归化、发布工程化”推进，那么这一阶段回头看，很可能会被认为是 `chemcore` 从“有潜力的共享内核项目”走向“有机会发行的化学文档平台”的转折段。
