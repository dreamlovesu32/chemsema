# Runtime Discovery

Use runtime discovery before assuming CLI paths, protocol details, or command
options. The public `chemsema-cli` skill must work without asking users to
install Rust, Cargo, Node, or a source checkout.

## Finding ChemSema CLI

Preferred order:

1. `$env:CHEMSEMA_CLI` if set and executable.
2. `chemsema-cli` on `PATH`.
3. The skill-bundled runtime declared in `assets/runtime-manifest.json`.
4. The default skill-bundled runtime at `assets/bin/<platform>/chemsema-cli`
   or `assets/bin/<platform>/chemsema-cli.exe` on Windows.

Use `scripts/find_chemsema_cli.ps1` for this lookup. Use
`scripts/run_chemsema_cli.py` when a task should work from either an installed
runtime or the self-contained skill runtime.

Do not run `cargo`, `npm`, or repo-local build commands from this skill. If the
user is working inside a ChemSema source checkout and needs build/test/package
behavior, switch to the `chemsema-development` skill.

## Bundled Runtime Layout

Package prebuilt runtimes inside the skill:

```text
assets/
  runtime-manifest.json
  bin/
    win-x64/
      chemsema-cli.exe
    macos-arm64/
      chemsema-cli
    macos-x64/
      chemsema-cli
    linux-x64/
      chemsema-cli
```

`runtime-manifest.json` maps each supported platform tag to a path under
`assets/`. Platform tags use `<os>-<arch>`, such as `win-x64`,
`macos-arm64`, and `linux-x64`.

## First Commands

Run:

```powershell
chemsema-cli version --pretty
chemsema-cli doctor --pretty
chemsema-cli capabilities --pretty
chemsema-cli about --pretty
chemsema-cli examples basic --pretty
chemsema-cli schema protocol --pretty
chemsema-cli guide --kind agent --pretty
chemsema-cli guide --kind detailed --out chemsema-guide.json --pretty
```

For large payloads, request `--out` and read the file.

Use `about` for installed entrypoints and packaging notes. Use `examples` for
ready-to-run command scripts and CLI workflows.

## Invocation Modes

Use one-shot commands for independent tasks:

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --pretty
chemsema-cli run input.cdxml commands.json --out edited.ccjs --results results.json --pretty
```

Use JSONL session mode for repeated work on one document:

```powershell
chemsema-cli session input.cdxml
```

Session mode keeps the document in memory and supports `targets`, `detail`,
`context`, `capture`, `execute`, `save`, `status`, `close`, and `exit`.

## Cache Notes

CDXML/CDX one-shot import uses a normalized import cache. Use these variables
only when needed:

```powershell
$env:CHEMSEMA_CLI_DISABLE_CACHE = "1"
$env:CHEMSEMA_CLI_CACHE_DIR = "D:\tmp\chemsema-cli-cache"
```

Check effective settings with `doctor`.
