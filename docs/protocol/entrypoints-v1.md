# ChemSema Entrypoints v1

Schema id: `chemsema.entrypoints.v1`.

Installed desktop builds ship a self-description file named
`chemsema-entrypoints.json`. It is intended for tools that discover ChemSema
without repository context.

## Stable Sections

- `schema`
- `product`
- `entrypoints`
- `packaging`
- `documentation`
- `formats`
- `agentWorkflow`

`entrypoints.cli` describes `chemsema-cli.exe`, installed path hints, and first
commands to run. `entrypoints.gui` describes the desktop executable and file
associations. `entrypoints.officeOleHelper` describes the Office/OLE helper.

## Discovery

Installed agents should run:

```powershell
chemsema-cli version --pretty
chemsema-cli guide --pretty
chemsema-cli doctor --pretty
chemsema-cli capabilities --pretty
```

When PATH is unavailable, callers can inspect installed path hints from
`chemsema-entrypoints.json` or Windows App Paths registration.
