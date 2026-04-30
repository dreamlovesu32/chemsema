# chemcore 格式 v0.1

## 范围

本文档定义 `chemcore` 第一版持久化文档格式。

`0.1` 版本会刻意收窄范围。它是一个面向渲染和未来编辑的文档 / 对象格式，不是完整的化学交换标准。

它当前的直接目的包括：

- 表示单页化学页面
- 支持只读渲染
- 接收从 CDXML 提取得到的转换结果
- 作为后续 runtime 和编辑逻辑的基础

## 格式总览

文件是一个 JSON 文档，包含 5 个顶层区段：

- `format`
- `document`
- `styles`
- `objects`
- `resources`

从职责上看：

- `document` 定义全局元数据和页面设置
- `styles` 存放可复用的渲染样式
- `objects` 存放场景图节点
- `resources` 存放可复用的化学载荷，例如 `molecule_fragment2d`

## 顶层结构

```json
{
  "format": {
    "name": "chemcore",
    "version": "0.1",
    "unit": "pt"
  },
  "document": {},
  "styles": {},
  "objects": [],
  "resources": {}
}
```

## 坐标系统

`0.1` 版本使用单一世界坐标系：

- 原点：左上角
- x 轴向右增长
- y 轴向下增长
- 单位：印刷点数（`pt`，1/72 inch），在 `format.unit` 中保存为 `"pt"`

文件里不应该出现任何 backend 专属的像素假设。

## 对象身份

文档中的每个对象都必须拥有全局唯一的 `id`。

规则：

- object id 是字符串
- style id 是字符串
- resource id 是字符串
- 引用一律通过 id 完成，不能依赖数组位置

## 收纳规则

`0.1` 版本使用严格的对象树来表达归属关系。

规则：

- 每个对象必须且只能属于一个容器
- 容器只能是顶层 `objects` 数组，或某一个 `group.children` 列表
- 一个对象最多只能有一个直接父 `group`
- 同一个对象不能同时出现在顶层和某个 group 内
- 同一个对象不能同时出现在多个 `group.children` 列表里

这样可以让归属、遍历、选择和编辑行为保持确定性。

## 对象模型

每个场景对象共享一层通用包络：

```json
{
  "id": "obj_001",
  "type": "molecule",
  "name": "optional human label",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_default",
  "meta": {},
  "payload": {}
}
```

### 通用字段

- `id`：唯一对象 id
- `type`：对象类型
- `name`：可选的人类可读标签，便于调试或 UI 展示
- `visible`：对象是否参与渲染
- `locked`：对象是否可编辑
- `zIndex`：全局堆叠顺序键
- `transform`：局部变换
- `styleRef`：可选的样式 id
- `meta`：不影响关键渲染的元数据
- `payload`：类型专属数据

### v0.1 支持的对象类型

- `molecule`
- `text`
- `line`
- `bracket`
- `shape`
- `group`

其他图形原语以后再增加。

## 对象类型基线

`0.1` 版本应该先从一组小而稳定的一等对象类型开始：

- `molecule`：带化学语义的 2D 结构
- `text`：带定位的富文本
- `line`：线性笔画对象，包括箭头
- `bracket`：括号类图形对象
- `shape`：简单的填充或描边区域
- `group`：逻辑组合和共同变换

这个拆分是刻意的：

- `molecule` 负责化学语义
- `text`、`line`、`bracket`、`shape` 负责文档图形
- `group` 只负责收纳和变换

重要原则：属于 `molecule` 的 label 不是普通 `text` 对象。
例如 `CN`、`Ph`、`N3`、`t-Bu`、`HN`，以及像 `H` 在上、`N` 在下这种杂原子标签，它们本质上都是结构标签，具备：

- 标签内部的连接锚点
- 相对连接键的朝向
- 受化学规则约束的字符顺序
- 可选的行内上下标格式
- 可选的多行 run 数据，例如 `lineRuns`。当结构标签需要上下分行显示时，
  仍然可以保留逐 token 的样式，比如 `SO2` 里的下标 `2`
- 归一化后的显示 runs 应当只保留化学上有意义的行内格式，例如上下标；
  不应直接把 CDXML `face` 这类源格式里的字重/字形样式原样当作结构标签显示规则
