# Chemcore 开发者日志 - 2026-05-06

作者：张家骏

时间范围：2026-05-06 00:00 至 2026-05-06 23:59，Asia/Shanghai

基线提交：`f21d60f Add selection resize handles`

工作目录：`<repo>`

### 总结

今天的主线是把 5 月 5 日完成的 Windows 原生开发环境继续推进成真正的 Windows 桌面产品形态。项目不再只是“能在 Windows 上跑 viewer”，而是开始形成清晰的桌面架构：Tauri 2 桌面壳、Rust native document service、桌面文件系统能力、Windows 剪贴板、多文档窗口、原生/自绘桌面 chrome、PDF/EMF 预览导出，以及未来 Office/OLE 集成所需要的边界。

同时，前端 viewer 和 Rust engine 继续收敛职责：浏览器端仍通过 WASM 工作，桌面端开始通过 Tauri command 探测 native Rust engine，但编辑主循环暂时保持 WASM 同步模型，以降低迁移风险。UI 层通过 `EngineHost`、`DesktopFileHost` 和 `ColorHost` 这些 host 抽象接入能力，避免后续 Web、Desktop 和 Office 各自分叉出一套化学逻辑。

化学语义和 CDXML 保真也继续推进。今天修复了过价氧 label 识别：中性三键氧会被标记为 invalid，而带正电的三键氧仍然有效。CDXML 导入导出继续围绕 double bond placement、ACS/Default 绘图参数、wedge width、label anchor、centered label 和 numeric suffix 做稳定化。晚间还加入了选择对象 resize handles，使选择工具从移动、旋转、排列、翻转、上色继续推进到可缩放选中对象。

### Windows 原生脚本和桌面架构

今天先把活跃 npm 脚本从 Bash 迁到 Node。`build:engine-wasm`、`dev:engine` 和 `verify` 不再调用 `bash scripts/*.sh`，而是分别调用：

```text
node scripts/build-engine-wasm.mjs
node scripts/dev-engine-wasm.mjs
node scripts/verify.mjs
```

旧的 Bash 脚本被删除。新的 `build-engine-wasm.mjs` 直接用 Node `spawnSync` 调 `wasm-pack build`，并删除 wasm-pack 生成的 `viewer/engine/.gitignore`，保持 viewer runtime artifact 可被仓库跟踪。新的 `dev-engine-wasm.mjs` 用 Node `fs` 和 `crypto` 扫描 `Cargo.toml`、`Cargo.lock`、engine `src/**/*.rs` 并计算 SHA-256；输入变化后自动重建 WASM，不再依赖 `find`、`sha256sum`、`awk` 等 Unix 工具。新的 `verify.mjs` 串行运行 `cargo test`、WASM 构建、`node --check viewer/app.js`，最后检查 `viewer/engine` 生成物是否同步。

这次脚本迁移的意义是让 Windows 原生 PowerShell 成为一等开发入口。Git Bash 仍可作为辅助工具存在，但项目的主 npm 工作流不再要求 Bash。

今天还新增了 `docs/windows-desktop-office-architecture.zh-CN.md`，并在 `docs/project-rules.zh-CN.md` 中把它作为 Windows 桌面端和 Office 集成的长期方案引用。该文档定义的方向是：

```text
crates/chemcore-engine
  唯一化学内核：document model、editing commands、CDXML、SVG、render primitives、hit testing。

crates/chemcore-desktop-service
  桌面 document service：native engine session、文件读写、最近文件、导出、未来自动恢复。

apps/chemcore-desktop
  Tauri Windows 桌面应用：窗口、菜单/命令、文件对话框、剪贴板、文件关联、拖拽打开。

apps/chemcore-office
  未来 Office/OLE 集成：可嵌入、可双击编辑、带 preview 的 Chemcore object。

viewer/
  Web 和 Desktop 共用的编辑 UI 层。
```

文档里明确禁止几条路线：不做临时 Electron 版，不在桌面端复制化学编辑逻辑，不让 Office 插件直接解析 Chemcore JSON，不只做 SVG 粘贴后再补可编辑对象，也不把 `.ccjz` API 永远设计死成单一 gzip JSON。`.ccjz` 第一阶段仍是 gzip JSON，但长期要通过稳定容器 API 暴露，为 preview、资源、缩略图和 Office object payload 留空间。

