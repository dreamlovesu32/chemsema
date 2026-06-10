# Chemcore 项目意义与完成度评价

日期：2026-06-10

## 结论摘要

Chemcore 不是普通的化学画板前端，也不是单纯的 SVG 绘图工具。它更接近一个 ChemDraw 类化学文档系统的基础内核：用 Rust 统一承载文档模型、编辑行为、渲染几何、格式导入导出、桌面端能力和 Office/OLE 集成。

从仓库现状看，项目已经超过早期原型。核心架构方向正确，工程投入明显，测试和文档也有一定规模。但它还没有达到稳定产品级，更准确的阶段判断是：

```text
后期工程原型 / alpha 到 pre-beta 之间
```

核心能力已经站起来了，后续重点不应该只是继续堆功能，而是收紧架构边界、扩大格式回归测试、降低大文件复杂度，并把真实用户工作流跑稳。

## 项目意义

### 1. 它在解决一个真实且困难的问题

化学结构编辑不是普通绘图。一个可靠的化学文档工具需要同时处理：

- 分子结构语义
- 键、原子、标签、隐式氢、价态规则
- 文本、箭头、括号、图形、轨道、模板
- ChemDraw/CDXML/CDX 兼容
- SDF 等结构格式导入导出
- SVG、EMF、PDF 等渲染输出
- 浏览器端、桌面端、Office/OLE 嵌入
- 剪贴板、多格式 fallback、可编辑对象

这些问题如果散落在前端里，长期一定会难以维护。Chemcore 当前最重要的价值，是把这些语义逐步收到 Rust engine 里，并让浏览器、桌面端和 Office 层复用同一套核心行为。

### 2. 它的方向不是“复制 ChemDraw UI”，而是建立可复用内核

仓库文档和代码都体现了一个明确方向：Chemcore 的核心资产不是第一版 UI，而是文档模型和引擎。

这个方向是正确的。真正有长期价值的是：

- 稳定文档格式
- 可迁移的对象模型
- 可测试的编辑命令
- 后端无关的渲染 primitive
- 统一的导入导出路径
- 可嵌入桌面和 Office 的系统能力

如果这个内核稳定下来，后续可以承载多个宿主：Web、Windows 桌面、Office、批量转换工具、云服务、自动化处理、企业集成。

### 3. 它有潜在产品价值

ChemDraw 类工具的用户群体明确，痛点也明确：价格、授权、平台、格式兼容、Office 体验、批量处理和自动化能力。

Chemcore 如果能做到足够稳定，即使本体免费，也可以成为后续项目的入口。它的意义不只在于“做一个免费画图软件”，而在于形成一个专业化学文档工作流的底层能力。

## 仓库现状

### 技术结构

当前仓库由几个主要部分组成：

- `crates/chemcore-engine`：Rust 核心内核
- `crates/chemcore-desktop-service`：桌面原生服务层
- `apps/chemcore-desktop`：Tauri 桌面应用
- `apps/chemcore-office`：Windows Office/OLE 集成
- `viewer`：浏览器和桌面 WebView 共用 UI
- `docs`：架构、格式、渲染、规则、开发日志
- `scripts`：构建、验证、导出、回归辅助脚本

根 `Cargo.toml` 已经是 workspace 结构，说明项目不是临时拼接，而是按长期模块化方向组织。

### 代码规模信号

排除 `target`、`node_modules`、`tmp` 等生成或临时目录后，仓库代码和文档规模已经比较大。

Rust engine 内部有多个大型模块，例如：

- `engine.rs`
- `document.rs`
- `render_objects/graphics.rs`
- `cdxml/export.rs`
- `cdxml.rs`
- `cdx.rs`
- `sdf.rs`
- `render_bonds.rs`
- `render_objects/arrows.rs`
- `glyph_kernel.rs`
- `wasm.rs`

前端也已经拆出不少模块，但 `viewer/app.js` 仍然接近 4000 行，是当前前端最大的复杂度集中点。

桌面和 Office 侧同样存在大文件：

- `apps/chemcore-desktop/src-tauri/src/lib.rs`
- `crates/chemcore-desktop-service/src/lib.rs`
- `apps/chemcore-office/src/windows_office.rs`
- `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`

这说明项目功能很多，但也提示后续需要继续拆分职责。

## 核心内核完成度

### 当前能力

`crates/chemcore-engine` 已经承担了大量核心职责：

- 文档模型
- scene object 管理
- molecule resource 管理
- 工具状态
- 选择和 focus
- 拖拽、旋转、缩放
- 文本编辑
- 键工具
- 箭头和图形
- 轨道和特殊对象
- 右键菜单
- 样式应用
- 命令历史
- render primitive 输出
- WASM API
- CDXML/CDX/SDF 导入导出

这已经不是只会画几条线的原型。

### 评价

内核方向正确，完成度约为：

```text
70%
```

这里的 `70%` 不是说所有功能都完成了，而是说核心架构和主要编辑路径已经成型。剩余难点主要在：

