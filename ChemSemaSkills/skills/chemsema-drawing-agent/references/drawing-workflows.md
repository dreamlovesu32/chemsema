# Drawing Workflows

## New Molecule

1. Query labels that will be used.
2. Use `plan-bond` for standard chain growth.
3. Use `plan-template` for rings.
4. Write the command array.
5. Run:

```powershell
chemsema-cli new commands.json --out molecule.ccjs --results results.json --document-json molecule-doc.json --pretty
```

6. Inspect:

```powershell
chemsema-cli targets molecule.ccjs --out targets.json --pretty
chemsema-cli capture molecule.ccjs --target all --out molecule.png --scale 8 --pretty
```

## Edit Existing Document

1. Discover the existing atom, bond, or molecule selectors.
2. Use `detail` to get coordinates and ids.
3. Use planning queries from those coordinates or ids.
4. Run:

```powershell
chemsema-cli run input.ccjs commands.json --out edited.ccjs --results results.json --pretty
```

5. Compare selected fragments structurally, then visually.

## Command Hygiene

- Prefer engine-generated commands from plan queries.
- Keep one logical edit per command when debuggability matters.
- Request `--results` for every nontrivial run.
- Request `--document-json` when downstream tools need internal JSON.
