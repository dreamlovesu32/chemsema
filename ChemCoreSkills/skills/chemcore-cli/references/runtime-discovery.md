# Runtime Discovery

Use runtime discovery before assuming CLI paths, protocol details, or command
options. The public `chemcore-cli` skill must work without asking users to
install Rust, Cargo, Node, or a source checkout.

## Finding ChemCore CLI

Preferred order:

1. `$env:CHEMCORE_CLI` if set and executable.
2. `chemcore-cli` on `PATH`.
3. The skill-bundled runtime declared in `assets/runtime-manifest.json`.
4. The default skill-bundled runtime at `assets/bin/<platform>/chemcore-cli`
   or `assets/bin/<platform>/chemcore-cli.exe` on Windows.

Use `scripts/find_chemcore_cli.ps1` for this lookup. Use
`scripts/run_chemcore_cli.py` when a task should work from either an installed
runtime or the self-contained skill runtime.

Do not run `cargo`, `npm`, or repo-local build commands from this skill. If the
user is working inside a ChemCore source checkout and needs build/test/package
behavior, switch to the `chemcore-development` skill.

## Bundled Runtime Layout

Package prebuilt runtimes inside the skill:

```text
assets/
  runtime-manifest.json
  bin/
    win-x64/
      chemcore-cli.exe
    macos-arm64/
      chemcore-cli
    macos-x64/
      chemcore-cli
    linux-x64/
      chemcore-cli
```

`runtime-manifest.json` maps each supported platform tag to a path under
`assets/`. Platform tags use `<os>-<arch>`, such as `win-x64`,
`macos-arm64`, and `linux-x64`.

## First Commands

Run:

```powershell
chemcore-cli version --pretty
chemcore-cli doctor --pretty
chemcore-cli capabilities --pretty
chemcore-cli about --pretty
chemcore-cli examples basic --pretty
chemcore-cli schema protocol --pretty
chemcore-cli guide --kind agent --pretty
chemcore-cli guide --kind detailed --out chemcore-guide.json --pretty
```

For large payloads, request `--out` and read the file.

Use `about` for installed entrypoints and packaging notes. Use `examples` for
ready-to-run command scripts and CLI workflows.

## Invocation Modes

Use one-shot commands for independent tasks:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --pretty
chemcore-cli run input.cdxml commands.json --out edited.ccjs --results results.json --pretty
```

Use JSONL session mode for repeated work on one document:

```powershell
chemcore-cli session input.cdxml
```

Session mode keeps the document in memory and supports `targets`, `detail`,
`context`, `capture`, `execute`, `save`, `status`, `close`, and `exit`.

## Cache Notes

CDXML/CDX one-shot import uses a normalized import cache. Use these variables
only when needed:

```powershell
$env:CHEMCORE_CLI_DISABLE_CACHE = "1"
$env:CHEMCORE_CLI_CACHE_DIR = "D:\tmp\chemcore-cli-cache"
```

Check effective settings with `doctor`.
