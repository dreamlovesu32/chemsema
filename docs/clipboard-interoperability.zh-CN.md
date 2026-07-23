# 剪贴板与多窗口互通规则

## 目标

ChemSema 不要求 ChemDraw 能回读 ChemSema 私有格式，但 ChemSema 必须尽可能把 ChemDraw、Word 嵌入对象和其他 CDX/CDXML 编辑器复制出的内容恢复成可编辑结构。只有不存在可读结构时，才允许退化为图片。

## 粘贴优先级

1. `ChemSema Clipboard Fragment` 或 HTML 中的 ChemSema portable payload。该路径无损保留分子、文本、箭头、括号、图形、图片、组合及其资源。
2. `ChemSema Document JSON`。外部文档作为对象集合粘入当前文档，不替换当前标签页。
3. `ChemDraw Interchange Format`、`chemical/x-cdxml` 或 OLE storage 中的 CDX/CDXML。CDX 的 `VjCD0100` 二进制先转为 CDXML，再进入同一导入链路。
4. Unicode 文本中的 CDXML。
5. PNG、DIB/BMP 等图片。

格式损坏时必须继续尝试下一层，不能因为一个不可读的私有格式阻断后面的标准结构格式。

## Web、桌面、标签页和窗口

- 同一页面内的标签页共享 portable payload；每个标签页拥有独立内核也不影响复制粘贴。
- Web 剪贴板同时写 `text/html` 与 `text/plain`：HTML 携带完整 ChemSema payload，纯文本携带所选对象的 CDXML。
- Windows 桌面剪贴板同时提供 ChemSema 私有格式、CF_HTML、CDXML、Unicode 文本、OLE/EMF。CF_HTML 使 WebView/浏览器能够无损回读 ChemSema 对象。
- 桌面标签页拖出后生成独立窗口。窗口之间不共享内存剪贴板，统一通过 Windows 剪贴板互通。
- 复制时导出的 CDXML 必须来自当前选择，不得错误地导出整个文档。

## Office 与 ChemDraw

- Word 中复制 ChemSema 或 ChemDraw 嵌入对象时，桌面端通过 `OleGetClipboard` 读取 `IDataObject`。
- 优先读取 `Embedded Object` / `Embed Source` storage；ChemDraw `CONTENTS` stream 中的 CDX 二进制按正式 CDX 头识别。
- 直接从 ChemDraw 或兼容编辑器复制时，同时接受文本 CDXML 与二进制 CDX 的 `ChemDraw Interchange Format`。
- OLE、CDX/CDXML 都不可读时才读取 Office 提供的位图预览。

## 回归要求

- 多分子文档跨标签页粘贴后，分子对象数量和各自资源不得被压成一个 fragment。
- 图片跨标签页/窗口粘贴后，图片对象和二进制资源必须同时存在。
- CDXML 粘贴后必须产生可编辑节点和键，而不是图片对象。
- HTML portable payload 必须覆盖单标签页、跨浏览器标签页、Web→桌面和桌面→Web。
- 标签页拖出新窗口后，未保存文档状态、缩放和跨窗口粘贴均应保持可用。
