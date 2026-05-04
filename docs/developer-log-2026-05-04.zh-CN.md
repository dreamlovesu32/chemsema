# Chemcore 开发者日志 - 2026-05-04

作者：张家骏

时间范围：2026-05-04 00:00 至 2026-05-04 23:59，Asia/Shanghai

对比提交：`7dba596 feat: add bracket symbols and valence labels`

包含提交：

- `95867fb Remove obsolete Python and C++ code`
- `5376949 feat: improve CDXML rendering fidelity`
- `0255c7c refactor: move render bounds into engine`
- `4fa0f23 refactor: split viewer editor modules`
- `66cd959 refactor: split chemcore engine modules`
- `3d365bf refactor: consolidate thin engine modules`

## 总结

今天不是单纯做拆分。实际工作分成四条主线：先清理已经被 Rust engine 取代的 Python/C++ 旧路径；然后围绕 ChemDraw 对比样例继续补 CDXML、ACS 模板、箭头、锲形键、标签裁剪、上下标和对象聚焦等渲染保真问题；再把 bounds 等能由内核计算的几何信息收到 engine；最后才是对 viewer 和 Rust engine 做模块拆分，并在发现拆得过细后做收敛。

整体方向没有变：ChemDraw/CDXML 的语义和几何参数应尽量保存在内核和 JSON 数据结构里，viewer 只负责交互、文件流和展示；用户画完后再缩放或切换模板时，最终呈现不能依赖前端临时推断。今天的改动继续把这些规则固定在 engine、文档格式和测试里。

## 旧实现清理

第一步删除了已经不再作为主路径维护的 Python CDXML 转换层和 C++ glyph kernel 工程。

清理内容包括：

- 删除 `src/chemcore` 下旧的 Python CDXML 解析、布局和转换实现。
- 删除 `cpp/chemcore_glyph_kernel` 下旧 C++ glyph kernel、C API、demo 和测试。
- 删除与这些旧路径绑定的脚本，例如 glyph preview、结构比较、箭头量测和 glyph wasm 构建脚本。
- 更新 README 和架构文档，把项目描述调整为 Rust engine + Wasm viewer 的当前结构。
- 保留并继续使用 Rust 侧 glyph/render/CDXML 管线，避免维护两套会逐渐分叉的实现。

这次清理减少了历史包袱，也让后续判断更直接：CDXML import/export、文字布局、渲染几何和编辑语义都以 `chemcore-engine` 为准。

## CDXML 和 ChemDraw 渲染保真

第二条主线是继续对齐 ChemDraw 默认模板和 ACS Document 1996 模板。`compare/` 下新增和更新了 ChemDraw 原始 CDXML/SVG、Chemcore 生成 JSON/SVG 的对照样例，用来直接比对 default 与 ACS 两套输出。

本轮明确了一个关键规则：Default 和 ACS 不是同一个模板按键长简单缩放。它们有各自固定的绘图参数。切换模板时，已有键和后续新键都要切到对应模板参数，而不是从当前键长反推。

写入文档和格式的关键参数包括：

- 普通线宽、粗键宽、hash spacing、bond spacing。
- 实锲形键和虚锲形键的宽端宽度。
- 标签裁剪/退让距离，并且 Default 和 ACS 使用不同退让。
- 键级、锲形键、hash 键、双键 spacing 等 bond-level 字段需要数值化保存在 JSON，而不是只在绘制时临时套模板。

今天也补了几类具体渲染修正：

- 实锲形键和虚锲形键宽端使用模板宽度，不受键长影响。
- Legacy JSON 或导入文档缺少 wedge width 时，会根据当前文档模板补默认值。
- ACS 模板切换后会重排 endpoint label geometry，避免 label bbox 只是按比例缩放而没有按当前字体重新布局。
- Default 与 ACS 使用不同 label clip margin，键端到 label glyph/box 的退让更接近 ChemDraw。
- 等长中心双键的接触/拼接逻辑保持普通线宽，避免交点和轮廓拼接造成局部粗细不一致。
- 小 bracket、shape、text bbox 等导入对象去掉不合理的固定下限，让小尺寸对象按源文件真实尺寸渲染和选中。
- 导入的 shape 对象可以被 select 工具正常聚焦。

