# ChemSema 全面代码审查（2026-07-23）

## 结论

仓库的化学规则单测基础已经很强，但“对象能力闭环”和“前端混合内核状态契约”还没有达到可持续交付的程度。

本次基线为 `5568fb7`。Rust 全工作区测试通过，Viewer 交互冒烟和桌面混合内核延迟回归通过；完整 GUI 回归稳定失败在复制/粘贴/剪切链路。这说明当前主要风险不是某个化学公式没有单测，而是同一个对象或命令在导入、选择、编辑、渲染、导出及前端同步链路中只有部分实现。

机器审查入口：

```text
npm run audit:core-contracts
```

该命令在存在 error 时返回非零。只生成报告、不阻断时可运行：

```text
node scripts/audit-core-contracts.mjs --report docs/core-contract-audit-2026-07-23.zh-CN.md
```

## P0：应在继续扩展功能前修复

### 1. CDXML curve 是半实现对象

`curve` 已经可以从 CDXML 导入，也可以绘制和导出，但没有完整进入对象操作链路：

- 不能通过统一对象选择规则选中；
- `select all` 不包含它；
- 剪贴板完整性判断不认识它；
- 旋转没有处理 `curvePoints`；
- 缩放没有处理 `curvePoints`；
- 删除依赖选择，因此也不能形成完整行为；
- 没有覆盖这些操作的回归测试。

更深层的问题是 CCJS v0.1 并没有声明顶层 `curve` 类型。当前文档把曲线定义为 `line` 的一种，而导入器创建了新的顶层对象类型。这里必须做一次明确设计：

1. 推荐：把 CDXML `<curve>` 规范化为 `type: "line"` 加明确的 curve kind/payload；或
2. 正式把 `curve` 加入 CCJS 规范，并为所有对象操作表补齐显式分支。

不能继续让未知对象落入 `_ => {}`、`_ => true` 或通用位移分支。

### 2. 混合内核读写缺少同步屏障，已经触发 GUI 错误

完整 GUI 回归在以下链路失败：

```text
全选 → 复制 → 粘贴 → 全选 → 剪切
```

失败点是剪切后 12 秒内 DOM 仍未清空。

原因链路已经定位到 `viewer/engine_host.js`：

- `selectAll()` 先修改本地 WASM，再异步推送 native；
- `clipboardSelectionJson()`、`clipboardDocumentJson()`、`clipboardCdxml()` 和 `hasClipboard()` 直接读取 native；
- 这些读取没有等待 `nativeBackgroundOperation`；
- `cutSelection()` 又走 native mutation。

因此剪贴板读取、全选和剪切看到的可能不是同一个 revision。此前“分子量不显示”也是同一类本地/远端状态权威不明确的问题。

需要建立统一规则：

- 同一 revision 内，本地 WASM 是即时交互和选择状态的权威；
- 任何必须读取 native 的 API，先等待所有影响该读取的本地 mutation；
- 所有 getter 明确标注读取 local、native 或 committed snapshot；
- 禁止各 getter 自己决定缓存优先级。

### 3. 仍存在真正的语义 fallback

以下不是“文档默认值”，而是在缺少权威数据时换了一套行为：

- CDXML 没有坐标时，`fallback_cdxml_topology_positions` 会凭拓扑生成布局；
- 标签缺少 glyph polygons 时，键退让会改用标签矩形；
- Viewer 对未知字形自行推断矩形 glyph profile；
- Viewer 另有一套字符宽度估算，和内核文本布局形成双规则；
- render/export/object transform 对未知对象静默跳过或当作成功；
- 多处 `.catch(() => {})` 会吞掉异步失败。

允许的只是明确规则，例如“字段缺失时使用该 CDXML 版本定义的默认值”。不允许“规则 A 不可用就换算法 B”。

建议把所有当前使用 `fallback` 命名的 60 个候选逐项归类为：

- documented default：改名为 `default_*` 或 `inherited_*`，并链接字段规范；
- explicit compatibility branch：名称中带格式和版本，配证据测试；
- forbidden fallback：删除，改为明确错误、不可绘制诊断或保真 opaque object。

### 4. 对象模型文档与实现漂移

除 `curve` 外，导入器和编辑器还使用顶层 `symbol`，但 CCJS v0.1 支持对象列表没有它。

这会导致“实现能跑、格式规范却无法完整解释文件”的状态。对象类型必须有单一注册表，至少驱动：

- CCJS schema；
- 导入/导出；
- render dispatch；
- hit test/selection；
- move/rotate/resize；
- style/context menu；
- group/link/order；
- copy/cut/paste/delete；
- inspector/CLI；
- 测试矩阵。

新增对象时，缺任一必需能力应由门禁直接失败。

## P1：架构收敛

### 1. 核心命令分派过大