- 复杂格式兼容
- 边角交互
- 高级化学语义
- 大文档性能
- 长期 API 收敛
- 回归测试矩阵

## 渲染系统完成度

### 当前能力

渲染层已经不是单一 SVG 拼接。Rust 侧生成 render primitives，viewer 消费 primitive，再映射到 DOM/SVG。项目里还有 EMF preview renderer，说明渲染目标已经不只浏览器。

当前已经投入较多的渲染主题包括：

- 键几何
- 虚键、楔形键、hash/wedge
- 箭头
- 文本 glyph
- 标签裁剪
- shape
- bracket
- orbital
- SVG 输出
- EMF preview

### 评价

渲染完成度约为：

```text
65% - 75%
```

常见绘制路径已经具备，但 ChemDraw 级别的视觉兼容仍然需要持续测试。尤其是箭头、半箭头、弯箭头、虚线、文本裁剪、Office EMF 输出，这些都属于细节密集区。

## 格式兼容完成度

### Chemcore 自有格式

`.ccjs` / `.ccjz` 的方向清楚：JSON 文档格式，以 `pt` 为单位，面向文档对象和资源模型。

当前自有格式处于可用基础阶段，但还需要继续强化：

- 版本迁移
- schema 校验
- 兼容测试
- 错误恢复
- 长期容器结构
- preview/resource 管理

完成度约：

```text
60% - 70%
```

### CDXML

CDXML 是当前最重要的外部兼容路径。仓库里已经有 Rust 原生 import/export，并且文档中明确 CDXML 应该进入 core，而不是 UI 临时状态。

完成度约：

```text
60% - 70%
```

原因是：常见对象和工作流已经在做，但 CDXML 范围非常大，完整兼容 ChemDraw 不是短期能完成的。

### CDX

CDX 已经有 Rust 原生二进制读写路径，但从代码结构看，它更像通过 CDXML tree 进行桥接，而不是完整重建 CDX 的全部语义层。

完成度约：

```text
35% - 45%
```

这已经是很有价值的开始，但不能认为是完整 CDX 兼容。

### SDF

SDF 目前更适合作为分子结构交换格式，而不是完整文档格式。项目已经接入 V2000 风格的基础导入导出，并且策略上也已经明确：SDF 只能保存分子对象，不能保存完整样式和文档对象。

完成度约：

```text
25% - 35%
```

后续需要考虑：

- V3000
- 多分子记录
- data fields
- stereo
- reaction/RXN
- 与 Chemcore 文档对象的边界说明

## Browser viewer 完成度

viewer 已经有真实编辑器形态，不是 demo 页面。工具栏、文件流、engine host、文本编辑、overlay、context menu、DOM renderer、desktop file host 都已经拆出模块。

但 `app.js` 仍然过大，说明前端协调层承担了太多职责。后续如果继续加功能，应该继续拆：

- 应用状态
- 工具栏状态
- 文件保存提示
- 弹窗和 palette host
- selection summary
- command dispatch
- desktop/browser 差异

完成度约：

```text
60%
```

主要缺口不是“没有 UI”，而是 UI 自动化测试和长期维护结构。

## Windows 桌面端完成度

桌面端已经不是空壳。Tauri 层接入了：

- native file open/save
- recent files
- clipboard
- export
- desktop service
- native command bridge
- WebView + WASM hybrid runtime

这个架构比较务实：高频编辑仍然走进程内 WASM core，低频系统能力走 Tauri/native service，避免每次 pointer/hover 都跨 IPC。

完成度约：

```text
60% - 70%
```

风险是 Tauri `lib.rs` 和 desktop service 文件偏大，后续需要按文件、剪贴板、导出、窗口、菜单、engine session 等职责拆分。

## Office/OLE 完成度

Office/OLE 是项目里最有野心、也最复杂的一层。

代码里已经有：

- COM local server
- OLE class registration
- IDataObject / IOleObject / IPersistStorage / IViewObject2 等接口方向
- clipboard payload
- embedded object 相关格式
- EMF presentation stream
- Word docx payload writer
- preview renderer

这不是占位代码，而是真正在往 ChemDraw 式 Office 集成推进。

但 Office/OLE 本身非常脆，Windows 行为、Word 行为、EMF、storage、clipboard、activation 都有大量边角。当前完成度应该保守估计：

```text
40% - 55%
```

这块价值很高，但要继续作为系统集成层，不应该承载化学业务逻辑。

## 测试与文档

### 测试

engine 测试投入明显：

- `crates/chemcore-engine/tests/render_document.rs` 接近 9000 行
- `crates/chemcore-engine/tests/bond_tool.rs` 超过 8000 行
- 另有 text、special objects、command、palette 等测试
- engine tests 和 src 内部测试合计约 450 个 `#[test]`

这说明项目不是只靠手测推进。

但测试仍有明显缺口：