## 箭头、Shape 和格式字段

箭头部分重新核对了 ChemDraw 的尺寸字段，而不是只按三个大小档做近似。

本轮补齐的语义包括：

- `length` 对应 CDXML `HeadSize / 100`。
- `centerLength` 对应 `ArrowheadCenterSize / 100`。
- `width` 对应 `ArrowheadWidth / 100`，实心箭头、空心箭头和开口箭头的解释不同。
- `curve` 对应 `AngularSize`，正负方向表示两种弯曲方向。
- `noGo` 对应 ChemDraw 的 cross/hash 标记。
- hollow/open 箭头有自己的三档模板，不复用 solid arrow 模板。

CDXML 导入、JSON 保存、SVG 渲染和 CDXML 导出都同步保存这些数值字段。这样用户先画箭头、再通过选中框缩放，或者从 CDXML 导入已有箭头时，最终几何不再被前端固定档位覆盖。

Shape 方面也补了格式字段：

- 矩形和圆角矩形使用局部 `bbox`。
- 圆角使用 `cornerRadius`。
- 圆和椭圆保存 `center`、`majorAxisEnd`、`minorAxisEnd`。
- `shaded`、`shadow`、`shadowSize` 等 ChemDraw graphic 风格会进入 JSON。

这些改动已经同步写入英文和中文格式文档。

## 文本 face、上下标和标签布局

CDXML 文本 face 位组合继续补齐。今天修正的重点是：导入时尊重源 CDXML 对粗体、斜体、下标、上标等 face 标记的组合，不把未知组合直接落到 normal。

具体包括：

- 识别包含多种 bit 的 face 数值组合，例如 `96`、`97` 这类 ChemDraw 压缩表示。
- 导入 CDXML 文本 run 时保存 source runs 和 display runs，让上下标、加粗、斜体等源格式继续参与渲染。
- 对 formula-like node label，按 CDXML/source run 信息展开显示下标，避免 `CF3`、`PF6`、`CH3` 等标签中的数字丢失下标语义。
- 非化学文本仍然按源文件普通文本处理，不强行套化学识别。

这部分的原则是：锚点、裁剪和 label geometry 可以由 Chemcore 自己计算，但上下标、加粗、斜体等文本格式应尽量尊重源 CDXML。

## SVG 输出和对比工具

为了更稳定地做后端对比，今天新增了 Rust 侧 SVG 输出路径。

- 新增 `render_svg.rs`，把 engine render primitives 输出为 SVG。
- 新增 `crates/chemcore-engine/examples/cdxml_to_svg.rs`，可以后台导入 CDXML 并生成 SVG，用于和 ChemDraw SVG 做精细对比。
- CDXML shape、arrow、text、molecule 的测试开始更多依赖 engine 后台生成结果，而不是只看 viewer 视觉截图。

这使得后续类似“箭头头大小不对”“等长双键线宽不一致”“锲形键轮廓不一致”的问题，可以在后端直接量 primitive 或 SVG，而不是只通过前台目测。

## Render bounds 回到内核

前端原先自己遍历 render primitive 来估算文档 bounds、选择 bounds 和对象 bounds。这类逻辑和 primitive 类型、role 过滤规则强绑定，放在 viewer 里会让前端复制渲染层知识。

今天把 bounds 计算移到 engine：

- 新增 `RenderBoundsScope`，区分 `all`、`document` 和 `selection`。
- `Engine::render_bounds()` 生成 render primitives 后按 scope 过滤。
- `document` scope 排除 knockout、hover、selection 和 preview role。
- Wasm 暴露 `renderBoundsJson(scope)`。
- viewer 通过 `engine_bridge.js` 消费内核 bounds。

