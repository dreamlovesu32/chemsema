# Chemcore 大模块拆分计划

日期：2026-06-10

## 目标

当前仓库已经形成真实产品形态，但有几个模块承担了过多职责。它们短期能支撑开发，长期会增加维护风险、回归风险和新成员理解成本。

本计划的目标不是大规模重写，而是分阶段降低复杂度：

1. 先拆边界清晰的协调层。
2. 保持用户行为不变。
3. 每一步都能单独测试和提交。
4. 不在拆分过程中顺手改业务规则。
5. 优先保护 Rust engine 作为唯一化学语义来源。

## 当前高风险大模块

优先关注以下文件：

- `viewer/app.js`
- `apps/chemcore-desktop/src-tauri/src/lib.rs`
- `crates/chemcore-desktop-service/src/lib.rs`
- `apps/chemcore-office/src/windows_office.rs`
- `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`

这些文件的问题不是“代码一定错了”，而是职责密度过高。后续继续增加格式、工具、Office 行为和 UI 状态时，会让局部修改影响面变大。

## 总体拆分原则

### 1. 只拆职责，不改语义

拆分时不改变：

- 文档格式
- engine API 行为
- 工具栏交互
- 保存/另存为逻辑
- 导入导出结果
- Office/OLE 对外行为

如果确实需要行为调整，应单独开任务，不混在拆文件提交里。

### 2. 保持薄外壳

大文件拆分后，原入口文件应该变成 orchestration layer，只负责：

- 初始化
- 依赖装配
- 事件接线
- 启动顺序
- 跨模块协调

具体业务逻辑应下沉到独立模块。

### 3. 按数据流拆，不按 UI 表面拆

例如“保存提示”不是简单属于某个按钮，而属于 document lifecycle。应该按文件流、dirty state、format policy、dialog host 拆，而不是按 toolbar 按钮拆。

### 4. 每个模块有清晰输入输出

拆出来的模块应尽量避免直接读写全局变量。优先通过显式参数传入：

- engineHost
- appState
- renderState
- command dispatcher
- host capability
- DOM root

### 5. 先加边界，再搬代码

第一步可以先创建 facade 或 controller，把现有函数包起来。确认无行为变化后，再逐步移动内部逻辑。

## 阶段一：拆 `viewer/app.js`

### 当前问题

`viewer/app.js` 是前端最大协调文件，承担了过多职责：

- 应用初始化
- DOM 查询
- 工具栏状态
- engine host 初始化
- 文件打开/保存
- dirty state
- render 调度
- selection/focus 同步
- pointer/text/editor/controller 接线
- toolbar palette
- desktop/browser 差异
- debug 暴露
- dialog host 接线
- 快捷键和命令调度

这个文件继续增长会导致任何 UI 或文件流修改都很容易影响全局。

### 目标结构

建议逐步形成：

```text
viewer/
  app.js
  app_bootstrap.js
  app_state.js
  app_dom.js
  app_render_loop.js
  app_debug.js
  app_lifecycle.js
  app_keyboard.js
  app_toolbar_controller.js
  app_document_controller.js
  app_selection_controller.js
  app_host_capabilities.js
```

### 模块职责

#### `app.js`

最终只保留：

- import
- 创建 app dependencies
- 调用 bootstrap
- 极少量全局错误处理

目标行数：

```text
200 - 400 行
```

#### `app_bootstrap.js`

负责启动顺序：

- 读取 URL 参数
- 初始化 engine host
- 初始化 DOM roots
- 初始化 controllers
- 注册事件
- 执行首次 render

不直接实现业务逻辑。

#### `app_state.js`

集中定义前端运行状态：

- currentDocumentPath
- dirty state
- currentFormat
- selected tool UI state
- active palette
- zoom / pan
- pending save state
- desktop/browser capability flags

注意：这里不能重新定义化学语义，只保存 UI/session 状态。

#### `app_dom.js`

集中管理 DOM 查询和 DOM refs：

- canvas root
- toolbar root
- status bar
- dialogs
- file input
- overlay layers

避免在多个模块里重复 `document.querySelector`。

#### `app_render_loop.js`

负责：

- requestAnimationFrame 调度
- render primitive 更新
- overlay 更新
- selection/focus overlay 刷新
- render error handling

不处理文件保存和工具栏业务。

#### `app_document_controller.js`

负责 document lifecycle：

- new document
- open document
- save
- save as
- export
- dirty state policy
- lossy format warning
- recent files update

它可以调用 `document_flow.js`，但不应该把文件策略散在 `app.js`。

#### `app_toolbar_controller.js`

负责：

- toolbar 初始化
- 工具按钮状态同步
- palette 展开/收回
- toolbar icon 状态更新
- tool option panel 接线

它不直接修改文档，只通过 command/engine host。

