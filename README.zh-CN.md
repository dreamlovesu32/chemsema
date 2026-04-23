# chemcore

`chemcore` 是一个跨平台的化学文档核心。

这个项目的目标不是“先做一个网页 demo，之后再重写成桌面版”。从第一天开始，
它就要定义一套稳定的文档核心：

- 平台无关的文档模型
- 稳定的文件格式
- 适合编辑和渲染的运行时场景模型
- 从 CDXML 等旧工具导入的路径
- 面向浏览器和桌面宿主的渲染后端

第一阶段的实现范围依然会刻意收窄：

- 先把格式边界定义清楚
- 先做到可读、可写
- 先做到可显示
- 继续把 CDXML 解析保留为导入路径

## 当前范围

当前 [`src/chemcore/cdxml`](./src/chemcore/cdxml) 下面的代码提供的是第一批导入侧基础：

- CDXML 提取入口：`extract_cdxml`
- 从 CDXML 几何中提取分子
- 提取文本 / 表格 / 箭头
- 通过 SDF 匹配为分子补充 `smiles` 和 `molblock2d`
- 对 2D 结构做立体后处理

这里目前只覆盖了解析侧。它还不是 `chemcore` 的 renderer、editor，也不是最终的文档序列化器。

## 设计文档

当前的设计基线在下面这些文件里：

- [README.md](./README.md)
- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
- [docs/architecture.zh-CN.md](./docs/architecture.zh-CN.md)
- [docs/format-v0.1.zh-CN.md](./docs/format-v0.1.zh-CN.md)
- [examples/document-v0.1.json](./examples/document-v0.1.json)

这几份文件共同构成了下一步开发的工作约定：

- 把导入数据转换成 `chemcore` 文档模型
- 用第一个 backend 把这个模型渲染出来
- 在引入编辑能力之前，先验证 round-trip 行为

## 工作区结构

```text
chemcore/
  README.md
  README.zh-CN.md
  docs/
    architecture.md
    architecture.zh-CN.md
    format-v0.1.md
    format-v0.1.zh-CN.md
  examples/
    document-v0.1.json
  src/
    chemcore/
      __init__.py
      cdxml/
        __init__.py
        extract_cdxml.py
        cdxml_layout.py
        cdxml_molecule.py
        cdxml_sdf_match.py
        cdxml_shared.py
        cdxml_stereo.py
```

## Conda 环境

环境名：`chemcore`

推荐的创建命令：

```bash
conda create -y -n chemcore python=3.11 rdkit -c conda-forge
```

激活命令：

```bash
conda activate chemcore
```

## 最小用法

在 `/home/jiajun/chemcore` 目录下：

```bash
PYTHONPATH=src python -c "from chemcore import extract_cdxml; print(extract_cdxml('/path/to/base_without_cdxml_suffix'))"
```

`extract_cdxml()` 接收的是不带 `.cdxml` 后缀的基础路径，同时要求旁边存在配套的 `.sdf` 文件，这和当前导入链路保持一致。

## 近期计划

1. 把提取出的 CDXML 对象映射到 `chemcore` v0.1 文档模型
2. 基于该模型实现第一个只读渲染后端
3. 验证对象身份、坐标、样式引用和 z-order
4. 之后再开始最小编辑操作
