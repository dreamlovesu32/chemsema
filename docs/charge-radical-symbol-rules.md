# ChemSema Charge And Radical Symbol Assignment Rules

This document defines the chemical semantics of eight charge/electron symbols. When these symbols are close to molecular endpoints or attached labels, they should be assigned to the corresponding atom and participate in valence, implicit hydrogen, red-box validity, and repeating-unit expansion.

## Symbol Set

The eight symbols in the bracket/symbol tool are grouped by chemical semantics:

| UI kind | Semantics | Effect on atom |
| --- | --- | --- |
| `circle-plus` | positive charge | `formalCharge += 1` |
| `plus` | positive charge | `formalCharge += 1` |
| `circle-minus` | negative charge | `formalCharge -= 1` |
| `minus` | negative charge | `formalCharge -= 1` |
| `radical-cation` | radical cation | `formalCharge += 1`, `radicalCount += 1` |
| `radical-anion` | radical anion | `formalCharge -= 1`, `radicalCount += 1` |
| `electron` | single electron | `radicalCount += 1` |
| `lone-pair` | lone-pair display symbol | keeps molecular graphics and assignment information |

Circled plus/minus and ordinary plus/minus are chemically equivalent. The circle is a display style and should not produce different node charge results.

The two dots of `lone-pair` are preserved as part of molecular graphics and participate in selection, dragging, and round-trip. A later format version may upgrade Lewis structure support to explicit lone-pair electron counts.

## Storage Model

These symbols may still exist as `SceneObject { type: "symbol" }` so they remain selectable, draggable, and independently visible. However, once assigned to an atom, they must also be written into the molecular semantic layer.

The symbol object stores assignment information:

```json
{
  "kind": "plus",
  "chemicalRole": "charge",
  "chargeDelta": 1,
  "radicalDelta": 0,
  "attachedFragmentObjectId": "obj_mol_1",
  "attachedAtomId": "n1",
  "attachmentSource": "endpoint",
  "attachmentDistance": 5.8
}
```

The node semantic state stores the summarized result:

```json
{
  "id": "n1",
  "element": "N",
  "charge": 1,
  "numHydrogens": 3,
  "meta": {
    "attachedElectronSymbols": [
      {
        "symbolObjectId": "obj_symbol_12",
        "kind": "circle-plus",
        "chargeDelta": 1,
        "radicalDelta": 0
      }
    ]
  }
}
```

The node's `charge`, future `radicalCount` or equivalent field is authoritative for chemical calculation. Assignment information on the symbol object is used for editing, selection, dragging, and round-trip. Expansion, export, red-box validity, and implicit-hydrogen refresh must all consume this assignment semantics.

## Assignment Rules

Symbol assignment occurs only within range of an atom. Candidate targets include:

- Bare endpoints, meaning atom endpoints without attached labels.
- Heavy-atom anchors of attached labels, such as the heavy atom in `N`, `O`, `Cl`, or `NH2`.
- Attachment atoms expanded from legal abbreviations or formula-like labels, such as the `C` in `CF3` or the entry `C` in `CO2Et`.

Automatically generated hydrogen characters assign back to the corresponding heavy atom. If the user drags a symbol next to the `H` character in `NH2`, the actual target remains the heavy atom `N` in that label.

Candidate selection:

1. Find the endpoint/label anchor nearest to the symbol center.
2. The distance must be less than the charge assignment radius. This radius should be on the same scale as the visual spacing for symbol click/drag and must be fixed by tests.
3. If multiple candidates are in range, prefer the currently hovered/focused atom; otherwise choose the nearest one.
4. While dragging a symbol in selection mode, the current candidate assignment and preview red box may update live during mouse move; mouse-up commits the document change.

Existing symbols can be dragged only in selection mode. The Symbol tool creates new symbols; the Select tool moves existing symbols and reassigns them.

## Relationship To Label Validity

The label red box comes from `meta.labelRecognition.status == "invalid"` or a mismatch between a simple element label and node state. After charge-symbol assignment, validity must be recalculated from the combined "node + assigned symbols" state.

### Invalid Label Missing One Connection Point

If a formula-like label is invalid because it is missing one connection point, dragging a positive/negative symbol or circled positive/negative symbol next to the label should reinterpret that label under the atom charge. If the charged state satisfies valence, the red box should disappear.

Examples:

```text
invalid neutral label + positive charge symbol -> recognized after positive valence is allowed
invalid neutral label + negative charge symbol -> recognized after negative valence is allowed
```