#### `app_selection_controller.js`

负责：

- selection summary
- focus state display
- bottom bar molecule summary
- object settings visibility
- selection-dependent UI enable/disable

化学统计仍来自 engine，不在前端重新计算。

#### `app_keyboard.js`

负责：

- 快捷键注册
- 命令映射
- 输入框/文本编辑状态下的快捷键屏蔽

#### `app_debug.js`

负责 `window.__chemcoreDebug` 暴露。

debug API 应从主逻辑中隔离，避免调试便利污染产品路径。

### 拆分顺序

1. 新建 `app_dom.js`，移动 DOM refs。
2. 新建 `app_debug.js`，移动 debug 暴露。
3. 新建 `app_keyboard.js`，移动快捷键逻辑。
4. 新建 `app_render_loop.js`，移动 render scheduling。
5. 新建 `app_document_controller.js`，收拢文件流。
6. 新建 `app_toolbar_controller.js`，收拢 toolbar 状态。
7. 新建 `app_selection_controller.js`，收拢 selection-dependent UI。
8. 最后整理 `app_bootstrap.js` 和瘦身 `app.js`。

每一步都应保持 `node --check viewer/app.js` 和实际启动可用。

## 阶段二：拆 `apps/chemcore-desktop/src-tauri/src/lib.rs`

### 当前问题

Tauri `lib.rs` 同时承担：

- Tauri app setup
- command 定义
- native menu
- file dialog
- window lifecycle
- recent files
- clipboard
- export
- desktop engine session bridge
- drag/drop
- single instance
- PDF/EMF/file path handling

这会让桌面端任何系统能力修改都集中到一个文件。

### 目标结构

建议拆成：

```text
apps/chemcore-desktop/src-tauri/src/
  lib.rs
  main.rs
  app_setup.rs
  commands/
    mod.rs
    engine.rs
    files.rs
    clipboard.rs
    export.rs
    window.rs
    recent_files.rs
  desktop_state.rs
  menus.rs
  dialogs.rs
  drag_drop.rs
  single_instance.rs
  paths.rs
  errors.rs
```

### 模块职责

#### `lib.rs`

只保留：

- module declarations
- `run()`
- Tauri builder 装配

#### `app_setup.rs`

负责：

- Tauri plugin 初始化
- state 注入
- setup hook
- window 初始配置

#### `commands/engine.rs`

负责所有 `desktop_engine_*` 命令：

- session create/destroy
- load document
- apply command
- render snapshot
- import/export document through engine

#### `commands/files.rs`

负责：

- open file
- save file
- save as
- file metadata
- extension/type detection

#### `commands/clipboard.rs`

负责：

- native clipboard read/write
- Chemcore fragment
- CDXML/SVG/text fallback
- OLE clipboard handoff

#### `commands/export.rs`

负责：

- SVG export
- EMF export
- PDF export
- preview payload

#### `menus.rs`

负责：

- native menu creation
- menu event mapping
- shortcut definitions

#### `desktop_state.rs`

负责 Tauri shared state：

- document service
- recent files
- active windows
- pending open files

### 拆分顺序

1. 先拆 `paths.rs` 和 `errors.rs`，低风险。
2. 拆 `menus.rs`，行为容易验证。
3. 拆 `commands/files.rs`。
4. 拆 `commands/export.rs`。
5. 拆 `commands/clipboard.rs`。
6. 拆 `commands/engine.rs`。
7. 最后整理 setup、single instance、drag/drop。

验证方式：

```powershell
cargo check --manifest-path apps/chemcore-desktop/src-tauri/Cargo.toml
npm run desktop:dev
```

## 阶段三：拆 `crates/chemcore-desktop-service/src/lib.rs`

### 当前问题

desktop-service 是 Rust engine 与桌面系统能力之间的服务层。当前单文件承担：

- session 管理
- document open/save
- `.ccjz/.ccjs` 读写
- format detection
- recent files
- render/export helper
- native service facade

这层后续会越来越重要，应该尽早拆出边界。

### 目标结构

```text
crates/chemcore-desktop-service/src/
  lib.rs
  service.rs
  session.rs
  file_format.rs
  document_io.rs
  recent_files.rs
  export.rs
  preview.rs
  error.rs
```

### 模块职责

#### `lib.rs`

只 re-export public API。

#### `service.rs`

定义 `DesktopDocumentService` 主 facade。

#### `session.rs`

负责：

- session id
- engine instance
- snapshot
- dirty tracking
- active document metadata

#### `file_format.rs`

负责：

- extension detection
- MIME/type mapping
- lossy format classification
- preferred save format

#### `document_io.rs`

负责：

- load `.ccjs`
- load `.ccjz`
- load `.cdxml`
- load `.cdx`
- load `.sdf`
- save 对应格式

