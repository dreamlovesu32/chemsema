# ChemCore CLI JSONL Session v1

Protocol id: `chemcore-cli-session-jsonl.v1`.

`chemcore-cli session [input]` starts a long-lived process over stdin/stdout.
The first stdout line is a ready event. After that, callers send one compact
JSON request per line and read one compact JSON response per line.

## Request Shape

```json
{"id":1,"op":"targets"}
```

Stable fields:

- `id`: optional caller value echoed in the response.
- `op`: operation name.

Stable operations:

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

## Response Shape

Successful responses include:

```json
{"ok":true,"id":1,"op":"targets","result":{}}
```

Failed responses include:

```json
{"ok":false,"id":1,"op":"targets","error":{"kind":"operation_failed","message":"..."}}
```

## History Policy

The session keeps the current document in memory. It does not persist an undo
stack or full snapshot history. `execute` responses report before/after
revision and per-command results; callers should maintain durable history with
git, temporary files, or their own logs.

## Selection State

`execute` accepts the same command JSON as one-shot `new` and `run`, including
`chemcore.command-transaction.v1` envelopes directly in the request body or
under a `transaction` field.
Selection state persists in the session until another command changes it, the
document is closed, or the process exits. Use `select-targets`, `select-all`,
and `clear-selection` before GUI-style selection commands such as
`apply-selection-arrange`, `scale-selection`, `center-selection-on-page`,
`apply-selection-color`, grouping, linking, style commands, delete, and cut.

For stateless edits that should not depend on current selection, use explicit
target commands such as `move-targets`, `rotate-targets`, `scale-targets`, and
`delete-targets`.

Transaction requests execute on a cloned engine state first. They may declare
hash/revision preconditions, editable scope, `dryRun`, and postconditions, and
they report structured diff plus unexpected scope changes.

## File Outputs

`bundle`, `capture`, and `save` may write files. File writes are verified before
success is reported. Prefer explicit output paths in automation.