The implementation must not simply hide the red box. `labelRecognition` must re-enter the recognized state, and charge/radical information must be written into the expansion.

### Positive Charges Beside N/O And Similar Elements

For elements with implicit-hydrogen support, a positive charge usually increases allowed connection valence, so it may increase displayed hydrogens and make an otherwise hypervalent node valid.

Typical examples:

```text
R-O      + plus -> R-OH2+
R-N      + plus -> R-NH3+
R2-O     + plus -> R2-OH+
R3-N     + plus -> R3-NH+
R4-N     + plus -> R4-N+
```

`R4-N` should be red in neutral rules; after a positive charge is dragged in, it becomes a tetravalent ammonium-like nitrogen and the red box disappears.

### Negative Charges Reduce Hydrogen

A negative charge usually means deprotonation and should reduce the required hydrogen count. Drawing or dragging a negative charge beside terminal `NH2` should produce `NH-`:

```text
R-NH2 + minus -> R-NH-
```

If there is no hydrogen to reduce, the negative charge should not force validity. For example, triple-connected `N` already has no implicit hydrogen to remove:

```text
R3-N + minus -> invalid
```

Radical anions are different because they carry both `-1` and one unpaired electron. They may be legal in some three-coordinate nitrogen scenarios:

```text
R3-N + radical-anion -> allowed when radical valence model permits it
```

Radical anions should be handled as an independent validity branch, not by sharing the ordinary negative-charge rule that requires removal of one H.

## Implicit Hydrogen Calculation

Authoritative calculation model:

```text
target_valence = valence_model(element, formal_charge, radical_count, connection_count)
numHydrogens = target_valence - connection_count
```

`target_valence` is selected by element, formal charge, and radical state, then existing connection count is subtracted from the target valence. Ordinary negative charge requires a removable hydrogen; radical anions use a separate radical branch.

```text
legacy_simple_model = typical_valence - radical_count - connection_count - abs(charge)
```

`legacy_simple_model` may only be used as a migration check. After charge/radical assignment, validity is defined by the `target_valence` model above.

Rules cover elements with implicit-hydrogen support:

| Element | Neutral baseline | Positive-charge rule | Negative-charge rule | Radical notes |
| --- | --- | --- | --- | --- |
| `C` | Skeletal carbon fills to valence 4 implicitly, but H is not displayed | Ordinary positive charge means one fewer implicit H and a carbocation; invalid if already four explicit bonds | Ordinary negative charge means one fewer implicit H and a carbanion; invalid if already four explicit bonds | Single-electron radical also means one fewer implicit H; invalid with four explicit bonds. `radical-cation`/`radical-anion` are special charged radical states and may be legal |
| `B` | valence 3 | rare; default does not auto-add H | valence-4 borate/boron anion can be supported | supplement by explicit templates |
| `N` | valence 3/5 | valence-4 ammonium-like; positive charge usually allows one more H | one fewer H than neutral; invalid when no H can be removed | radical-anion may allow three-coordinate N |
| `P` | valence 3/5 | valence-4/6 phosphonium-like state can be an extension rule | one fewer H than neutral; conservative | may only affect expansion |
| `O` | valence 2 | valence-3 oxonium; positive charge usually allows one more H | one fewer H than neutral; invalid when no H can be removed | radical-anion may support oxygen radical anions |
| `S` | valence 2/4/6 | valence-3/5 sulfonium-like state can be supported | one fewer H than neutral; conservative | hypervalent sulfur requires bond-order context |
| `F/Cl/Br/I` | valence 1 and hypervalent ladder | hypervalent positive halogen is not inferred automatically by default | halide anions usually do not attach to organic endpoints; conservative invalid | radical halogens may keep symbols but do not auto-add H |
| `Si` | valence 4 | silicon cation handled conservatively | silicon anion handled conservatively | extend by explicit needs |

"Conservative" means the symbol still assigns and enters expansion, but if no explicit valence rule exists, the engine should not invent a chemically unreliable state only to clear a red box.

## Expansion Requirements

Repeating-unit expansion and functional-group expansion must include charge/radical semantics introduced by symbols.

### Repeating Unit Expansion

When an atom inside brackets carries assigned symbols, every repeated atom in the expansion must copy that semantics:

```json
{
  "id": "n1_r1",
  "element": "N",
  "atomicNumber": 7,
  "charge": 1,
  "radicalCount": 0,
  "numHydrogens": 3,
  "electronSymbols": [
    {
      "sourceSymbolObjectId": "obj_symbol_12",
      "kind": "circle-plus",
      "chargeDelta": 1,
      "radicalDelta": 0
    }
  ],
  "sourceAtomId": "n1",
  "repeatIndex": 1
}
```