### Tauri 桌面壳和 native engine 边界

今天建立了 `apps/chemcore-desktop/src-tauri`。Tauri app 加入根 Cargo workspace，包含 `Cargo.toml`、`build.rs`、`src/main.rs`、`src/lib.rs`、capabilities、Tauri 配置和一组默认应用图标。`tauri.conf.json` 指向现有 `viewer/`，dev URL 是 `http://127.0.0.1:8767/viewer/`，窗口默认 1280x900，最小 960x640。

npm 新增桌面命令：

```text
npm run desktop:dev
npm run desktop:build
npm run desktop:info
```

`scripts/desktop-tauri.mjs` 负责在 `apps/chemcore-desktop` 下调用本地 Tauri CLI。`scripts/desktop-dev-server.mjs` 是一个小型 Node 静态服务器，服务整个仓库并正确返回 JS、CSS、WASM、SVG、PNG 等 MIME；如果 8767 已被占用，它会保持进程存活，让 Tauri dev 复用已有服务。

随后新增 `crates/chemcore-desktop-service`。这个 crate 直接依赖 `chemcore-engine`，用 `DesktopDocumentService` 管理 native engine sessions。它提供 session 创建/释放、JSON/CDXML 加载、document JSON、state JSON、render list、render bounds、CDXML 和 SVG 导出等 API，并带有 session 生命周期和空文档 render JSON 的测试。

Tauri 后端把 `DesktopDocumentService` 放进 state，并暴露一组 `desktop_engine_*` commands。前端新增 `viewer/engine_host.js`，把原来直接 `new WasmEngine()` 的路径改成 `engineHost.createEngineSession()`。Web 默认走 `WasmEngineHost`；Tauri 环境短期走 `DesktopHybridEngineHost`：编辑 UI 仍使用 WASM session，启动时额外创建 native session 做 smoke test，读取 document JSON、render list、bounds 和 SVG 后释放，用来证明 Tauri command 到 Rust engine 的通路可用。

这条 hybrid 路线很重要：它让桌面端 native engine path 开始真实运行，但不把编辑器同步调用模型一次性推翻。后续要切 native path，应继续只实现 `TauriEngineHost`，而不是让 UI 层到处直接调用 Tauri commands。

### 桌面文件、剪贴板、导出和窗口能力

下午桌面端能力明显加厚。`crates/chemcore-desktop-service` 从单纯 engine session wrapper 扩展成文件服务：

- `.ccjz` 在 native Rust 侧用 `flate2` 做 gzip 读写。
- `.ccjs`、`.cdxml`、`.svg` 走文本读写。
- 未知保存扩展默认按 `.ccjz`。
- 读取时如果文件头是 gzip，即使扩展不明确也按 `.ccjz`。
- 文本内容如果像 CDXML，会识别为 `cdxml`。
- 最近文件列表持久化到用户 data dir 下 `Chemcore/desktop/recent-files.json`，最多保留 10 个，并在返回时标记文件是否仍存在。

Tauri 后端新增了原生文件对话框和文件读写 commands，包括打开、保存、导出保存路径选择、按路径读写文档、写 base64 导出数据、清空最近文件和读取启动 pending open paths。桌面端打开/保存不再只依赖浏览器 File System Access API，而是可以走真正的 Windows 文件路径。

剪贴板也开始走 Windows 多格式模型。桌面端写剪贴板时会同时写入：

```text
Chemcore Clipboard Fragment
Chemcore Document JSON
ChemDraw Interchange Format
chemical/x-cdxml
image/svg+xml
SVG
Unicode text
```

读取时按同样的格式取回，viewer 再决定 paste 优先级。这为后续 Chemcore 与 Office、ChemDraw、浏览器和其他应用之间的复制粘贴打基础。

导出方面新增了两个预览目标。PDF preview 在 viewer 侧实现：`viewer/export_preview.js` 把当前 SVG 渲染到 canvas，以 JPEG 嵌入一个单页 PDF，再把 base64 交给桌面端写文件。EMF preview 在 Tauri 后端实现：后端读取 render list JSON 和 bounds JSON，用 Windows GDI 绘制线、polygon、path、rect、circle、ellipse、text 等基础 render primitives，生成 Windows Office 更友好的 Enhanced Metafile。

