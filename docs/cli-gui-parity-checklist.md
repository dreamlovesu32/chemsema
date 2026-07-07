# ChemCore CLI and GUI Parity Checklist

This checklist tracks GUI editing capabilities that must stay reachable from
`chemcore-cli` command scripts and JSONL `session execute`, plus the skill docs
agents use to discover them.

Status key:

- Done: implemented, documented, and covered by focused tests or existing tests.
- Partial: implemented but missing broader docs, aliases, or coverage.
- Planned: known gap.

| Area | GUI capability | CLI command path | Skill/docs status | Status |
| --- | --- | --- | --- | --- |
| Selection | Single select | `select-targets` with one target | `chemcore-cli` skill command-scripts reference | Done |
| Selection | Multi-select | `select-targets` with multiple target ids | `chemcore-cli` skill command-scripts reference | Done |
| Selection | Select all | `select-all` | `chemcore-cli` skill command-scripts reference | Done |
| Selection | Clear selection | `clear-selection` | `chemcore-cli` skill command-scripts reference | Done |
| Transform | Move selection/targets | `move-targets`; or `select-targets` + selection commands | CLI guide and skill reference | Done |
| Transform | Rotate selection/targets | `rotate-targets` with explicit targets, center, and degrees | CLI guide and skill reference | Done |
| Transform | Scale uniformly | `scale-targets` with equal factors; `select-targets` + `scale-selection` | CLI guide and skill reference | Done |
| Transform | Stretch non-uniformly | `scale-targets` with unequal `scaleX`/`scaleY` | CLI guide and skill reference | Done |
| Arrange | Align selected items | `select-targets` + `apply-selection-arrange` | CLI guide and skill reference | Done |
| Arrange | Distribute selected items | `select-targets` + `apply-selection-arrange` | CLI guide and skill reference | Done |
| Arrange | Flip selected items | `select-targets` + `apply-selection-arrange` | CLI guide and skill reference | Done |
| Z order | Bring/send front/back | `apply-selection-order` with ids or current selection | CLI guide | Done |
| Grouping | Group | `group-selection` with ids or current selection | CLI guide and tests | Done |
| Grouping | Ungroup | `ungroup-selection` with ids or current selection | CLI guide | Done |
| Links | Link bracket/text | `link-selection` with ids or current selection | CLI guide | Done |
| Links | Unlink bracket/text | `unlink-selection` with ids or current selection | CLI guide | Done |
| Styling | Text style/alignment | `apply-text-style` with ids or current selection | CLI guide and skill reference | Done |
| Styling | Bond style | `apply-bond-style` with ids or current selection | CLI guide and skill reference | Done |
| Styling | Object settings/bond width | `apply-object-settings-to-selection` with ids or current selection | CLI guide and skill reference | Done |
| Styling | Shape/bracket/orbital/line styles | explicit ids or current selection where applicable | CLI guide and skill reference | Done |
| Color | Apply selection color | `select-targets`/`select-all` + `apply-selection-color` | Tests cover select-all path | Done |
| Delete/Cut | Delete current selection | `select-targets` + `delete-selection`, or `delete-targets` | Protocol docs and skill reference | Done |
| Delete/Cut | Cut current selection | `select-targets` + `cut-selection` | Protocol docs and skill reference | Done |
| Labels | Expand labels | `select-targets` + `expand-labels-in-selection` | Protocol docs mention selection path | Partial |
| Chemistry | Chemical check toggle | `select-targets` + `set-chemical-check-for-selection` | Needs explicit guide example | Partial |
| Session | Repeated edits on one document | `chemcore-cli session` + `execute` | Session protocol and skill reference | Done |
| One-shot | Stateless script edits | `chemcore-cli new` / `chemcore-cli run` | Command-script protocol and skill reference | Done |

When adding a GUI editing command, update this checklist, the command JSON
implementation, `docs/chemcore-cli-guide.md`, `docs/chemcore-cli-guide.zh-CN.md`,
the relevant `docs/protocol/*.md`, and `ChemCoreSkills/skills/chemcore-cli`.
