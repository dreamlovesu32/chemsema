# Changelog

ChemCore 的公开变更记录会保存在这里。

## 1.0.0-beta.4

大文件交互、CDXML 兼容和 agent-friendly CLI beta 版本。

- 新增 `chemcore-cli` crate，支持 headless 文档检查、格式转换、导出、空白文档生成和 JSON 命令执行。
- 新增适合 agent 调用的直接 engine 命令：文本创建、文本 runs 替换、原子标签 runs 替换、目标删除、目标移动、目标旋转、箭头几何编辑、图形几何编辑、文档 load/export/convert/inspect，以及文档样式应用。
- 新增结构化 CLI 执行报告，记录每条命令是否执行、是否改变文档、新建/更新/删除了哪些 id、失败错误、每条命令后的 after 快照，以及最终文档状态。
- 新增 `--document-json` 和 `--inspect-after` CLI 选项，方便脚本和 agent 在不打开 GUI 的情况下读取 ChemCore 内部 JSON、分子状态和对象状态。
- 改进桌面文件格式服务中的 `.json`/`.ccjs` 处理，让 ChemCore 内部 JSON 更容易作为交换格式使用。
- 改进 CDXML 导入导出保真度，覆盖标签、箭头、符号、粗线宽度、自由基价态、grouped arrows、堆叠标签、attached-label group layout、数字 glyph 锚点、标签内部缓存 fragment，以及带括号的 sulfonyl 标签。
- 收紧 glyph clipping 和标签几何，包括更新 glyph clip polygon 覆盖、更保守的导入标签锚点，以及同步 synthetic SVG 快照。
- 重构大文件交互性能：让编辑器渲染更新更局部化，缓存结构移动预览输入，避免不必要的整页刷新，优化对象创建延迟，并在 patch 前同步 deferred object creation。
- 重建大结构选择和拖拽预览管线，包括本地绘制预览、frame-local structure drag preview、后端 target primitives、前端 partial-bond preview，以及更稳定的 live selection preview。
- 修复绘制、pointer commit、对象创建、选择拖拽、已选对象拖拽、括号 handle 编辑、箭头 handle 编辑和多分子拖拽后的 hover、focus 与 overlay 生命周期问题。
- 改进 grouped object 和选择语义，包括 group 内子对象编辑、grouped molecule hover hit testing、防止框选区域拖动父 group，以及修复多分子拖拽的增量渲染 target。
- 优化箭头、括号、图形和对象控制点，包括弯箭头样式/几何预览、括号 handle resize refresh、统一对象控制点样式，以及拖拽选择时隐藏诊断 marker。
- 新增浏览器文件拖拽打开能力，支持拖入文件在当前 viewer 中打开，补充 shared viewer display scale 处理，以及更快的桌面端/viewer 开发脚本。
- 增加 viewer interaction、大文件拖拽预览、大对象操作、glyph clip manifest、SVG 像素对比和 PowerPoint/CDX 渲染对比相关的回归与诊断覆盖。
- 新增中英文 CLI 命令指南、公开交互反馈规则、早期项目历史说明，并更新 README 架构说明，明确共享 Rust engine、对人友好的 UI 和对 agent 友好的 CLI。

## 1.0.0-beta.3

安装包热修复 beta 版本。

- 修复 Windows NSIS 安装器里的 Office/OLE 注册 hook：现在会在实际安装目录中查找 `chemcore-office.exe`，不再固定假设它位于旧的 `resources` 子目录。
- 同时兼容安装目录根部和 `resources` 子目录两种 Office server 布局，避免旧打包实验路径影响注册。
- 加固安装后注册流程：安装器会优先尝试 machine-wide COM/OLE 注册；如果 machine 注册无法启动或返回失败码，会自动降级为当前用户注册。
- 加固卸载清理流程：卸载时会同时尝试清理 machine-wide 和 current-user Office/OLE 注册。
- 重新构建 Windows x64 安装包，并在清理安装痕迹后完成手动干净安装验证。

## 1.0.0-beta.2

第二个公开 beta release。

- 添加括号与计数文本之间的 link 关系，用于表达重复单元；支持右键菜单 Link/Unlink、`Ctrl+L` / `Ctrl+Shift+L` 快捷键、CDXML 导入时自动配对，以及编辑后的重复单元语义刷新。
- 改进括号文本编辑：括号绘制后生成的空标签会在下一次工具动作时丢弃，非空标签会在切换工具前提交；括号标签在文本工具下仍可聚焦和编辑；括号标签的位置与默认字体大小按 ChemDraw 括号 fixture 对齐。
- 修复重复单元的化学摘要：当括号计数文本已 link 且重复单元定义明确时，计数会参与分子式和分子量计算；取消 link 会解除计数语义，但不破坏括号自身的选择关系。
- 扩展 group 与括号相关的选择行为：双击分子会带上包围它的括号和已 link 的计数文本；group 内的普通文本仍可编辑；从选择工具切到其他工具时会清除当前选中状态。
- 修复绘制、修改弯箭头弧度、以及在选中对象之间移动鼠标后的 hover/focus 残留；已选中的标签、键和原子不会在选择框内部继续显示内部 hover。
- 补齐桌面端与浏览器端的编辑细节：窗口上沿在弹窗状态下也能拖动；编辑区域会拦截浏览器右键菜单和常见浏览器快捷键；右键菜单的乱码指示符已替换为稳定字符。
- 修复泛基团标签的化学摘要逻辑：选中含有 `R`、`R'`、`R''` 或已连接 `Ar` 的分子时，不再显示会暗示组成已确定的分子式或分子量。
- 将已连接的 `Ar` 标签按芳基泛基团处理，不再在结构标签编辑时误判为氩元素；显式元素替换仍通过元素工具链路完成。
- 重建浏览器 WASM engine 和 Windows 桌面端可执行文件，确保 Web 与桌面端使用同一套修正后的内核行为。
- 添加括号 CDXML 导入、重复单元 link、group 编辑、选中对象 hover 抑制、泛基团化学摘要，以及完整缩写展开摘要的回归测试和公开 fixture。

## 1.0.0-beta.1

第一个公开 beta release。

- 公开共享 Rust 化学编辑内核、浏览器 viewer、Windows 桌面壳，以及 Office/OLE 集成基础。
- 添加 CDXML/CDX 导入导出、SVG 导出、EMF preview 生成，以及面向 Word 的剪贴板/OLE payload 支持。
- 加入公开 synthetic CDXML 回归 fixture，并保留维护者本人绘制的真实论文图 benchmark 文件。
- 添加 GitHub Actions CI、GitHub Pages demo 部署、issue templates、roadmap 和渲染对比文档。
- 记录当前 beta 状态：源码构建已可用，Windows 安装包仍在测试中。
