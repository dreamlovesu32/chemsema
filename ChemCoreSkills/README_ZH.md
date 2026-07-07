# ChemCore Skills

这是一组面向 ChemCore 生态的 agent skills，可安装到 Codex 或 Claude Code。
源码 skill 统一放在 `ChemCoreSkills/skills` 下；每个可安装 skill 都是一个
独立目录，并以 `SKILL.md` 作为入口。当前覆盖 ChemCore 的核心能力：
化学文档、可编辑对象、渲染内核、Office/OLE、CLI 协议和绘制 agent。

## 技能列表

`skills/chemcore-cli` 是主要对外 skill。普通 agent 使用时优先安装这个。

- `skills/chemcore-cli`
  - ChemCore CLI、协议、selector、capture、command script、selection/target
    editing、label-query 和 JSONL session。
- `skills/chemcore-office`
  - Office/OLE payload 诊断、Word/PowerPoint 粘贴检查、可编辑对象调试。
- `skills/chemcore-drawing-agent`
  - 给绘制 agent 用的 `plan-bond`、`plan-template`、label-query 工作流。
- `skills/chemcore-development`
  - ChemCore 编译、测试、WASM、桌面包、CI、仓库卫生。

可选专项 skill 覆盖更窄的工作流：`chemcore-office` 支持 clipboard/OLE
粘贴诊断，`chemcore-drawing-agent` 支持带 planning query 的命令脚本生成，
`chemcore-development` 支持仓库维护者和贡献者工作流。

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
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir $env:CODEX_HOME\skills
```

Linux/macOS 或 Git Bash:

```bash
./ChemCoreSkills/flatten_skills.sh "${CODEX_HOME:-$HOME/.codex}/skills"
```

安装后重启 Codex，让新 skill 被重新发现。

## 安装到 Claude Code

Claude Code 也支持以 `SKILL.md` 为入口的 skills。如果希望这套 skill 跟随当前仓库，
可以平铺到项目内 `.claude/skills`：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir .\.claude\skills
```

如果希望作为个人 Claude Code skills 全局可用，可以安装到：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir "$HOME\.claude\skills"
```

Git Bash / Linux / macOS:

```bash
./ChemCoreSkills/flatten_skills.sh .claude/skills
./ChemCoreSkills/flatten_skills.sh "$HOME/.claude/skills"
```

在 Claude Code 里可以直接用 `/chemcore-cli`、`/chemcore-office`、
`/chemcore-drawing-agent` 或 `/chemcore-development` 调用；自然语言请求匹配
description 时，Claude 也可以自动加载。

也可以输出到临时目录检查：

```powershell
$out = Join-Path $env:TEMP "chemcore-skills"
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir $out -Clean
Get-ChildItem $out
```

期望看到 4 个直接子目录：

- `chemcore-cli`
- `chemcore-development`
- `chemcore-drawing-agent`
- `chemcore-office`

## 远程安装路径

如果通过 Codex 的 skill installer 从 GitHub 安装，需要分别安装每个 skill 路径，
因为本仓库把多个 skill 作为一个 suite 管理：

```text
ChemCoreSkills/skills/chemcore-cli
ChemCoreSkills/skills/chemcore-office
ChemCoreSkills/skills/chemcore-drawing-agent
ChemCoreSkills/skills/chemcore-development
```

使用内置 installer helper 时，多个路径应放在同一个 `--path` 后面：

```powershell
python install-skill-from-github.py --repo dreamlovesu32/chemcore --path `
  ChemCoreSkills/skills/chemcore-cli `
  ChemCoreSkills/skills/chemcore-office `
  ChemCoreSkills/skills/chemcore-drawing-agent `
  ChemCoreSkills/skills/chemcore-development
```

## 校验

检查 CLI skill 文档是否覆盖当前运行时暴露的 commands 和 formats：

```powershell
python .\ChemCoreSkills\skills\chemcore-cli\scripts\check_cli_skill_sync.py --suite-root .\ChemCoreSkills --json
```

检查开发 helper 是否可用：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\skills\chemcore-development\scripts\chemcore_check.ps1 -Help
```

完整仓库验证：

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\skills\chemcore-development\scripts\chemcore_check.ps1 -All
```