窗口能力也从普通 Tauri window 往桌面应用形态推进。`viewer/index.html` 新增自绘 `desktop-titlebar`，包含品牌图标、文档标签页、新建 tab 按钮和 minimize/maximize/close。Tauri 后端新增 window commands：设置标题、最小化、最大化/还原、关闭、开始拖动窗口、查询最大化状态、分离文档窗口、取回分离窗口 payload。前端通过 `DesktopFileHost` 调用这些能力。

多文档标签页是今天桌面 UI 的重点。`viewer/app.js` 新增 `documentTabs` 状态，每个 tab 保存自己的 engine、文件名、文件路径、zoom、标题和当前文档状态。New/Open/DnD/启动路径都改为 tab-aware：打开文件会进入新 tab，关闭最后一个 tab 会创建新的 Untitled。桌面端拖拽 tab 到标题栏下方可以创建新的 Tauri window，并把当前 document payload 暂存在 Tauri state 中，新窗口启动后按 window label 取回。

原生菜单也做过一轮 File/Edit/View command wiring，包括 recent files、导出 CDXML/SVG/PDF/EMF、undo/redo/cut/copy/paste/delete、zoom 等。后续又把 `USE_NATIVE_MENU` 暂设为 `false`，说明当前阶段优先走自绘 desktop chrome 和 HTML command system，避免 native menu 与 focused window command 产生双源。

### 颜色工具和 toolbar 体验

今天颜色工具经历了两步收敛。第一步是把“当前文档使用过哪些颜色”从前端 DOM/JSON 扫描迁到 Rust engine。`Engine::document_colors()` 会遍历 `ChemcoreDocument` 的 JSON 表示，收集 key 中包含 `color`、`fill`、`stroke`、`background` 的字符串值，支持 `#rrggbb`、`#rgb` 和 `rgb(r, g, b)`，跳过 `none` 和空值，并保持去重后的稳定顺序。WASM 和 desktop service 分别暴露 `documentColorsJson()` / `document_colors_json()`，viewer 的 `currentDocumentColors()` 改为优先询问 engine。

第二步是把颜色选择 UI 收敛成 `ColorHost`。短暂尝试过 Windows `ChooseColorW`，但随后移除系统 color dialog 依赖，统一改为 Chemcore 自绘颜色对话框。新的 `viewer/color_host.js` 提供：

- 基本颜色 palette。
- 自定义颜色 palette。
- HSV 色谱区域。
- 亮度滑杆。
- RGB / HSV / Hex 输入。
- 添加到自定义颜色。
- OK/Cancel、Escape 和 backdrop 关闭。

这样 Web 和 Desktop 的颜色体验一致，也不会被 Windows common dialog 的行为限制。

toolbar 侧也做了密度和图标整理。`viewer/toolbar.js` 把 bond、arrow、shape、bracket、ring、text format、distribute 等图标从大量手写 SVG 字符串收敛成生成函数。`viewer/styles.css` 增加一组 CSS 变量控制 desktop/browser 两套密度：topbar 高度、secondary toolbar 高度、tool rail 宽度、button 尺寸、icon 尺寸、stroke width、color panel 尺寸等。浏览器 shell 使用更紧凑的比例，桌面 shell 保持更接近专业绘图软件的控件密度。

选择工具还新增了 selection color picker。Rust engine 的 `apply_color_to_selection()` 可以给选中的 text object、arrow、shape、molecule object、bond、node label 和 label runs 上色。前端 toolbar 的 selection color 通过这个 engine command 修改文档，而不是只改 viewer 状态。

### 化学语义和 CDXML 保真

今天修复的一个明确化学问题是过价氧。`engine/text_edit/labels.rs` 的元素/氢 label 验证不再只看 label 文本，而是接收整个 fragment，统计节点相连 bond order，再调用隐式氢/典型价态规则判断当前元素 label 是否合理。加载 JSON 或 CDXML 后会对所有已有节点刷新 valence recognition。新增测试覆盖：

- 中性三键氧 label `O` 会生成 `labelRecognition.status = invalid`，并渲染红色 invalid 框。
- 带 `+1` 电荷的三键氧仍然有效。

