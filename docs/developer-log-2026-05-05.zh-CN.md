# Chemcore 开发者日志 - 2026-05-05

作者：张家骏

时间范围：2026-05-05 00:00 至 2026-05-05 23:59，Asia/Shanghai

基线提交：`56cdd4a feat: adopt ccjs and ccjz document extensions`

工作目录：`D:\chemcore`

### 总结

今天的主线是把项目从 WSL 交接到 Windows 原生开发环境，并确认 `chemcore-engine`、WASM viewer、npm 脚本、Git、VS Code 和浏览器保存流程在 Windows 上可以正常工作。迁移后的方向不变：Rust `chemcore-engine` 仍然是文档模型、编辑语义、导入导出、命中测试和 render primitives 的权威；`viewer/` 只作为浏览器端适配层。

今天还修复了 viewer 里的保存失败问题。直接原因是 `viewer/document_flow.js` 使用了 `CHEMCORE_TEXT_EXTENSION` 但没有导入，点击 Save as 时会抛出 `ReferenceError`。同时调整了保存顺序，让浏览器文件保存框先在用户点击手势内打开，之后再生成或压缩保存内容，避免 Chromium 的 File System Access API 因异步压缩丢失用户激活。

原根目录临时交接文件 `HANDOFF-2026-05-05.md` 的内容已经收敛进本日志，包括迁移背景、今日提交、交接注意事项、已知风险和后续路线。该临时文件已经删除，不再作为单独交接文档保留。

### 今日接收的迁移状态

迁移目标目录是 `D:\chemcore`，对应 WSL 路径为 `/mnt/d/chemcore`。原 WSL 源目录是 `/home/jiajun/chemcore`。迁移采用完整复制，包含 `.git`、`target/`、`node_modules/`、`tmp/`、`viewer/engine` 生成物和当时工作树状态。

迁移时两边 HEAD 一致：

```text
56cdd4a feat: adopt ccjs and ccjz document extensions
```

今天接手时需要吸收的两个最近开发提交是：

```text
0e4e0e1 feat: improve cdxml rendering fidelity
56cdd4a feat: adopt ccjs and ccjz document extensions
```

`0e4e0e1` 的核心内容是继续提升 CDXML 导入、内部模型、另存、再导入和渲染保真：

- 新增统一 CDXML color table 解析和导出路径。
- 颜色语义按 ChemDraw 规则处理：`color="0"` 为前景色，`bgcolor="1"` 为背景色，`<colortable>` entries 从 id `2` 开始。
- CDXML import/export 统一处理 line、shape、text、text run、fragment label、页面背景和对象样式颜色。
- 扩充箭头内部语义，包括 solid/hollow/open、单双头、半边箭头、弯曲箭头、粗箭头和 no-go cross/hash。
- `render_objects/arrows.rs` 扩充到更接近 ChemDraw 的箭头渲染语义。
- 分子 label 不再当作普通 text object；结构 label 支持 `lineRuns`，保留多行和 run 信息。
- 新增文本符号和 glyph profile 路径，包括 `shared/text_symbols.json`、`viewer/text_symbol_palette.js`、glyph profile 生成和回归脚本。
- 扩充 Rust 测试，尤其是 `crates/chemcore-engine/tests/render_document.rs` 的 CDXML、文本、箭头、glyph 和渲染稳定性覆盖。

`56cdd4a` 的核心内容是把原生格式入口迁到 `.ccjs` / `.ccjz`：

- `.ccjz` 是默认产品保存格式，即 gzip 压缩后的 chemcore JSON。
- `.ccjs` 是可读调试格式，即纯文本 chemcore JSON。
- viewer 文件流支持打开 `.ccjz`、`.ccjs`、`.cdxml`，并支持保存 `.ccjz`、`.ccjs`、导出 `.cdxml` 和 `.svg`。
- 示例文件从 `examples/document-v0.1.json` 改为 `examples/document-v0.1.ccjs`。
- README、格式文档、项目规则和 viewer 渲染报告同步改成 `.ccjs/.ccjz` 术语。

### Windows 原生环境重建

今天在 Windows 上完成了工具链重建和路径整理。目标原则是能安装到 D 盘的尽量安装到 D 盘；系统已有但版本不合适的工具不做强行兼容，而是更新或安装新的 Windows 原生版本。

已确认或设置的主要工具：

