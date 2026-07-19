# 编辑器交互反馈规则

本文定义 ChemSema 编辑器里 hover、聚焦、预览和临时拖拽层的视觉反馈规则。目标是在小文件和大文件里都保持一致、及时、可预测。

## 视觉控制点

- 普通对象控制点使用空心圆，视觉半径为 1.5 CSS px。
- endpoint hover 在需要显示时，也使用 1.5 CSS px 的视觉半径。
- endpoint 命中范围独立于视觉大小，命中半径保持 10 CSS px。
- 选中框 resize 点和箭头端点样式控制点属于独立交互系统，继续使用各自规则。

## Endpoint 反馈

Endpoint hover 是化学键编辑反馈，不是通用对象创建反馈。

- 键工具在绘制或延伸键时，可以显示 endpoint hover。
- 键工具拖动绘制时，可以显示末端预览点。
- 非键对象创建工具不得显示 endpoint hover 圆圈或末端预览点，除非该命令直接以原子端点或相连标签为目标。
- 非键对象创建工具仍然可以在内部使用 endpoint 作为放置锚点，但这种锚定不能产生 endpoint hover 视觉反馈。
- 符号、文本和删除工具的命令会直接操作这些化学对象，因此保留各自的端点或标签反馈。

## 临时层

编辑器里不止一个临时视觉层：

- 内核 interaction render list；
- editor overlay layer；
- canvas drag preview layer；
- 文档 preview transform 和 mask。

任何完成、取消或放弃的指针交互，都必须清掉它可能触碰过的所有临时层。已经过期的 animation frame 或异步 pointer move，不允许在提交之后把旧 hover 或旧 preview 重新画回来。

本地拖动预览中，每个文档对象的位移只能应用一次。如果 SVG 对象外层容器和内部图元重复使用同一个 `data-object-id`，只能给最外层的匹配 DOM 节点施加 preview transform。选择框、提交后的文档几何和可见对象必须跟随同一份指针位移。

## 回归要求

覆盖对象创建和大文件编辑的测试应断言：

- 普通对象控制点和 endpoint hover 使用配置好的视觉半径；
- 非键对象工具 hover 到原子时不显示 endpoint hover 视觉，除非该命令直接以原子端点或相连标签为目标；
- 所有电荷和电子符号变体都会聚焦裸端点与相连标签字形；
- 箭头、电荷符号及其他文档对象严格按指针位移移动，选择框始终覆盖可见对象；
- 对象创建 pointer-up 后，所有临时层里的 hover/preview role 都被清掉；
- 清理临时反馈不需要触发整份文档 render list 刷新。
