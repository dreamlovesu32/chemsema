# ChemSema 1.0.0-beta.1

这是 ChemSema 名称下的第一个公开 beta。版本保留既有 Git 历史，同时完成仓库、Rust
crate、桌面端与 Office 应用、CLI、agent skills、文档、生成的 WASM bindings 和公开
网址的统一改名。

主要内容：

- 产品显示名统一为 `ChemSema`，代码与包标识统一为 `chemsema`。
- 完整继承此前 beta 版本的编辑器、桌面端、Office/OLE、CLI 和 agent 工作流，包括
  最新的化学标签显示改进。
- 为此前公开的 GitHub 仓库链接和 Pages 链接保留永久兼容入口，并加入本地及定时监测。
- 重新构建带有 ChemSema 身份的 Windows x64 安装包和 CLI 产物。

Windows 安装包暂未进行代码签名，安装时可能出现 SmartScreen 提醒。