```text
Git:                 D:\Git
Git version:         2.54.0.windows.1
Git Bash:            D:\Git\bin\bash.exe
Git Bash version:    GNU bash 5.3.9

Rust cargo home:     D:\Rust\cargo
Rust rustup home:    D:\Rust\rustup
Rust toolchain:      stable-x86_64-pc-windows-msvc
rustc:               1.95.0
cargo:               1.95.0
Rust targets:        x86_64-pc-windows-msvc, wasm32-unknown-unknown
wasm-pack:           0.14.0

Node installed path: D:\nodejs-24.15.0
Node installed ver.: 24.15.0
npm installed ver.:  11.12.1

Playwright browsers: D:\ms-playwright
VS Build Tools:      C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools
VS Build version:    17.14.36717.8 / 17.14.21
```

保留的一个细节：当前已运行的 Codex/PowerShell 进程仍可能解析到旧 `D:\nodejs` 和 WSL `bash.exe`，因为进程 PATH 在启动时已经固定。但用户级 PATH 已经把这些目录放到前面：

```text
D:\nodejs-24.15.0
D:\Rust\cargo\bin
D:\Git\cmd
D:\Git\bin
D:\Git\usr\bin
```

新开的 PowerShell、VS Code terminal 或重新加载后的开发环境应优先使用新的 Node、Rust 和 Git Bash。npm 的 `script-shell` 已设置为：

```text
D:\Git\bin\bash.exe
```

因此 `npm run build:engine-wasm`、`npm run verify` 这类调用 `bash scripts/*.sh` 的脚本会走 Git Bash，而不是 WSL bash。

### Git、VS Code 和 Windows 文件系统处理

已设置 Git：

```powershell
git config --global core.autocrlf false
git config --global core.eol lf
git config core.filemode false
```

目的分别是：

- 避免 Windows 自动把 LF 改成 CRLF，导致跨环境 diff 噪音。
- 避免 NTFS/WSL 挂载导致大量 `100644 => 100755` 的 file mode 假修改。
- 让 Windows 目录 `D:\chemcore` 成为主开发目录，而不是继续通过 `\\wsl$` 编辑旧 WSL 目录。

VS Code 内置 Git 功能可直接显示跟踪状态，不需要额外插件。已更新 `%APPDATA%\Code\User\settings.json`：

```json
{
  "git.path": "D:\\Git\\cmd\\git.exe",
  "git.enabled": true
}
```

可选插件只有 GitLens，不是 Git 跟踪的必要条件。若 VS Code 仍看不到 Git 状态，应确认打开的是 `D:\chemcore` 本地目录，而不是 WSL remote 窗口，并重新加载 VS Code。

Windows 复制过来的 WSL metadata 还产生了私有用区冒号文件名，例如 `*Zone.Identifier` 和 `*mshield`。今天已清理 `compare/` 下这类未跟踪文件，并在 `.gitignore` 增加规则：

```text
*Zone.Identifier
*mshield
```

### 依赖安装与验证结果

已运行 `npm install`。npm 11 会把根 package 的 `name`、`version`、`license` 写入 `package-lock.json`，因此 lockfile 出现了小幅元数据变化。安装过程中出现过旧 `.bin` 临时 symlink 清理警告，但不阻塞安装和后续验证。

已完成的验证：

```text
cargo test
npm test
npm run build:engine-wasm
node --check viewer/app.js
node --check viewer/document_flow.js
```

`cargo test` 通过结果包括：

- library unit tests: 44 passed
- `bond_tool`: 141 passed
- `render_document`: 98 passed, 2 ignored
- `text_tool`: 36 passed
- doc-tests: 0

`npm test` 通过，内容是：

```text
cargo test && node --check viewer/app.js
```

`npm run build:engine-wasm` 通过。Windows 原生重建后 `viewer/engine/chemcore_engine_bg.wasm` 发生变化，重新构建两次得到稳定 SHA-256：

```text
BC3A87EF5A2310D622F6E58D44DB351C002348A2205D527FBDDF8A52036EA62C
```

这说明差异不是随机输出，而是 Windows 原生工具链重建后的稳定生成物差异。`npm run verify` 的测试、WASM build 和 JS syntax check 均通过，最后在 generated artifact sync check 阶段因 `viewer/engine/chemcore_engine_bg.wasm` 与仓库旧生成物不同而失败。后续提交时应把该 WASM 变化作为 Windows-native rebuild artifact 一并评审。