- 但为了保真，原始 source runs 仍然可以保留；它们应放在
  `meta.import.<source>` 下，而不是和归一化显示字段并列

这类内容应当保留在分子资源或分子专属 payload 里，而不是建模成独立的文档文本框。

Viewer 说明：渲染器可以在显示阶段做小幅且有上限的光学校正，例如把
`attached-group` 标签与附近原子标签轻微拉开。这类调整只属于 viewer
行为，不应回写到文档存储几何里。

`v0.1` 中，括号先作为独立对象，不并入 `molecule`。它经常围绕化学结构出现，但本质上仍然先是文档对象。以后如果需要承载聚合物、重复单元或分组语义，再通过元数据或新的化学对象扩展。

## Transform

所有对象都可以携带局部变换：

```json
"transform": {
  "translate": [120, 40],
  "rotate": 0,
  "scale": [1, 1]
}
```

规则：

- `translate` 必填
- `rotate` 默认为 `0`
- `scale` 默认为 `[1, 1]`

在 `v0.1` 中，局部到世界坐标的变换顺序为：

1. scale
2. rotate
3. translate

## Styles

样式单独存放，通过 `styleRef` 引用。

示例：

```json
"styles": {
  "style_text_default": {
    "kind": "text",
    "fontFamily": "Helvetica",
    "fontSize": 12,
    "fontWeight": 400,
    "fill": "#111111",
    "stroke": null
  },
  "style_line_default": {
    "kind": "stroke",
    "stroke": "#222222",
    "strokeWidth": 1.5,
    "lineCap": "round",
    "lineJoin": "round"
  }
}
```

`v0.1` 除了 `kind` 之外，不强制一套死板的样式分类；但 renderer 应当预期样式主要描述以下几类外观：

- 文本外观
- 线条 / 填充外观
- 分子外观

## Resources

`resources` 用来放那些不适合内联到每个对象里的可复用内容块。

`v0.1` 目前明确规定一种资源类型：

- `molecule_fragment2d`

示例：

```json
"resources": {
  "mol_a": {
    "type": "molecule_fragment2d",
    "encoding": "chemcore.molecule.fragment2d",
    "data": {}
  }
}
```

这样可以让 molecule 对象保持轻量，也为重复引用留出空间。

## Molecule 对象

molecule 对象表示页面上一个已定位、带化学语义的结构。

示例：

```json
{
  "id": "obj_mol_1",
  "type": "molecule",
  "visible": true,
  "locked": false,
  "zIndex": 10,
  "transform": {
    "translate": [96, 72],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_molecule_default",
  "meta": {
    "source": "cdxml",
    "collapsed": false
  },
  "payload": {
    "resourceRef": "mol_a",
    "bbox": [0, 0, 88, 42],
    "role": "substrate"
  }
}
```

### Molecule Payload 字段

- `resourceRef`：必填，指向一个 `molecule_fragment2d` 资源
- `bbox`：可选，局部包围盒
- `role`：可选语义提示，例如 `substrate`、`product`、`ligand`

`v0.1` 还不在对象模型里编码完整反应语义。`role` 只是提示字段。

## Text 对象

text 对象表示带定位信息的富文本内容。

示例：

```json
{
  "id": "obj_text_1",
  "type": "text",
  "visible": true,
  "locked": false,
  "zIndex": 20,
  "transform": {
    "translate": [220, 88],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_text_default",
  "meta": {},
  "payload": {
    "text": "PhB(OH)2 (1.0 equiv)",
    "box": [0, 0, 120, 18],
    "align": "left",
    "valign": "middle"
  }
}
```

### Text Payload 字段

- `text`：必填纯文本内容
- `box`：可选，局部文本框
- `align`：`left | center | right`
- `valign`：`top | middle | bottom`
- `runs`：可选，行内富文本片段

### 富文本支持

`v0.1` 的文本应至少能表达：

- 字体族
- 字号
- 字重 / 斜体
- 上标
- 下标
- 符号和特殊字符

推荐的行内模型：

```json
"runs": [
  {
    "text": "SO",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "script": "normal"
  },
  {
    "text": "4",
    "fontFamily": "Arial",
    "fontSize": 10,
    "fill": "#000000",
    "fontWeight": 700,
    "fontStyle": "normal",
    "script": "subscript"
  }
]
```

