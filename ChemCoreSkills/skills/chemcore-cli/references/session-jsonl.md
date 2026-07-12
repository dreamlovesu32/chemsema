# JSONL Session

Use session mode when many operations target one document.

```powershell
chemcore-cli session input.cdxml
```

Write one JSON request per stdin line and read one JSON response per stdout
line:

```jsonl
{"id":1,"op":"targets"}
{"id":2,"op":"detail","target":"molecule:0"}
{"id":3,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":4,"op":"bundle","target":"molecule:0","outDir":"molecule-0-bundle","captureFormat":"svg"}
{"id":5,"op":"exit"}
```

Responses include `ok`, `id`, `op`, and either `result` or `error`.

## Operations

Stable operations include:

- `open`
- `targets`
- `detail`
- `context`
- `bundle`
- `capture`
- `execute`
- `save`
- `status`
- `close`
- `exit`

`execute` accepts normal command objects/arrays and
`chemcore.command-transaction.v1` envelopes. Put transaction fields directly in
the execute request, or pass a nested `transaction` object. Use transaction
`dryRun:true` to get execution, diff, and scope validation without mutating the
open session document.

## Script Helper

Use `scripts/session_jsonl.py`:

```powershell
python scripts\session_jsonl.py input.cdxml requests.jsonl --out transcript.jsonl
```

The helper starts `chemcore-cli session`, sends all non-empty request lines,
adds an `exit` request if needed, and writes response lines to the transcript.