### 本地后台和 viewer 状态

已从 `D:\chemcore` 启动本地静态文件服务：

```text
URL:      http://127.0.0.1:8766/viewer/
Process: 8632
Runtime: C:\Python314\python.exe
```

选择 `8766` 是因为 `8765` 已有其他监听进程。`Invoke-WebRequest http://127.0.0.1:8766/viewer/` 返回 `200 OK`。

### 保存问题修复

用户在 viewer 中发现不能保存。排查路径：

- 顶部保存按钮在 `viewer/index.html` 中是 `data-command="save"`。
- `viewer/editor_bindings.js` 将该按钮绑定到 `saveCurrentDocumentAs`。
- `viewer/document_flow.js` 中 `saveCurrentDocumentAs` 使用 `CHEMCORE_TEXT_EXTENSION` 生成 `.ccjs` accept list，但文件顶部没有导入该常量。

直接错误：

```text
ReferenceError: CHEMCORE_TEXT_EXTENSION is not defined
```

修复：

- 在 `viewer/document_flow.js` 导入 `CHEMCORE_TEXT_EXTENSION`。
- 调整 `saveCurrentDocumentNative`、`saveCurrentDocumentCdxml`、`saveCurrentDocumentSvg` 的顺序：在支持 `showSaveFilePicker` 的浏览器中，先打开保存框，再生成、导出或压缩内容。
- 更新 `viewer/index.html` 中 `app.js` 的查询版本号为 `20260505-savefix`，减少浏览器缓存旧脚本的概率。

验证：

```text
node --check viewer/document_flow.js
node --check viewer/app.js
```

并用 Playwright 注入假的 `showSaveFilePicker` 后点击保存按钮，确认保存链路成功写出内容：

```json
{
  "suggestedName": "Untitled.ccjz",
  "write": {
    "constructorName": "Uint8Array",
    "byteLength": 524
  },
  "closed": true
}
```

### 今日工作树变化

截至本日志写入前，今日新增或修改的主要文件是：

```text
.gitignore
package-lock.json
viewer/document_flow.js
viewer/engine/chemcore_engine_bg.wasm
viewer/index.html
docs/developer-log-2026-05-05.md
```

`HANDOFF-2026-05-05.md` 的内容已并入本日志，并已从根目录删除。

各项变化含义：

- `.gitignore`：忽略 Windows/WSL 复制产生的私有用区冒号 metadata 文件。
- `package-lock.json`：npm 11 写入根 package 元数据。
- `viewer/document_flow.js`：修复保存未导入常量，并把保存框调用提前到内容生成前。
- `viewer/index.html`：更新 `app.js` cache-busting query。
- `viewer/engine/chemcore_engine_bg.wasm`：Windows-native WASM rebuild 后的稳定生成物变化。
- `docs/developer-log-2026-05-05.md`：本双语开发者日志。

### 后续注意事项

Windows 主开发应直接使用 `D:\chemcore`。旧 WSL 目录可以作为参考备份，但不建议双线开发。切换环境前应提交或明确备份未提交改动。

如果 Windows 上出现大量行尾差异，应先停止开发，确认 Git 的 `core.autocrlf=false` 和 `core.eol=lf` 后再继续。如果出现大量 file mode 差异，应确认仓库本地 `core.filemode=false`。

如果 `npm run verify` 只因为 `viewer/engine` 生成物不同而失败，应先确认 Rust 测试、WASM 构建和 JS 语法检查都通过，再决定是否提交生成物。

`.ccjz` 依赖浏览器 `CompressionStream` / `DecompressionStream`。现代 Chromium 可用；未来如果引入 Tauri/WebView2，需要确认 WebView2 支持情况，或者在 Tauri 后端提供压缩读写。

CDXML/SVG 保真仍应继续用 `tmp/` 和 `compare/` 下 fixture 做导入、保存、再导入和 SVG 对比，重点检查对象数、类型、颜色、文本 run、结构 label、箭头参数、相对位置、线宽、字体大小和 SVG primitive。

未来桌面壳推荐继续走 Tauri：第一阶段只做窗口、菜单、快捷键、文件打开/另存为、文件关联、最近文件、拖拽打开和打包安装；不要把化学编辑逻辑迁入壳层。第二阶段再考虑 Tauri 后端直接调用 Rust engine、原生压缩读写、缩略图和系统集成。