`Engine::execute_command` 约 583 行，已经同时承担命令解码、权限判断、行为调用、结果包装和 revision/历史语义。Rust 的穷尽 match 能防漏枚举，却不能防不同分支在历史、selection、targets、错误处理上产生差异。

建议按命令域拆成明确 handler：

- document/file；
- selection/arrange；
- molecule/bond/atom；
- graphics/image；
- text；
- style/property；
- group/link。

公共提交语义保留在唯一外层，不允许每个 handler 自己决定历史和 revision。

### 2. 已发现 16 组完全重复函数

高风险重复包括：

- molecule connected components、component bounds、label geometry translation 在 CDXML 与 document 层各有一份；
- `render_primitive_role` 在 engine/render/SVG 三处重复；
- polygon bounds、polygon anchor、label glyph anchor 在编辑与文本编辑层重复；
- point-in-polygon 在 engine 与 selection geometry 重复；
- text width estimation 在 render bounds 与 SVG 重复；
- arrow distance helper 在 editing 与 renderer 重复。

这些当前完全相同，但未来只改一处就会产生“同一对象在选择框、SVG、屏幕绘制中行为不同”。应移动到最小共享规则模块，EMF 的明确格式差异继续保留在 Office 层。

### 3. 大文件和大闭包集中在前端

主要热点：

- `viewer/app.js`：约 4084 行；
- `createEditorPointerController`：约 1582 行；
- `createEditorOverlayRenderer`：约 769 行；
- `createEditorViewportHost`：约 739 行；
- `createDocumentFlow`：约 702 行；
- `createTextEditorController`：约 678 行；
- `createCanvasContextMenuHost`：约 588 行；
- `createBrowserDocumentTabs`：约 557 行。

这些大闭包通过 options 注入大量可变状态，类型和生命周期只能靠调用者默契维持，是“每次展示又冒出新 bug”的主要结构原因。拆分时应按状态机而不是按文件长度：

- pointer gesture lifecycle；
- selection gesture；
- creation gesture；
- text edit session；
- document tab lifecycle；
- clipboard transaction；
- render patch transaction。

每个状态机只允许一个开始、更新、提交、取消出口。

## P1：前端可靠性

### 1. 异步错误处理不统一

机器审查找到：

- 6 处明确吞异常；
- 15 个没有 `try/catch` 或统一安全执行器的 async DOM 事件；
- window、tab、toolbar、text editor 和 palette 都存在未处理 rejection 的入口。

应只有一个 UI action runner，负责：

- 禁止同一 action 重入；
- 捕获并记录错误；
- 恢复 loading/drag/selection 状态；
- 给用户可理解的错误；
- 在测试环境把错误重新抛出，使回归必然失败。

### 2. 本地/缓存 getter 仍不一致

除已修复的 selection chemistry summary 外，下列 getter 仍只读 native cache：

- document colors；
- document style preset；
- can undo；
- can redo。

这意味着按钮可用性、颜色和样式显示仍可能落后于用户刚完成的本地操作。应由统一 snapshot/revision API 提供，不再逐个补 getter。

### 3. 最近新增的重要行为缺少浏览器端回归

内核已有较完整的 image 单测，包括选择、缩放、旋转、复制粘贴、撤销、跨文档粘贴、focus 和 transient drag。但是浏览器/桌面层仍没有覆盖：

- 图片拖入；
- 图片复制粘贴；
- 空白处右键插入图片；
- 插入后自动切换选择工具并选中；
- focus 框与 selection 框一致；
- 拖拽结束不保持选中；
- 跨标签页、Web/桌面之间的结构化剪贴板；
- 桌面标签页拖出生成新窗口。

这些必须是端到端回归，因为内核单测无法验证浏览器事件、文件选择器、系统剪贴板和窗口生命周期。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| Rust workspace tests | PASS |
| Engine tests | PASS（264 passed, 2 ignored） |
| Viewer JS syntax | PASS |
| Viewer interaction smoke | PASS |
| Desktop hybrid latency regression | PASS |
| GUI selection summary/minimum box case | PASS |
| Full GUI regression | FAIL：copy/paste/cut 链路，cut 后 DOM 未清空 |
| Core contract audit gate | FAIL：存在 error，符合预期 |

## 建议修复顺序

1. 先建立对象类型注册表，并决定 `curve`/`symbol` 的 CCJS 归属。
2. 补齐 curve 的选择、变换、剪贴板、删除和测试。
3. 建立 hybrid revision/read barrier，修复完整 GUI copy/paste/cut。
4. 删除四类已确认语义 fallback 和未知对象静默分支。
5. 把前端 async action 统一到一个执行器，并补 image/tab/clipboard 端到端测试。
6. 合并完全重复规则，再拆 `execute_command` 和前端状态机大闭包。
7. 最后逐项清理 60 个 fallback 命名候选；只有有官方文档或版本证据的默认分支可以保留。
