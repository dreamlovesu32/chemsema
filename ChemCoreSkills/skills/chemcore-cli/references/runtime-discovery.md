# Runtime Discovery

Use runtime discovery before assuming CLI paths, protocol details, or command
options.

## Finding ChemCore CLI

Preferred order:

1. `$env:CHEMCORE_CLI` if set and executable.
2. `chemcore-cli` on `PATH`.
3. The current repo's `target/release/chemcore-cli.exe`.
4. The current repo's `target/debug/chemcore-cli.exe`.
5. `npm run cli --` or `cargo run -p chemcore-cli --` from the repo root.

Use `scripts/find_chemcore_cli.ps1` for this lookup. Use
`scripts/run_chemcore_cli.py` when a task should work from either an installed
build or a checkout.

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