这样 fit-to-document、选择范围、导出视图和对象聚焦不再依赖前端维护另一套 primitive bounds 规则。

## Viewer 模块整理

`viewer/app.js` 已经承担太多职责。今天把可以稳定分离的部分拆出来：

- engine JSON/render/bounds 桥接。
- 文件打开、保存、CDXML 判断和下载。
- bounds/viewBox/点距离等 viewer 几何工具。
- render primitive 到 SVG DOM 的直接渲染。
- JSON/CDXML 文档加载、示例加载、文档标题和 meta 刷新。
- toolbar、输入控件、导入导出按钮和文本编辑控件绑定。
- 主 toolbar 和 secondary toolbar 的 SVG 按钮渲染。

拆分后，`app.js` 保留应用状态、核心 pointer/text editing 流程和 render orchestration。中途出现过一次前端 helper 引用漏掉的问题，随后改为显式 import，避免模块化后继续依赖隐式全局变量。

## Rust engine 模块整理和收敛

Rust 侧的大文件也做了模块化。拆分范围包括 abbreviation、CDXML、editing、engine、select、text edit、render、render objects 等长期增长的 hub 文件。

拆分后又做了一轮收敛：把只服务单个父模块、没有独立领域意义的薄模块合并回调用上下文。例如 render bounds 回到 `engine.rs`，arrow object 的局部几何回到 arrow object renderer，free text 的换行 helper 回到 text object renderer。

保留的小模块必须有清晰边界，例如 CDXML XML tree、CDXML text run 转换、bond geometry、bond metrics、label refresh、selection arrange 等。后续不再按行数机械拆分，而应按是否经常一起改、是否有稳定领域概念、是否减少跨文件跳转来判断。

## 测试和验证

今天新增或强化的测试覆盖了：

- Default/ACS 模板参数、wedge width、label clip margin 和 ACS 切换后的 label geometry 重排。
- CDXML arrow geometry modifier 的导入和导出。
- hollow/open arrow 的独立尺寸模板和细线宽渲染。
- CDXML shape style 参数导入导出。
- CDXML face 位组合、formula-like label 下标、示例文件中的下标导入。
- 小文本 bbox、小 bracket 几何、shape 对象 select 聚焦。
- 中心双键拼接后的线宽保持。
- SVG 导出是否使用 engine render primitives。

运行过的验证命令包括：

- `cargo fmt`
- `cargo test -p chemcore-engine`
- `npm run build:engine-wasm`
- `node --check viewer/app.js`
- `node --check viewer/*.js`
- `git diff --check`

最终 `chemcore-engine` 测试保持通过：

- unit tests：39 passed
- `tests/bond_tool.rs`：141 passed
- `tests/render_document.rs`：81 passed，2 ignored
- `tests/text_tool.rs`：32 passed
- doctests：0

## 代码量统计

统计口径：`7dba596..3d365bf`，即从上一份完整开发日志提交之后到今天已提交工作的末尾；统计 Git 跟踪的文本文件，二进制文件只计入文件数，不计入行数。本节不包含本次新增的开发者日志文件。

- 提交数：6
- 变更文件：120 个
- Git diff：`+25,477 / -25,497`
- 文件状态：新增 50 个，修改 44 个，删除 26 个
- 项目跟踪文件总数：132 个 -> 156 个，净增加 24 个
- 其中文本文件：127 个 -> 151 个，净增加 24 个
- 二进制文件：5 个 -> 5 个
- 项目文本总行数：69,494 行 -> 69,474 行，净减少 20 行

今天的净行数接近持平，是因为两类变化相互抵消：一方面删除了大量旧 Python/C++ 实现，另一方面新增了 CDXML/渲染保真能力、对比样例、测试、SVG 输出和模块化后的 Rust/viewer 文件。
