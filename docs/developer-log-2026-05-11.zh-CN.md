# Chemcore 开发者日志 - 2026-05-11

作者：张家骏

时间范围：2026-05-11 00:00 至 2026-05-11 23:59，Asia/Shanghai

说明：本日志覆盖当前仓库可见的 2026-05-11 已提交内容。工作区中后续未提交的实验性修改不计入本日志。

基线提交：`7c2a6bc Improve EMF preview vector clarity`

工作目录：`D:\Projects\chemcore`

### 总结

今天的提交主线很集中：Office/OLE 预览继续从“能粘贴、能显示”推进到“按 Chemcore 内部 render primitives 生成可分析、可调优的 EMF/WMF 矢量记录”。这不是为了兼容 ChemDraw 的私有对象格式，而是为了让 Word/Office 看到的嵌入对象预览，尽量接近 Chemcore 自己画布上的几何和排版。

具体来说，今天把 `windows_office.rs` 里已经膨胀的 metafile 预览逻辑拆成独立 `emf_preview` 模块，并新增专门的 GDI renderer。Office 容器层继续负责 COM、OLE storage、剪贴板格式和 presentation stream；renderer 层负责把 Chemcore document 渲染出的 line、polygon、path、ellipse、text 等 primitives 重新记录成 GDI/EMF 绘图命令。

第二个重点是清晰度。上一版 EMF 预览虽然已经不是普通位图，但在细线、键、多边形键身和箭头上仍容易出现整数坐标量化带来的锯齿和形变。今天新增了 EMF 高精度记录作用域，用 `SetWorldTransform` 把记录坐标临时放大，再由 world transform 缩回原尺寸，从而让 Office 仍按同样大小显示，但 EMF 内部记录拥有更多坐标精度。

同时，CDXML 箭头导入导出补了一轮关键语义：箭头头部尺寸不再简单按百分比直读，而是考虑 CDXML 默认线宽、当前 stroke width 和 bold arrow 的关系，避免粗箭头导入后头部比例被放大或缩小。测试中也补了 bold arrow、bracketusage objecttag 文本过滤等回归用例。

### 工作范围和代码面

今天当前仓库可见的提交范围是：

```text
feec386^..7c2a6bc
```

这段范围包含 2 个提交，合计 11 个文件变更，约 3598 行新增、1020 行删除。主要代码面如下：

```text
apps/chemcore-office/src/windows_office.rs
apps/chemcore-office/src/windows_office/emf_preview.rs
apps/chemcore-office/src/windows_office/emf_preview/renderer.rs

crates/chemcore-engine/src/cdxml/export.rs
crates/chemcore-engine/src/cdxml/import_objects.rs
crates/chemcore-engine/tests/render_document.rs

scripts/chemdraw-oracle.mjs
scripts/compare-emf-oracle.mjs
scripts/emf-inspect.mjs
scripts/render-emf-preview.mjs
package.json
```

从文件分布看，今天不是在 UI 上做表层修补，而是在 Office 输出边界重建一条更可控的矢量预览管线：engine 仍然是几何和文档语义来源，Office adapter 只消费 engine 的 document/render 输出，再把这些输出翻译成 Office 能理解的 metafile/presentation stream。

### Office 预览模块拆分

`windows_office.rs` 之前同时承担 COM server、剪贴板、OLE object、storage、Word OOXML、WMF/EMF 生成和 GDI 绘图，文件变得很难继续调。今天把 metafile 相关逻辑拆到：

```text
apps/chemcore-office/src/windows_office/emf_preview.rs
apps/chemcore-office/src/windows_office/emf_preview/renderer.rs
```

新的 `emf_preview.rs` 更像容器层。它负责根据 payload 计算 HIMETRIC extent，生成 `CF_METAFILEPICT` / `CF_ENHMETAFILE` 数据，写 `OlePres` presentation stream，管理 WMF/EMF handle 生命周期，以及为 Word OOXML package 提供 EMF bytes。这里关心的是 Office 和 OLE 需要什么格式、什么尺寸、什么 frame bounds。