If the symbol itself is inside the bracket but is not assigned to any internal atom, the repeating unit should be marked incomplete or should not produce expansion, avoiding semantic loss.

### Functional Group Expansion

If a charge symbol next to a label is assigned to the attachment atom of a functional group, it must also be written to the atom in `functionalGroupExpansion.v1`:

```json
{
  "id": "n1",
  "element": "N",
  "formalCharge": 1,
  "radicalCount": 0,
  "numHydrogens": 3,
  "sourceSymbolObjectIds": ["obj_symbol_12"]
}
```

The recognition result's `canonicalLabel` may still be the user's original input, such as `N` or `NH2`; however, `formula` and expansion must reflect the real charged semantics, such as `-NH3+`.

## Editing Behavior

### Creation

When the Symbol tool creates one of the eight symbols:

- If the creation position is within an atom/label assignment radius, bind it to that atom immediately.
- If it is out of range, still create an unassigned molecular symbol; it participates in selection and display but does not affect valence.
- After creation, refresh the atom's `charge`, `radicalCount`, `numHydrogens`, label text, and red-box state.

### Movement

When the Select tool drags an existing symbol:

- Drag start records the original assignment.
- Drag move computes the current candidate atom and may display hover/preview.
- Mouse-up commits the new assignment; if it leaves all candidate ranges, the symbol becomes unassigned and its charge/radical effect is removed from the original atom.
- The entire drag is one undoable command.

### Deletion

When deleting an assigned symbol:

- Remove it from the original atom's `attachedElectronSymbols`.
- Recompute `charge`, `radicalCount`, `numHydrogens`, and `labelRecognition`.
- If deleting a positive charge makes an originally tetravalent N invalid again, restore the red box.

## Chemical Rule Summary

The rules maintain these principles:

1. Plus/minus signs are atom properties.
2. Circled and non-circled plus/minus signs are chemically equivalent.
3. Ordinary positive charges may allow N/O/S/P and similar elements to enter higher valence states, and may increase implicit H.
4. Ordinary negative charges usually come from deprotonation and should reduce one implicit H; if no H can be removed, validity should not be forced.
5. Radical anions/cations include both charge and unpaired electrons, so validity must be modeled separately.
6. Two-dot lone-pair symbols only preserve display and molecular assignment, and do not participate in valence.
7. All of these semantics must enter expansion; otherwise copy, bracket expansion, export, and later chemical analysis lose information.
8. The viewer should not infer these rules independently; assignment, validity, and implicit-hydrogen refresh must be unified in the Rust engine.

Special rule for bare carbon endpoints:

- A bare endpoint is still a carbon atom. It fills implicit hydrogens to valence 4, but hydrogens are not displayed.
- Ordinary positive charge, ordinary negative charge, and single-electron radical all require that the carbon still has at least one implicit hydrogen to reduce.
- If carbon already has four explicit bonds, dragging in ordinary plus/minus or a single-electron radical should keep the symbol assignment but mark the atom invalid, using a red circle the same size as a focus point.
- `radical-cation` and `radical-anion` are special charged radical states. On four-connected carbon, they are handled by an independent radical branch rather than the ordinary radical rule that requires a removable H.

## Regression Cases

These rules should have automated test coverage:

```text
circle-plus and plus assigned to the same N produce the same charge/numHydrogens.
circle-minus and minus assigned to the same N produce the same charge/numHydrogens.
terminal N + plus -> NH3+, red box disappears.
terminal O + plus -> OH2+, red box disappears.
four-connected N + plus -> N+, red box disappears.
terminal NH2 + minus -> NH-.
three-connected N + minus -> invalid.
three-connected N + radical-anion -> recognized when radical branch is enabled.
unassigned symbols do not change any atom charge.
bare carbon endpoint + plus/minus/electron reduces one implicit H.
four-connected bare carbon + plus/minus/electron -> invalid red circle.
four-connected bare carbon + radical-cation/radical-anion -> allowed.
dragging a symbol from atom A to atom B refreshes charge/numHydrogens on both A and B correctly.
deleting an assigned symbol restores the original atom red-box/implicit-hydrogen state.
repeating unit expansion copies charge/radical/electronSymbols on internal atoms.
functional group expansion writes formalCharge/radicalCount onto the attachment atom.
```