`script` 可取 `normal | subscript | superscript`。CDXML 的 `face`、`font`、
`color` 这类源格式字段应在导入阶段解码成这些显式字段；原始值可以保留在
`meta.import.cdxml` 中，用于调试或未来 round-trip。

## Molecule Fragment2D

`molecule_fragment2d` resource 用局部坐标保存节点和键。字段应直接表达化学
语义和渲染意图，而不是暴露源格式的位掩码。

节点 label 示例：

```json
{
  "id": "n1",
  "element": "N",
  "atomicNumber": 7,
  "position": [47.4, 29.96],
  "charge": 0,
  "numHydrogens": 0,
  "label": {
    "text": "N",
    "sourceText": "N",
    "position": [43.79, 33.86],
    "box": [43.79, 25.52, 51.01, 33.86],
    "layout": "default",
    "anchor": "start",
    "runs": [
      {
        "text": "N",
        "fontFamily": "Arial",
        "fontSize": 10,
        "fill": "#000000",
        "fontWeight": 400,
        "fontStyle": "normal",
        "script": "normal"
      }
    ]
  }
}
```

键示例：

```json
{
  "id": "b1",
  "begin": "n1",
  "end": "n2",
  "order": 1,
  "stereo": {
    "kind": "solid-wedge",
    "wideEnd": "end"
  }
}
```

```json
{
  "id": "b2",
  "begin": "n2",
  "end": "n3",
  "order": 2,
  "double": {
    "placement": "right"
  }
}
```

分子 label 字段：

- `text`：归一化后的显示文本
- `sourceText`：可选，化学重排前的原始 label 文本
- `position`：局部 label 点
- `box`：局部 label 包围盒
- `layout`：label 布局模式，例如 `default`、`attached-group`、
  `attached-group-above` 或 `centered-atom`
- `anchor`：label 内部连接锚点，通常是 `start | center | end`
- `runs`：归一化显示 runs
- `lineRuns`：可选，逐渲染行的归一化 runs
- `glyphPolygons`：可选，局部坐标系下的逐字形 optical polygon；存在时，
  renderer 可优先用它做 label knockout 和 bond clipping，而不是只用粗颗粒
  的 `box`

键字段：

- `order`：数字键级
- `stereo.kind`：`solid-wedge | hashed-wedge`
- `stereo.wideEnd`：`begin | end`
- `double.placement`：`left | right | center`

## Line 对象

line 对象表示页面上的线性笔画几何。

它应覆盖：

- 实线
- 虚线
- 折线
- 曲线
- 半箭头
- 全箭头

示例：

```json
{
  "id": "obj_line_1",
  "type": "line",
  "visible": true,
  "locked": false,
  "zIndex": 15,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_line_default",
  "meta": {},
  "payload": {
    "kind": "polyline",
    "points": [[260, 120], [380, 120]],
    "head": "end",
    "tail": "none",
    "arrowHead": {
      "kind": "solid",
      "head": "full",
      "tail": "none",
      "length": 18,
      "centerLength": 14,
      "width": 5
    }
  }
}
```

### Line Payload 字段

- `kind`：必填几何类型，例如 `line | polyline | curve`
- `points`：必填，局部坐标系中的控制点
- `head`：`none | start | end | both`
- `tail`：`none | start | end | both`
- `arrowHead`：可选箭头装饰数据；省略或为 `null` 就是普通线
- `curve`：可选，bezier 或弧线等曲线元数据

line 的外观主要放在样式里，包括：

- 描边颜色
- 描边宽度
- 虚线模式
- line cap
- line join

因此，箭头在模型里应当被看作同一个 `line` 对象上的线端装饰，而不是单独的顶层对象类型。

## Bracket 对象

bracket 对象表示独立的括号图形。

它应覆盖：

- 小括号：`()`
- 中括号：`[]`
- 大括号：`{}`

示例：

```json
{
  "id": "obj_bracket_1",
  "type": "bracket",
  "visible": true,
  "locked": false,
  "zIndex": 12,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_bracket_default",
  "meta": {
    "semanticRole": "annotation"
  },
  "payload": {
    "kind": "square",
    "side": "left",
    "box": [180, 60, 12, 80]
  }
}
```