`renderer.rs` 则只关心绘图。它从 payload 中解析 Chemcore document，调用 engine 的 `render_document()` 生成 primitives，过滤出 Office 预览应该出现的 `DocumentBond`、`DocumentGraphic` 和 `DocumentText`，然后用 GDI 记录 line、polyline、polygon、path、ellipse、rect、circle、text 等内容。这样之后继续调键退让、箭头、文本下标或 label 位置时，不需要再碰 COM/OLE plumbing。

这次拆分最重要的收益不是“文件变短”，而是职责边界清楚了：Office 层只包装对象，Chemcore render primitives 才是几何真相，GDI renderer 是两者之间的翻译层。

### Render Primitives 到 GDI/EMF

今天的预览路线明确选择从 Chemcore 内部渲染结果出发，而不是把 SVG 当作唯一真相。SVG 仍保留为 fallback，但优先路径是：

```text
Chemcore document JSON
  -> parse_document_json()
  -> render_document()
  -> RenderPrimitive[]
  -> GDI / EMF records
```

renderer 里新增了 `PreviewTransform`，负责把 source primitive bounds 映射到 Office 目标 DC bounds。这样复制到 Word 时，对象大小由可见内容范围和 Office extent 决定，而绘制时仍保持源文档内部的相对几何关系。

GDI replay 覆盖了今天 Office 预览最需要的一批 primitives：

```text
Line / Polyline / Polygon
Path / FilledPath
Rect / Ellipse / Circle
Text
```

路径绘制不是简单塞一张图，而是解析 SVG path command，再回放到 GDI path：`BeginPath`、`MoveToEx`、`LineTo`、`PolyBezierTo`、`CloseFigure`、`StrokePath`、`FillPath` 等。对于无法矢量回放的 SVG，才走 `resvg + tiny-skia` raster fallback，再用 `StretchDIBits` 写入 DC。

文本绘制也开始有独立的 Office 预览逻辑：按 LabelRun 拆分文本行和 run，创建 GDI font，估算 run advance，处理 subscript/superscript 的缩放和 baseline shift，并为 WMF 路径提供 ANSI fallback。这个阶段还没有完全解决“内部数字下标和画布一致”的最终问题，但今天的提交已经把文本问题收到 renderer 内部，后续可以继续围绕同一套 primitives 修。

### EMF 清晰度和记录精度

后一个提交集中处理 EMF 预览锯齿和线条发虚的问题。核心变化是为 EMF 路径增加高精度记录作用域：

```text
SetGraphicsMode(GM_ADVANCED)
SetWorldTransform(1 / record_scale)
PreviewTransform.record_scale = record_scale
```

绘图时，primitive 坐标和线宽先按 `record_scale` 放大写入 EMF 记录，再通过 world transform 缩回原来的视觉尺寸。这样 Office 看到的对象大小不变，但 EMF 内部不再被过早压到低精度整数坐标。

本次提交里记录精度按 primitive 类型区分：

```text
DocumentBond: 16x
非箭头 DocumentGraphic: 16x
line arrow 类 DocumentGraphic: 3x
Text: 1x
```

文本保持 1x 是为了避免字体高度和 GDI text metrics 被高精度 world transform 放大后引入新的布局偏差。键和图形则优先吃到高精度记录收益。

此外还修了几处线条观感：

```text
DocumentBond 线段使用 round cap
细长 bond polygon 可退化为中心线绘制
四点 bond polygon 的 centerline 也使用 round cap
CreateFontW 使用 ANTIALIASED_QUALITY
```

这里的思路是：对于很细的键身，多边形填充在 EMF/Office 里容易被量化成参差边缘；如果它本质上是细长键，可以按中心线和等效宽度记录成 GDI stroke，让 Office 的 stroke renderer 接管抗锯齿和端点处理。

### CDXML 箭头尺寸和图形语义

CDXML 箭头今天补的是一个容易被忽略但很关键的比例问题。CDXML 里的 `HeadSize`、`ArrowheadCenterSize`、`ArrowheadWidth` 不是 Chemcore 内部最终像素宽度，而是相对线宽体系的尺寸属性。以前导入时直接 `parse_scaled_100()`，导出时直接 `value * 100`，对普通箭头勉强可用，但遇到 `LineType="Bold"` 时会把 bold stroke 和箭头头部比例搞混。

今天新增了双向换算：