- UI 端到端测试不足
- 桌面端真实文件流测试不足
- Office/OLE 自动化测试难度高但必须逐步补
- CDXML/CDX/SDF fixture 回归集需要继续扩大
- 大文档性能测试还不明显

### 文档

`docs` 目录内容很多，方向明确，包括：

- architecture
- format v0.1
- project rules
- bond rendering rules
- implicit hydrogen rules
- glyph/text rules
- Office/desktop architecture
- context menu matrix
- developer logs

文档覆盖面很好。

但有一个明显问题：部分中文文档存在编码乱码。这会影响后续维护和外部协作，应该找时间统一修复。

## 当前最大优势

### 1. 架构路线正确

项目没有把化学语义写死在 UI 里，而是持续往 Rust core 收。这是最重要的正确选择。

### 2. 能力已经跨过“玩具”阶段

CDXML、CDX、SDF、SVG、EMF、Office/OLE、桌面端、浏览器端都已经有实质代码。即使有不完整之处，也不是空想。

### 3. 对 ChemDraw 兼容有真实投入

项目已经在细抠键、箭头、轨道、字体、导入导出、Office 行为。这些工作很琐碎，但正是专业化学绘图工具的壁垒。

### 4. 测试意识较强

Rust engine 的测试规模是积极信号。后续只要把格式 fixture 和 UI 测试补起来，稳定性会有基础。

## 当前主要风险

### 1. 大文件复杂度

几个大文件已经承载太多职责。短期能跑，长期会影响迭代速度。

优先关注：

- `viewer/app.js`
- `apps/chemcore-desktop/src-tauri/src/lib.rs`
- `crates/chemcore-desktop-service/src/lib.rs`
- `apps/chemcore-office/src/windows_office.rs`
- `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`

### 2. 格式兼容范围太大

ChemDraw 兼容没有自然终点。必须建立 fixture/oracle 回归机制，否则每个修复都可能引入新回归。

### 3. Hybrid state 同步复杂

桌面端同时有 WASM mirror 和 native session。这个设计目前合理，但 API 语义必须持续保持清楚：谁是热编辑路径，谁是系统能力路径，谁只是同步镜像。

### 4. UI 自动化不足

工具栏、弹窗、palette、右键菜单、保存提示、格式切换都已经复杂。只靠手测会越来越难。

### 5. Office/OLE 易碎

Office 层价值很大，但也最容易被 Windows/Word 细节拖住。需要保持边界：Office 只做嵌入、存储、预览、激活，不复制化学语义。

### 6. 产品边界还要继续定义

Chemcore 是文档编辑器，不是完整 cheminformatics toolkit。SDF、SMILES、mass、formula、reaction、stereo 等能力要明确边界，避免用户误解。

## 完成度总评

如果按“能不能作为正式产品替代 ChemDraw”来评估：

```text
还不能。
```

如果按“是否已经完成核心技术验证”来评估：

```text
已经完成相当大一部分。
```

如果按“是否值得继续投入”来评估：

```text
值得。
```

综合判断：

```text
项目意义：高
技术路线：正确
内核成熟度：中高
产品成熟度：中
格式兼容成熟度：中
Office 集成成熟度：中低到中
长期商业/生态潜力：高
```

## 建议的下一阶段重点

### 1. 固化自有格式

继续完善 `.ccjs/.ccjz`：

- schema
- migration
- validation
- preview/resource 容器设计
- roundtrip tests

### 2. 建立格式回归体系

围绕真实文件建立：

- CDXML fixture
- CDX fixture
- SDF fixture
- SVG/EMF visual oracle
- ChemDraw 对照导出

### 3. 拆大文件

优先拆协调层，而不是顺手重构核心行为。目标是降低维护风险，不改变用户行为。

### 4. 补端到端测试

至少覆盖：

- 浏览器启动
- 桌面端启动
- 打开/保存/另存为
- CDXML/CDX/SDF 导入
- 工具栏切换
- palette 展开收回
- 选择、绘制、撤销
- 导出 SVG/EMF

### 5. 明确产品边界

文档中要清楚说明：

- Chemcore 文件保存什么
- CDXML/CDX 支持到什么程度
- SDF 为什么只保存分子对象
- SVG/EMF 为什么是导出格式，不是完整项目格式
- Office 嵌入对象的预期行为

### 6. 修文档编码

当前部分中文文档乱码，这会影响项目长期可维护性。建议专门清理一次。

## 最终评价

Chemcore 是一个有真实意义的项目，价值不在于“免费画化学结构”，而在于建立一个跨平台、可测试、可扩展、可嵌入 Office 的化学文档核心。

它目前已经超过普通个人项目或早期 demo，核心架构和大量功能已经存在。但要成为稳定产品，还需要继续做三件事：

1. 把格式兼容和视觉渲染做成可回归验证的体系。
2. 把前端、桌面和 Office 的大文件复杂度降下来。
3. 把产品边界和用户承诺写清楚。

如果这三件事持续推进，Chemcore 有机会成为后续化学软件项目的入口和底层资产。