CDXML 绘图参数继续稳定。导入 CDXML 时会保留 bond length、line width、bold width、hash spacing、bond spacing 等参数，但不再把导入参数自动等同于 UI 当前 preset。`document_style_preset` 默认仍是 Default，用户如果要套用 ACS 1996，需要从 Style 菜单显式确认。前端把原来的 style preset `<select>` 改成 Style 按钮和菜单，应用 preset 前会弹出确认，因为它会 rescale drawing 并更新现有 bond、label 和 graphic metrics。

双键导入也继续补强。CDXML 中有显式 `DoublePosition=Left/Right/Center` 的 double bond 会标记为 frozen。没有显式位置但带 imported line style / line weight 的 double bond，也会保持 center/frozen，避免 dashed-solid 等样式被自动挪侧。真正没有位置和特殊样式的 double bond 会调用 engine 的 `automatic_double_bond_placement_for_segment()` 自动判断应该 center 还是放在某侧。相关测试覆盖了 benzene inward side、未指定 alkene double bond、right-side double bond rendering、double bond spacing 与 CDXML BondSpacing 的关系。

楔键宽度不再走固定例外，而是按 `BoldWidth * 1.5` 派生，并受默认 bond stroke 下限约束。ACS/Default 的 label clip margin 也根据 imported bold width 派生，避免把 ACS 当成 Default 的简单缩放。

键交汇渲染也有一轮比较重要的重构，尤其是虚楔形键和 hash bond。原先 hash/hashed wedge 在连接到普通键、中心双键外侧线或多键节点时，交汇逻辑容易在两个方向之间摇摆：有时保持母 polygon 不动，只让别的键退让；有时又用中心双键外线的专门 intersection 逻辑去推算退让距离。今天把这套逻辑收敛为更明确的规则：

- `render_contact` 的 main bond contact kernel 会把 hash bond 和 hashed wedge 视为 hash contact obstacle，不让它们参与普通主键交汇 patch 的 ring/kernel 计算。
- hash bond 和 hashed wedge 自己在无 label 的连接端按统一的 `hash_contact_retreat_distance_for_bond()` 主动退让；这个距离由黑段长度和目标 gap 组成，并随 stroke width 缩放。
- 如果连接端有可见 label，则仍以 label clipping 为主，不额外叠加 hash contact retreat，避免 label 旁边的虚线/虚楔被过度截短。
- hashed wedge 的外轮廓 polygon 和内部 knockout/hash pattern 分离计算：outline 会按连接端退让，pattern 仍按原始楔形范围生成 knockout，保证虚楔视觉黑白节奏不被接触退让破坏。
- 原来放在 `render/bond_geometry.rs` 里针对 center double outer line 的大段特殊 retreat/intersection 逻辑被删掉，改成更直接的 endpoint 是否还有其他 bond、是否有 label、是否为 hash obstacle 的判断。
- 普通 main contact patch 现在会保留参与键的 stroke；如果一个交汇处所有键颜色相同，bridge patch 用共享颜色，否则各自 patch 用各自 bond stroke，避免 CDXML 彩色键在交汇处被默认黑色覆盖。

solid wedge 和普通主键交汇也被一起补强。三向/四向主键交汇会生成有效 contact patch；solid wedge 接入三向节点时，宽端会被 contact kernel 正确裁出五边形，而不是压到错误的端点。对角度比较刁钻的三向 solid wedge，交汇点使用 extended contour intersection，保证相邻普通键和楔键共享同一个真实交点。测试新增和改写了 hashed wedge against connected single bond、both endpoints retreat、hash bond retreat、label clip without extra retreat、hash obstacle ignored by other contacts、center double outer line、solid/dashed center double outer line、three-way/four-way contact、solid wedge extended intersection 等情况。

label anchor 是今天反复修正最多的区域。最终收敛出的规则是：

- 普通 attached label 保持 glyph anchor 语义，右侧 anchor 仍偏向 terminal letter，跳过 trailing digit/subscript。
- CDXML centered label 使用专门的 centered layout：如果导入时看到 `LabelDisplay="Center"` 或 text justification 为 Center，则 label 标记为 `attached-group-center`、`align=center`、`anchor=middle`。
- centered label 会保存 bbox、box value 和 glyph polygons，并在导出时写回 `LabelDisplay="Center"` / `LabelJustification="Center"`。
- 普通 label 的 hover、bond drag、text edit reopen 继续使用 glyph polygon center；centered label 则用 whole text bbox center 和最接近中心的 glyph box。