```text
导入：cdxml_arrow_size_for_render_scale()
导出：cdxml_arrow_size_attribute()
```

导入时根据 CDXML 默认 line width 和实际 render stroke width 把箭头尺寸转回 Chemcore 内部比例；导出时再按当前 stroke width 和 default line width 写回 CDXML 属性。这样 bold arrow 的头部在导入、渲染、导出一轮后仍保持相对 CDXML 线宽的语义。

同一块还整理了 line object 的样式派生：`LineType` 中的 `Bold` / `Dashed` 会进入 style id 和 stroke width，箭头 payload 里继续保留 head/tail endpoint、fill type、curve、no-go、bounding box、center、major/minor axis 等几何信息。对于普通 `graphic Line`，仍作为 line scene object 导入，和 arrow 区分 payload。

文本导入方面，今天补了 `bracketusage` objecttag 的过滤。CDXML bracket 内部可能带有用于 ChemDraw 参数化括号的 objecttag 文本，这类文本不是用户可见正文。导入时跳过 `Name="bracketusage"` 的 objecttag，避免把内部标记误变成 Chemcore 页面上的 text object。

### EMF Oracle 和检查工具

今天还新增了一组脚本，把“看起来像不像”往可检查、可复现的方向推进：

```text
npm run emf:chemdraw-oracle
npm run emf:compare-oracle
npm run emf:inspect
npm run emf:render-preview
```

`chemdraw-oracle.mjs` 通过 Windows COM 调用 `ChemDraw.Application`，批量打开 CDXML/CDX，并导出 ChemDraw 的 SVG/EMF 作为外部参考。它默认寻找 `tmp/color.cdxml`、`tmp/arrows-acs.cdxml`、`tmp/kuohao.cdxml`、`tmp/硫氰基化反应条件.cdxml` 等本地 fixture。

`emf-inspect.mjs` 是 EMF record parser。它读取 EMF bytes，统计 record type，解析 header frame/bounds、pen、brush、font、text、polyline、polygon、path、stretch dibits、world transform 等关键记录，并能输出 JSON 和 Markdown summary。

`render-emf-preview.mjs` 用 PowerShell + `System.Drawing.Imaging.Metafile` 把 EMF 渲染成 PNG，方便把 ChemDraw EMF 和 Chemcore EMF 放到同一目录下肉眼对比。

`compare-emf-oracle.mjs` 把这些步骤串起来：先生成 ChemDraw oracle，再用 Chemcore 的 `cdxml_to_svg`、`cdxml_to_clipboard_payload` 和 office `--write-emf-payload` 生成 Chemcore EMF，随后 inspect 两边 record counts，并渲染 PNG 预览，最后写 summary。这个工具链的意义不是盲目照抄 ChemDraw，而是让 Office/EMF 的差异可以被记录、被定位、被回归。

### 测试和验证

提交中新增和扩展了 engine 层渲染测试，重点覆盖：

```text
CDXML bold arrow head dimensions 仍相对 CDXML LineWidth
小尺寸箭头头部不被额外 floor 放大
CDXML arrow geometry modifiers 导入导出保留
bracketusage objecttag 文本不会误导入为可见 text object
```

这些测试主要保护 CDXML import/export 和 render primitive 输出，不直接测试 Word 中的最终显示。Office 侧今天更多依靠新增脚本工具做外部检查：生成 EMF、inspect record、渲染 PNG，再和 ChemDraw/Office 看到的结果对比。

### 后续注意事项

今天的提交把 Office EMF 预览的工程边界立住了，但还有几类问题需要继续沿这条线往下做：

```text
文本 run 的真实宽度、下标/上标位置和画布渲染仍需要继续对齐。
label 与 bond 的退让应尽量复用 engine 已经计算出的 render primitives，而不是在 EMF 层二次裁剪。
箭头和 reaction graphic 的 record scale 还需要根据 Word 实测继续调。
Office extent 和 visible bounds 需要继续确认是否完全按“包裹所有内容的最小边框”工作。
```

总体上，今天的方向是正确的：不要把 Office 预览当作一张截图，也不要在 Office adapter 里重写化学几何。Chemcore 内部 render primitives 是唯一绘图事实；EMF/WMF 只是把这个事实交给 Office 的载体。
