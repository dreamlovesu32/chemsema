# ChemCore Skills

This suite contains agent skills for the ChemCore ecosystem, installable in
Codex or Claude Code. Source skills live under `ChemCoreSkills/skills`, and
each installable skill is its own folder with a `SKILL.md` entrypoint.

## Skills

- `skills/chemcore-cli`
  - ChemCore CLI, protocol discovery, selectors, capture, command scripts,
    label-query, and JSONL sessions.
- `skills/chemcore-office`
  - Office/OLE payloads, Word and PowerPoint paste behavior, editable object
    debugging, and clipboard verification.
- `skills/chemcore-ocr-reconstruction`
  - PNG-to-ChemCore JSON/command reconstruction, structure gates, molecule-pool
    regression, and failure taxonomy.
- `skills/chemcore-drawing-agent`
  - Drawing-agent workflows for `plan-bond`, `plan-template`, `label-query`,
    template insertion, and GUI-compatible command scripts.
- `skills/chemcore-development`
  - ChemCore build, test, WASM, desktop package, CI, release, and repository
    hygiene workflows.

## Install From A Checkout

Codex expects installed skills to be direct children of `$CODEX_HOME/skills`.
Use the flatten script to copy the nested skill folders into an installable
directory.

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir $env:CODEX_HOME\skills
```

Bash:

```bash
./ChemCoreSkills/flatten_skills.sh "${CODEX_HOME:-$HOME/.codex}/skills"
```

Restart Codex after installation so the new skills are discovered.

## Install For Claude Code

Claude Code also supports skills built around a `SKILL.md` entrypoint. Install
ChemCore skills into a project-local `.claude/skills` directory when you want
them to travel with this checkout:

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir .\.claude\skills
```

Or install them as personal Claude Code skills:

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir "$HOME\.claude\skills"
```

Bash:

```bash
./ChemCoreSkills/flatten_skills.sh .claude/skills
./ChemCoreSkills/flatten_skills.sh "$HOME/.claude/skills"
```

In Claude Code, invoke the skills directly with `/chemcore-cli`,
`/chemcore-office`, `/chemcore-ocr-reconstruction`,
`/chemcore-drawing-agent`, or `/chemcore-development`. Claude can also load
them automatically when a request matches each skill description.

## Dry Run Installation

Write the flattened skills to a temporary directory first:

```powershell
$out = Join-Path $env:TEMP "chemcore-skills"
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\flatten_skills.ps1 -OutDir $out -Clean
Get-ChildItem $out
```

Expected direct child folders:

- `chemcore-cli`
- `chemcore-development`
- `chemcore-drawing-agent`
- `chemcore-ocr-reconstruction`
- `chemcore-office`

## Remote Installation

If installing through a Codex skill installer from GitHub, list each installable
skill path explicitly because this repository packages them as a suite:

```text
ChemCoreSkills/skills/chemcore-cli
ChemCoreSkills/skills/chemcore-office
ChemCoreSkills/skills/chemcore-ocr-reconstruction
ChemCoreSkills/skills/chemcore-drawing-agent
ChemCoreSkills/skills/chemcore-development
```

With the bundled installer helper, pass all paths after one `--path` flag:

```powershell
python install-skill-from-github.py --repo dreamlovesu32/chemcore --path `
  ChemCoreSkills/skills/chemcore-cli `
  ChemCoreSkills/skills/chemcore-office `
  ChemCoreSkills/skills/chemcore-ocr-reconstruction `
  ChemCoreSkills/skills/chemcore-drawing-agent `
  ChemCoreSkills/skills/chemcore-development
```

## Validation

Check that the CLI-facing skill documentation is still in sync with the runtime
commands and formats:

```powershell
python .\ChemCoreSkills\skills\chemcore-cli\scripts\check_cli_skill_sync.py --suite-root .\ChemCoreSkills --json
```

Check that the development helper is available:

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\skills\chemcore-development\scripts\chemcore_check.ps1 -Help
```

For a full repository verification, run:

```powershell
powershell -ExecutionPolicy Bypass -File .\ChemCoreSkills\skills\chemcore-development\scripts\chemcore_check.ps1 -All
```
