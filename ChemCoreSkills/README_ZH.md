# ChemCore Skills

这是一组面向 ChemCore 生态的 Codex skills，目录结构参考 MindScienceSkills
的学科树，但定位是 ChemCore 的核心能力：化学文档、可编辑对象、渲染内核、
Office/OLE、CLI 协议和 OCR 逆渲染。

## 技能列表

- `chemistry_and_materials/chemical_documents/chemcore-cli`
  - ChemCore CLI、协议、selector、capture、command script、label-query。
- `chemistry_and_materials/chemical_documents/chemcore-office`
  - Office/OLE payload、Word/PowerPoint 粘贴、可编辑对象调试。
- `chemistry_and_materials/chemical_documents/chemcore-ocr-reconstruction`
  - PNG 到 ChemCore JSON/command stream、结构门禁、分子池回归。
- `chemistry_and_materials/chemical_documents/chemcore-drawing-agent`
  - 给绘制 agent 用的 `plan-bond`、`plan-template`、label-query 工作流。
- `research_tools/development/chemcore-development`
  - ChemCore 编译、测试、WASM、桌面包、CI、仓库卫生。

## 设计原则

- skill 的 `SKILL.md` 保持轻量，复杂规则放入 `references/` 按需读取。
- CLI 是机器契约，优先使用 `version`、`schema`、`capabilities`、`guide`
  做运行时发现。
- OCR 的目标是可编辑 ChemCore 文档，不是像素拟合。
- 绘制 agent 应该问内核要吸附、标签、模板落点，不要手写 GUI 几何。
- Office 调试先看 payload，再看 Office 粘贴结果。

## 平铺安装

Codex skill 通常需要每个 skill 作为 `$CODEX_HOME/skills` 下的直接子目录。
使用平铺脚本生成安装目录：

```powershell
powershell -ExecutionPolicy Bypass -File .\flatten_skills.ps1 -OutDir $env:CODEX_HOME\skills
```

也可以输出到临时目录检查：

```powershell
powershell -ExecutionPolicy Bypass -File .\flatten_skills.ps1 -OutDir .\.generated\skills
```
