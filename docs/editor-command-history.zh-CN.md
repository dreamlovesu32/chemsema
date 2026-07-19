# 编辑器命令历史

本文档定义编辑器内核使用的命令层。当前实现把每一次已经提交的编辑操作记录为命令和文档快照：

```text
HistoryEntry {
  command: EditorCommand,
  before: ChemSemaDocument,
  after: ChemSemaDocument
}
```

命令对象是稳定的语义记录。`before` 和 `after` 文档保证 undo/redo 行为精确，同时允许命令接口继续演进。后续单个命令可以把完整快照替换为更小的 patch 或反向操作，而不改变编辑器事件模型。

## 规则

只有已经提交的文档变化才是命令。

Pointer hover、focus halo、preview bond、lasso outline、active drag state 和文本光标移动都是 history 之外的交互状态。

## 当前命令

### `add-bond`

从一个锚点到另一个锚点创建键。任一锚点都可以引用已有节点，也可以引用会创建或复用节点的世界坐标。

记录数据：

- `begin`：锚点节点 id 和世界坐标
- `end`：锚点节点 id 和世界坐标
- `order`：键级
- `variant`：当前键变体

典型来源：

- 使用键工具点击空白画布
- 从端点点击或拖拽
- 从聚焦点拖拽

### `cycle-bond-style`

修改已有键中心的样式、键级或立体状态。

记录数据：

- `bond_id`
- `variant`：当前键变体

典型来源：

- 使用单键、双键、三键、虚键、粗键、锲形键等工具点击键

### `delete-selection`

删除当前选择；如果没有选择，则按命令键语义删除当前聚焦项。

选择删除语义：

- 删除选中的键
- 只有当选中键完全覆盖端点原始度数时，才删除这些键的端点
- 选中的原子会和相连键一起删除
- 相邻原子保留
- 选中的标签转换回碳原子

典型来源：

- `Delete`
- `Backspace`

### `delete-focused-at-point`

删除指针位置处的聚焦项。

记录数据：

- `x`, `y`：世界坐标
- `source`：`delete-tool` 或 `command-key`

Delete 工具和命令键删除刻意保持分离，因为它们的端点语义不同。

### `cut-selection`

把当前选择复制到编辑器内部剪贴板，然后把删除选择作为一个可撤销命令提交。

典型来源：

- `Ctrl/Cmd+X`

### `paste-clipboard`

把编辑器内部剪贴板粘贴到可编辑分子中。

典型来源：

- 粘贴工具栏按钮
- `Ctrl/Cmd+V`

### `insert-template`

提交一个结构模板。

记录数据：

- `template`：模板 id，例如 `ring-6` 或 `benzene`
- `x`, `y`：提交点

典型来源：

- 使用模板工具点击或拖拽

对于 `benzene`，六条环边必须形成一组严格交替的三根双键。以已有键为锚点插入时，共边必须复用，不能重复创建。共边原本是双键时，它直接计入三根双键，并把侧双键位置移入新插入的环；共边是已有交替六元环中的单键时，内核重新排布原环，使共边成为两个并环共同复用的那根双键。

### `apply-selection-arrange`

对选择应用布局命令。

记录数据：

- `command`：工具栏命令 id

当前命令 id：

- `align-left`
- `align-right`
- `align-top`
- `align-bottom`
- `align-h-center`
- `align-v-center`
- `distribute-h`
- `distribute-v`
- `flip-h`
- `flip-v`

### `apply-selection-color`

给当前选择应用颜色。

记录数据：

- `color`：归一化 hex 颜色字符串

当前行为：

- 选中的文本对象更新文本填充样式和富文本 run 填充
- 选中的分子标签更新 label 和 run 填充
- 选中的分子节点或键更新分子 style color
- 选中的 line、bracket、symbol 和 shape 对象更新 stroke 和/或 fill style color

### `move-targets`, `rotate-targets`, `scale-targets`, `delete-targets`

对显式目标集合执行命令脚本编辑，不依赖 GUI 当前选区。

记录数据：

- `targets`：节点、键、scene object 和标签节点
- `move-targets` 的 `delta`
- `rotate-targets` 的 `center` 和 `degrees`
- `scale-targets` 的 `scaleX`、`scaleY` 和可选 `pivot`

### `move-selection`

移动当前选中的分子部分。

命令在第一次改变文档的拖拽更新时打开，并持续刷新它的 `after` 快照，直到最终 mouse-up 位置。

### `rotate-selection`

旋转当前选中的分子部分。

命令在第一次改变文档的旋转更新时打开，并持续刷新它的 `after` 快照，直到最终 mouse-up 角度。

### `apply-text-edit`

应用当前 active text edit session。

记录目标：

- `text-object`：可选对象 id
- `endpoint-label`：节点 id

### `replace-hovered-endpoint-label`

把 hovered endpoint 替换为输入的原子或缩写标签。

记录数据：

- `label`

### `legacy-mutation`

当某个文档变化仍在命令上下文之外调用低层 snapshot API 时使用的兜底命令。它应被视为迁移警告：新的编辑功能应使用语义命令。

## 临时动作

以下动作是临时 UI/runtime 动作：

- `copy-selection`：只改变内部剪贴板
- `select-targets`、`select-all` 和 `clear-selection`：只改变当前内存选区，
  除非后面接文档变更命令
- `set-tool`
- `set-template`
- hover/focus 更新
- preview 生成
- viewport zoom 和 pan
- open/load document，它会重置 history

## 实现说明

所有已提交 mutation 都应在 `Engine::with_command` 内运行。现有低层 mutation helper 仍可以调用 `push_undo_snapshot`；命令上下文会把当前语义命令分配给该 snapshot。

如果一个用户命令创建多个内部 snapshot，命令层会用第一个 `before` 文档和最终 `after` 文档把它们合并成一个 `HistoryEntry`。

对于拖拽命令，中间更新可能在第一次 snapshot 之后继续修改文档。命令层会刷新最近匹配 history entry 的 `after` 文档，使 redo 回到最终 pointer-up 状态。