### Bracket Payload 字段

- `kind`：`round | square | curly`
- `side`：`left | right | pair`
- `box`：必填，用于拟合括号几何的局部包围盒

`v0.1` 中，括号先按文档图形处理。如果以后它需要承载聚合物、重复单元或结构分组语义，再通过显式元数据或新的化学对象扩展。

## Shape 对象

shape 对象表示简单的填充或描边区域。

它应覆盖：

- `circle`
- `ellipse`
- `rect`
- `roundRect`

示例：

```json
{
  "id": "obj_shape_1",
  "type": "shape",
  "visible": true,
  "locked": false,
  "zIndex": 8,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_shape_default",
  "meta": {},
  "payload": {
    "kind": "roundRect",
    "box": [420, 80, 160, 64],
    "radius": 8
  }
}
```

### Shape Payload 字段

- `kind`：`circle | ellipse | rect | roundRect`
- `box`：必填，局部包围盒
- `radius`：可选，`roundRect` 的圆角半径

shape 的外观主要放在样式里，包括：

- 填充颜色
- 描边颜色
- 描边宽度
- 虚线模式
- 是否填充

## Group 对象

group 对象用于组织 children，但自身不携带可见几何。

示例：

```json
{
  "id": "obj_group_1",
  "type": "group",
  "visible": true,
  "locked": false,
  "zIndex": 5,
  "transform": {
    "translate": [0, 0],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": null,
  "meta": {
    "kind": "reaction_block"
  },
  "payload": {
    "children": ["obj_mol_1", "obj_line_1", "obj_text_1"]
  }
}
```

### Group Payload 字段

- `children`：必填，有序的子对象 id 列表

children 会继承 group 的变换。

## Group 语义

在 `v0.1` 里，`group` 的定义会刻意收窄：

- `group` 只负责组织归属和共同变换
- `group` 不创建独立 stacking context
- `group` 不决定重叠时谁在前谁在后
- `group` 不是图层
- `group` 本身不要求拥有可见几何
- 顶层 `objects` 只应包含没有父 group 的根对象

这样可以把“组合”和“重叠顺序”彻底拆开。

## Document 区段

`document` 区段存放全局元数据和页面设置。

示例：

```json
"document": {
  "id": "doc_001",
  "title": "example reaction page",
  "page": {
    "width": 1024,
    "height": 768,
    "background": "#ffffff"
  },
  "meta": {
    "createdBy": "chemcore",
    "sourceFormat": "cdxml"
  }
}
```

### Document 字段

- `id`：文档 id
- `title`：可选标题
- `page.width`：必填
- `page.height`：必填
- `page.background`：可选
- `meta`：可选通用元数据

## 渲染顺序示例

对象绘制顺序定义为：

1. `zIndex` 升序
2. 同级容器中的稳定顺序作为第二排序键

后绘制的对象，在发生重叠时显示在前面。

group 不替代子对象排序；它只负责变换作用域和归属关系。

## 重叠与堆叠顺序

如果两个可见对象发生部分重叠，前后显示顺序只由堆叠顺序决定，不能由对象类型或重叠面积来推断。

规则：

- `zIndex` 更大的对象显示在前面
- 当两个对象的 `zIndex` 相同时，同级顺序更靠后的对象显示在前面
- 渲染本质上是有序绘制，后画的覆盖先画的
- 是否属于同一个 `group` 不改变这套规则

## v0.1 的约束

`0.1` 版本刻意不包含：

- 多页
- 内嵌二进制资源
- 原生 reaction graph 语义
- query chemistry 语义
- 编辑历史
- viewport 状态
- selection 状态
- 协作元数据

这些能力等到底层模型被验证之后，再进入后续版本。

## 文件扩展名

当前推荐扩展名为：

- `.chemcore.json`

这样可以明确表达：

- 载荷是 JSON
- schema 仍处于演进阶段

## 兼容性承诺

`0.1` 是一个不稳定的开发中格式。

当前只承诺：

- 字段应保持显式
- 一旦生成，id 应尽量稳定
- 应可通过带版本号的迁移过程完成升级

目前还不保证向后兼容。