#### `recent_files.rs`

负责最近文件持久化。

#### `export.rs`

负责：

- SVG
- EMF payload handoff
- SDF export policy
- future PDF hooks

### 拆分顺序

1. 先抽 `file_format.rs`。
2. 抽 `recent_files.rs`。
3. 抽 `session.rs`。
4. 抽 `document_io.rs`。
5. 抽 `export.rs`。
6. 最后让 `lib.rs` 只保留 facade/re-export。

验证方式：

```powershell
cargo test -p chemcore-desktop-service
cargo check --manifest-path apps/chemcore-desktop/src-tauri/Cargo.toml
```

## 阶段四：拆 `apps/chemcore-office/src/windows_office.rs`

### 当前问题

Office/OLE 文件非常复杂，集中承担：

- CLI command parsing
- COM registration
- OLE class factory
- IDataObject
- IOleObject
- IPersistStorage
- IViewObject2
- IRunnableObject
- clipboard payload
- storage streams
- edit session
- Word docx package writer
- EMF payload writer
- Windows registry
- message loop

这是高风险文件。Office/OLE 本来就难调试，所有东西放在一起会增加未来 bug 定位成本。

### 目标结构

```text
apps/chemcore-office/src/
  main.rs
  windows_office/
    mod.rs
    cli.rs
    constants.rs
    com_server.rs
    class_factory.rs
    ole_object.rs
    data_object.rs
    persist_storage.rs
    view_object.rs
    runnable_object.rs
    clipboard.rs
    storage.rs
    registry.rs
    edit_session.rs
    word_package.rs
    emf_payload.rs
    win32.rs
    error.rs
```

### 模块职责

#### `cli.rs`

负责：

- `--register-user`
- `--unregister-user`
- `--self-test`
- `--copy-clipboard-payload`
- `--write-word-docx-payload`
- `--write-emf-payload`

#### `constants.rs`

负责：

- ProgID
- CLSID
- clipboard format names
- stream names
- HIMETRIC/unit constants

#### `registry.rs`

负责注册表写入和删除。

#### `com_server.rs`

负责：

- COM 初始化
- class object registration
- message loop
- server lifetime

#### `class_factory.rs`

只负责 `IClassFactory`。

#### `ole_object.rs`

只负责 `IOleObject`。

#### `data_object.rs`

只负责 `IDataObject` 和 `IEnumFORMATETC`。

#### `persist_storage.rs`

只负责 `IPersistStorage`。

#### `view_object.rs`

只负责 `IViewObject` / `IViewObject2`。

#### `runnable_object.rs`

只负责 `IRunnableObject`。

#### `clipboard.rs`

负责：

- clipboard format registration
- clipboard payload creation
- OLE clipboard object
- fallback formats

#### `storage.rs`

负责：

- OLE compound storage stream names
- manifest
- ChemcoreDocument stream
- preview streams
- EMF presentation streams

#### `edit_session.rs`

负责：

- desktop wakeup
- temp file session
- polling
- update back to OLE object

#### `word_package.rs`

负责直接写 `.docx` package。

#### `emf_payload.rs`

负责命令行 EMF 输出和 preview payload 组织。

#### `win32.rs`

封装低层 Win32 helper：

- UTF-16 conversion
- HGLOBAL handling
- HRESULT helpers
- registry helper wrappers

### 拆分顺序

Office/OLE 拆分必须非常保守：

1. 抽 `constants.rs`。
2. 抽 `win32.rs` helper。
3. 抽 `registry.rs`。
4. 抽 `cli.rs`。
5. 抽 `storage.rs`。
6. 抽 `clipboard.rs`。
7. 抽 `word_package.rs`。
8. 抽 `emf_payload.rs`。
9. 最后再拆 COM interface vtable 相关模块。

不要一开始就拆 vtable 和 COM object。那部分最容易引入 ABI 错误。

验证方式：

```powershell
cargo check -p chemcore-office
npm run office:self-test
npm run office:print-registration
```

如果涉及真实 Word/OLE 行为，还需要手动验证：

- 复制 Chemcore 对象到 Word
- 粘贴为可编辑对象
- 预览显示
- 双击激活
- 保存 Word 文档后重新打开

## 阶段五：拆 `emf_preview/renderer.rs`

### 当前问题

EMF renderer 文件很大，说明它同时承担：

- primitive dispatch
- GDI object 管理
- path 渲染
- text 渲染
- shape 渲染
- molecule/bond 渲染
- transform/unit 转换
- clipping
- fallback/diagnostic

EMF 输出是 Office 质量的关键路径，后续会持续修细节。它需要更强边界。

### 目标结构