测试中用 `N3`、`Ph`、invalid `X3`、example fixture 中的 centered numeric suffix label 反复覆盖这些情况，并检查 import -> export -> import 后 centered display 仍然稳定。

### 选择缩放手柄

今天最后一轮功能是给 selection 添加 resize handles。Rust engine 新增 `selection_resize_drag` 状态和 `EditorCommand::ResizeSelection`，WASM 暴露：

```text
beginSelectionResize(handle, x, y)
updateSelectionResize(x, y)
finishSelectionResize(x, y)
```

支持八个 handle：

```text
n, s, e, w, ne, nw, se, sw
```

边 handle 做单轴缩放，角 handle 做等比缩放。resize drag 保存初始 selection bounds、选中节点的原始位置、选中 scene object 的原始 payload/transform、undo 状态和 changed 状态。第一次实际产生缩放变化时才 push undo snapshot，避免点击 handle 但不移动也污染历史。

底层缩放逻辑集中在 `engine/select/drag.rs`：

- 根据 handle 和 opposite pivot 计算 scale。
- 最小 scale 限制为 `0.05`，避免缩成零或翻转。
- molecule 节点按 world 坐标绕 pivot 缩放，再写回 fragment local position。
- 缩放后刷新 attached node label geometry、fragment bounds 和 symbol chemistry。
- text、bracket、symbol、shape、arrow 等 scene object 会按类型更新 transform translate、payload bbox、box、points、named points、text width/height 和 graphic dimensions。
- circle/ellipse 这类使用 absolute points 的 shape 单独处理，避免 transform 与 payload 双重缩放。

viewer 侧根据当前 selection bounds 生成八个 handle，做 pointer hit test 和 cursor 切换。拖动时调用 engine resize API，并显示缩放百分比 overlay。CSS 新增 `.editor-selection-resize-handle` 和 `.editor-selection-resize-label`。测试覆盖了 east handle 单轴缩放、对象几何缩放和 corner handle 等比缩放。

这个功能让 select tool 的编辑能力明显完整了一档：选择对象现在可以移动、旋转、排列、翻转、上色和缩放，而且核心 mutation 仍在 Rust engine 中。

### 今日生成物和工作树

今天多次更新 `viewer/engine/chemcore_engine_bg.wasm`、`viewer/engine/chemcore_engine.js`、`viewer/engine/chemcore_engine.d.ts` 和 `viewer/engine/chemcore_engine_bg.wasm.d.ts`。这些都是 Rust engine/WASM API 变化后的生成物，主要对应 document colors、selection coloring、clipboard、selection resize 等新增能力。

本日志写入前工作树是干净的。写入日志后新增：

```text
docs/developer-log-2026-05-06.zh-CN.md
docs/developer-log-2026-05-06.en.md
```

### 后续注意事项

桌面端已经有 Tauri 壳、native document service、hybrid native probe、原生文件/剪贴板/导出、多文档标签页和分离窗口。但编辑主循环仍主要走 WASM host。后续切 native engine path 时，应继续通过 `EngineHost` / `DesktopDocumentService` 分阶段迁移，不要让 UI 层散落直接调用 Tauri commands。

自绘 titlebar 和 document tabbar 已接管很多窗口行为。如果后续恢复 native menu，需要明确 native menu 与 HTML command system 的关系，避免 recent files、focused window command 和菜单状态出现双源。

颜色选择器现在是 Chemcore 自绘 dialog。后续应补焦点归还、键盘可访问性、自定义颜色持久化，以及小屏/高 DPI 下的布局验证。

CDXML label anchor 今天已经收敛出普通 attached label 与 centered label 两套规则。以后改 glyph kernel、label layout 或 CDXML label import/export 时，应优先跑相关 render/text/bond tool 测试，避免再次把普通 glyph anchor 和 centered label anchor 混在一起。

selection resize 已进入 engine，但还需要继续补多对象混合选择、旋转 transform、curved arrow、absolute point shape、undo/redo、与 arrange/flip/rotate 组合，以及 resize 后 CDXML/SVG 导出稳定性。
