# ChemSema Skills

这是一组面向 ChemSema 生态的 agent skills，可安装到 Codex 或 Claude Code。
源码 skill 统一放在 `ChemSemaSkills/skills` 下；每个可安装 skill 都是一个
独立目录，并以 `SKILL.md` 作为入口。当前覆盖 ChemSema 的核心能力：
化学文档、可编辑对象、渲染内核、Office/OLE、CLI 协议和绘制 agent。

## 技能列表

`skills/chemsema-cli` 是主要对外 skill。普通 agent 使用时优先安装这个。
它可以作为 self-contained skill 发布，在 `assets/bin/<platform>` 内置预编译
`chemsema-cli`，所以普通用户不需要安装 Rust、Cargo、Node，也不需要源码仓库。

- `skills/chemsema-cli`
  - ChemSema CLI、协议、selector、capture、command script、selection/target
    editing、label-query 和 JSONL session。
- `skills/chemsema-office`
  - Office/OLE payload 诊断、Word/PowerPoint 粘贴检查、可编辑对象调试。
- `skills/chemsema-drawing-agent`
  - 给绘制 agent 用的 `plan-bond`、`plan-template`、label-query 工作流。
- `skills/chemsema-development`
  - ChemSema 编译、测试、WASM、桌面包、CI、仓库卫生。

可选专项 skill 覆盖更窄的工作流：`chemsema-office` 支持 clipboard/OLE
粘贴诊断，`chemsema-drawing-agent` 支持带 planning query 的命令脚本生成，
`chemsema-development` 支持仓库维护者和贡献者工作流。

## 设计原则

- skill 的 `SKILL.md` 保持轻量，复杂规则放入 `references/` 按需读取。
- CLI 是机器契约，优先使用 `version`、`schema`、`capabilities`、`guide`
  做运行时发现。
- 绘制 agent 通过内核获取吸附、标签和模板落点，保持和 GUI 几何一致。
- Office 调试先看 payload，再看 Office 粘贴结果。

## 平铺安装

Codex skill 通常需要每个 skill 作为 `$CODEX_HOME/skills` 下的直接子目录。
使用平铺脚本生成安装目录：

```powershell
powershell -ExecutionPolicy Bypass -File .\flatten_skills.ps1 -OutDir $env:CODEX_HOME\skills
```

从仓库根目录运行时也可以这样写：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\flatten_skills.ps1 -OutDir $env:CODEX_HOME\skills
```

Linux/macOS 或 Git Bash:

```bash
./ChemSemaSkills/flatten_skills.sh "${CODEX_HOME:-$HOME/.codex}/skills"
```

安装后重启 Codex，让新 skill 被重新发现。

当前 `chemsema-cli` skill 已内置 Windows x64 和 Linux x64 runtime，分别位于
`assets/bin/win-x64` 与 `assets/bin/linux-x64`。Linux runtime 通过 Ubuntu/WSL
执行 `npm run cli:ubuntu:test` 构建和冒烟测试。做 skill-only 用户分发时，必须
保留 `assets/` 目录；如果目标平台暂时没有内置 runtime，再让用户安装
ChemSema CLI 并放入 `PATH`，或设置 `CHEMSEMA_CLI`。

当前内置的 Windows runtime 尚未代码签名。发布 skill-only 压缩包时，同时发布
`SHA256SUMS.txt`，保留 `assets/runtime-manifest.json`，并提醒用户安装前校验
checksum。不想运行内置 runtime 的用户可以把 `CHEMSEMA_CLI` 指向自己信任的
可执行文件。

## 安装到 Claude Code

Claude Code 也支持以 `SKILL.md` 为入口的 skills。如果希望这套 skill 跟随当前仓库，
可以平铺到项目内 `.claude/skills`：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\flatten_skills.ps1 -OutDir .\.claude\skills
```

如果希望作为个人 Claude Code skills 全局可用，可以安装到：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\flatten_skills.ps1 -OutDir "$HOME\.claude\skills"
```

Git Bash / Linux / macOS:

```bash
./ChemSemaSkills/flatten_skills.sh .claude/skills
./ChemSemaSkills/flatten_skills.sh "$HOME/.claude/skills"
```

在 Claude Code 里可以直接用 `/chemsema-cli`、`/chemsema-office`、
`/chemsema-drawing-agent` 或 `/chemsema-development` 调用；自然语言请求匹配
description 时，Claude 也可以自动加载。

也可以输出到临时目录检查：

```powershell
$out = Join-Path $env:TEMP "chemsema-skills"
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\flatten_skills.ps1 -OutDir $out -Clean
Get-ChildItem $out
```

期望看到 4 个直接子目录：

- `chemsema-cli`
- `chemsema-development`
- `chemsema-drawing-agent`
- `chemsema-office`

## 远程安装路径

如果通过 Codex 的 skill installer 从 GitHub 安装，需要分别安装每个 skill 路径，
因为本仓库把多个 skill 作为一个 suite 管理：

```text
ChemSemaSkills/skills/chemsema-cli
ChemSemaSkills/skills/chemsema-office
ChemSemaSkills/skills/chemsema-drawing-agent
ChemSemaSkills/skills/chemsema-development
```

使用内置 installer helper 时，多个路径应放在同一个 `--path` 后面：

```powershell
python install-skill-from-github.py --repo dreamlovesu32/chemsema --path `
  ChemSemaSkills/skills/chemsema-cli `
  ChemSemaSkills/skills/chemsema-office `
  ChemSemaSkills/skills/chemsema-drawing-agent `
  ChemSemaSkills/skills/chemsema-development
```

## 校验

构建本地 unsigned skill-only 压缩包：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\package_chemsema_cli_skill.ps1 -OutDir .\dist\chemsema-skills -Clean
```

检查 CLI skill 文档是否覆盖当前运行时暴露的 commands 和 formats：

```powershell
python .\ChemSemaSkills\skills\chemsema-cli\scripts\check_cli_skill_sync.py --suite-root .\ChemSemaSkills --json
```

检查开发 helper 是否可用：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\skills\chemsema-development\scripts\chemsema_check.ps1 -Help
```

完整仓库验证：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemSemaSkills\skills\chemsema-development\scripts\chemsema_check.ps1 -All
```