```text
apps/chemcore-office/src/windows_office/emf_preview/renderer/
  mod.rs
  renderer_env.rs
  context.rs
  gdi.rs
  units.rs
  primitives.rs
  paths.rs
  text.rs
  shapes.rs
  bonds.rs
  arrows.rs
  molecule.rs
  clipping.rs
  diagnostics.rs
```

### 模块职责

#### `context.rs`

负责 renderer context：

- HDC
- bounds
- scale
- style state
- transform stack

#### `gdi.rs`

负责 GDI resource RAII：

- pen
- brush
- font
- path begin/end
- object selection restore

#### `units.rs`

负责：

- pt 到 EMF logical unit
- HIMETRIC
- CSS px 兼容换算

#### `primitives.rs`

负责 render primitive dispatch。

#### `paths.rs`

负责 path/polyline/bezier 绘制。

#### `text.rs`

负责文字和 glyph。

#### `shapes.rs`

负责 shape。

#### `bonds.rs`

负责 molecule bond primitive。

#### `arrows.rs`

负责 arrow primitive。

#### `clipping.rs`

负责 clip region 和 knockout。

#### `diagnostics.rs`

负责 debug report。

### 拆分顺序

1. 抽 `units.rs`。
2. 抽 `gdi.rs` RAII helper。
3. 抽 `context.rs`。
4. 抽 `diagnostics.rs`。
5. 抽 `paths.rs`。
6. 抽 `shapes.rs`。
7. 抽 `text.rs`。
8. 抽 `bonds.rs` 和 `arrows.rs`。

验证方式：

```powershell
cargo check -p chemcore-office
```

并使用固定 fixture 对比 EMF preview 输出。

## 不建议优先拆的部分

### `crates/chemcore-engine/src/engine.rs`

虽然大，但它是核心状态机入口。它已经通过子模块承载大量行为。拆它时风险高，应该等外层更稳定后再做。

短期只建议：

- 新功能继续放入明确子模块
- 不把 UI 特定逻辑塞回 engine.rs
- 保持 engine.rs 作为 facade/state machine

### `crates/chemcore-engine/src/document.rs`

文档模型文件大是正常的。除非 schema 明显稳定，否则暂时不急拆。

未来可以按：

- molecule resource
- scene object
- styles
- normalization
- serialization

拆分。

### CDXML/CDX/SDF 模块

格式模块虽然大，但拆分时容易引入兼容回归。建议先补 fixture 回归测试，再拆。

## 推荐提交节奏

每个提交只做一种事情：

```text
refactor(viewer): extract DOM refs from app.js
refactor(viewer): move debug API out of app.js
refactor(desktop): split file commands
refactor(office): extract OLE constants
refactor(emf): extract unit conversion helpers
```

避免提交信息里出现：

```text
refactor and fix toolbar and change save behavior
```

这类提交后续很难回滚和审查。

## 每阶段最低验证标准

### Viewer 拆分

```powershell
node --check viewer/app.js
node --check viewer/<new-file>.js
npm run build:engine-wasm
```

并手动验证：

- 浏览器端启动
- 桌面端启动
- 工具栏切换
- 打开/保存
- palette 展开收回

### Desktop 拆分

```powershell
cargo check --manifest-path apps/chemcore-desktop/src-tauri/Cargo.toml
cargo test -p chemcore-desktop-service
```

并手动验证：

- 打开文件
- 保存
- 另存为
- 最近文件
- 剪贴板
- SVG/EMF 导出

### Office 拆分

```powershell
cargo check -p chemcore-office
npm run office:self-test
```

并手动验证：

- 注册/反注册
- 复制到 Word
- Word 中显示 preview
- 重新打开 Word 文档

### Engine 相关拆分

```powershell
cargo test -p chemcore-engine
npm run build:engine-wasm
npm run verify
```

## 优先级建议

### 第一优先级

1. `viewer/app.js`
2. `apps/chemcore-desktop/src-tauri/src/lib.rs`
3. `crates/chemcore-desktop-service/src/lib.rs`

这三块直接影响日常开发效率。

### 第二优先级

4. `apps/chemcore-office/src/windows_office.rs`
5. `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`

这两块风险更高，但调试成本也更高。建议等核心产品路径更稳后集中做。

### 第三优先级

6. CDXML/CDX/SDF 模块内部拆分
7. engine/document 内部进一步拆分

这类拆分必须以更强 fixture 回归测试为前提。

## 最终目标状态

理想状态不是文件都很小，而是每个模块的修改边界清楚。

最终应达到：

- 改保存逻辑时，不需要理解 toolbar。
- 改 toolbar 时，不需要理解 SDF 提示。
- 改 Tauri 文件对话框时，不影响 native engine session。
- 改 Office clipboard 时，不影响 COM registration。
- 改 EMF text 时，不影响 shape 渲染。
- 改 viewer UI 时，不重新实现化学语义。

这才是拆分的实际价值。

