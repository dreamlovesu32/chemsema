# ChemCore Valence-Driven Label Recognition Rules

This document defines ChemCore valence-driven label recognition rules. The valence parser interprets formula-like labels such as `CN`, `CO2Cl`, and `CH2COOCH2SO2NHCl`, where meaning can be derived from elemental valence and linear writing order. Named substituents such as `Boc`, `Fmoc`, and `Ts` are interpreted by dedicated templates.

## Behavioral Goals

The valence parser splits formula-like labels such as `CN`, `CF3`, `CONH2`, and `CO2Et` into general rules:

- Read elements and counts, such as `CH3` as `C + H + H + H`, and `CO2Cl` as `C + O + O + Cl`.
- Consume valence on the attachment atom according to the node's external connection count.
- Read later atoms from left to right. The current connectable atom tries to satisfy its valence as much as possible without exceeding allowed valence.
- Automatically decide single, double, or triple bonds from remaining valence, such as `CN` -> `C#N` and `CO2Cl` -> `C(=O)OCl`.
- After successful parsing, generate `functionalGroupExpansion.v1` and mark `meta.labelRecognition` as `status: recognized`, `source: valence-parser`.

## Connection Count Context

Recognition cannot depend on the string alone; it must know how many external bonds the label node already has.

### Terminal Labels

A terminal functional group requires exactly one external bond. During parsing, the external bond connects to the first connectable atom and consumes one valence unit on that atom.

Example:

```text
-CH3
```

Parsing process:

```text
C allows valence 4; the left external bond consumes 1, leaving 3.
Each H consumes 1.
C is exactly satisfied and is recognized as methyl.
```

### Bridging Labels

A two-bond bridging label requires exactly two external bonds. Current two-bond bridging is handled by the abbreviation recognition entry point. If left and right attachments cannot both satisfy valence, the label should be marked invalid.

This rule preserves existing bridging abbreviation behavior: `NH`, `CO`, `CO2/COO`, and `SO2` remain legal on two-bond nodes.

### Other Connection Counts

Zero external bonds enter chemical text validation: text understood by the valence tokenizer is marked as `groupKind: "chemical-text"`, but no `functionalGroupExpansion.v1` is generated. Three or more external bonds do not enter terminal functional-group parsing by default; unless an explicit template supports them, they should be marked as ordinary unknown labels or invalid.

## Element Valence Table

In the valence parser, "valence" first means the number of connectable bond units, not positive/negative character. Alkali metals have valence 1 and alkaline-earth metals have valence 2 as connection capacities. The parser must not invent hidden formal charges to make second-period over-valent labels pass.

### Regular Valence

| Element | Allowed valence | Notes |
| --- | --- | --- |
| `H` | 1 | Terminal atom only. |
| `Li/Na/K/Rb/Cs/Fr` | 1 | Alkali metals are uniformly treated as valence 1. |
| `Be/Mg/Ca/Sr/Ba/Ra` | 2 | Alkaline-earth metals are uniformly treated as valence 2. |
| `B` | 3 | Regular boron is valence 3. Valence-4 boron requires explicit negative-charge evidence; without it the label is invalid. |
| `C` | 4 | Core skeleton atom for formula-like labels. |
| `N` | 3 | Ordinary nitrogen is valence 3. Second-period nitrogen does not support expanded octets; valence-4 nitrogen requires explicit positive-charge evidence, and ordinary four-connected nitrogen is invalid. |
| `O` | 2 | Can be carbonyl oxygen, ether oxygen, hydroxy oxygen, or continuing oxygen. Valence-3 oxygen requires explicit positive-charge evidence; second-period oxygen does not support expanded octets. |
| `Si` | 4 | Silicon is treated as valence 4. |
| `P` | 3, 5 | Phosphorus is treated as valence 3/5. |
| `As` | 3, 5 | Arsenic is treated as valence 3/5. |
| `S` | 2, 4, 6 | `SO2` is explicitly treated as S(VI); `SOO` is treated as S(IV). |
| `F/Cl/Br/I` | 1, 3, 5, 7 | Ordinary organic substituents prefer valence 1; higher valence is used only when context satisfies and clearly needs it. |

The default principle for selecting valence is "minimum valence that satisfies the structure". For elements such as `S` that have writing conventions, local patterns are examined first. For example, `SO2` directly chooses S(VI).

### Formal Charge Evidence

Second-period `B`, `N`, and `O` charged valence states may participate in topological parsing only when explicit charge evidence is present in the source label or node metadata. The current valence tokenizer does not parse visible `+` / `-` charge tokens, so one-bond terminal labels such as `BH3`, `NH3`, `OH2`, and `OH3` remain invalid instead of silently recording `formalCharge` in `expansion.atoms[]`. Second-period elements do not use expanded-octet fallback valences such as 5 or 6.

## Tokenization Rules

Labels are first converted into an atom occurrence stream:

- Element symbols are recognized with standard case, for example `Cl` is one atom.
- A number immediately after an element means that element repeats.
- `H3` expands into three hydrogens; `O2` expands into two oxygens.
- `CO2Cl` expands into `C, O, O, Cl`.
- Parenthesized groups parse as sub-token streams; a number after the group repeats it. Empty groups, zero repetitions, and more than 32 repetitions are invalid.
- `CH2COOCH2SO2NHCl` expands into:

```text
C, H, H, C, O, O, C, H, H, S, O, O, N, H, Cl
```

Dots, explicit charges, isotopes, aromatic lowercase, ring numbers, and SMILES syntax are handled by dedicated templates or invalid fallback.

## Core Parsing Principles

### Fill The Current Atom As Much As Possible

When reading left to right, the currently open skeleton atom preferentially absorbs right-side atoms until its valence is satisfied or the next connection would exceed valence.

Example `-CO2Cl`:

```text
C consumes 1 by the left external bond, leaving 3.
The first O can be valence 2, so C and O form C=O as much as possible; C has 1 left.
The second O can only single-bond to C, satisfying C; O has 1 left.
Cl chooses valence 1 and single-bonds to the second O.
```

Result:

```text
-C(=O)OCl
```

### First Multiple-Bond-Capable Heteroatom Has Priority

When carbon encounters `O`, `S`, `N`, or another multiple-bond-capable heteroatom to its right, prefer forming the highest feasible bond order from the current carbon to that first heteroatom. Remaining valence goes to later atoms.

This guarantees:

```text
-CN    -> -C#N
-COCl  -> -C(=O)Cl
-CSO-  -> -C(=S)O-
-COS-  -> -C(=O)S-
```

The main path for `-CSO-` is `-C(=S)O-`: `C` first forms a double bond to the first `S`. `-COS-` similarly first forms a carbon-oxygen double bond.

### Later Atoms Attach To The Nearest Satisfiable Attachment

When the current skeleton atom is valence-satisfied, later atoms should attach to the nearest atom that still has remaining valence and can serve as a right-side attachment in the written order.

In `-CO2Cl`, after the second `O` single-bonds to `C`, it has one remaining valence, so the later `Cl` attaches to that `O`.

### Special Writing Conventions

The oxidation state of `S` is determined first by common writing conventions:

```text
SO2  -> S(VI), two S=O
SOO  -> S(IV), one S=O and one S-O
```

Therefore:

```text
-SO2NHCl
```

parses as:

```text
The left single bond consumes 1 on S.
The SO2 convention chooses valence-6 sulfur; two O atoms each form S=O, consuming 4.
S has 1 left and attaches to N.
N chooses valence 3 and is satisfied after attaching H and Cl.
```

Result:

```text
-S(=O)2NHCl
```

## Example Derivations

### `-CH3`

```text
C: external bond 1 + H + H + H = 4
```

Result: `-CH3`.

### `-CO2Cl`

```text
C: external bond 1 + O(double) + O(single) = 4
O(single): C + Cl = 2
Cl: O = 1
```

Result: `-C(=O)OCl`.

### `-CH2COOCH2SO2NHCl`

Token stream:

```text
C H H C O O C H H S O O N H Cl
```

Derivation:

```text
C1: external bond 1 + H + H + C2 = 4
C2: C1 + O1(double) + O2(single) = 4
O2: C2 + C3 = 2
C3: O2 + H + H + S = 4
S: C3 + O3(double) + O4(double) + N = 6
N: S + H + Cl = 3
Cl: N = 1
```

Result:

```text
-CH2-C(=O)-O-CH2-S(=O)2-NHCl
```

This example is a core regression case for the valence parser. It covers carbon valence filling, carbonyl, ester oxygen continuation, methylene, sulfonyl, nitrogen bridge, and halogen termination at the same time.

## Relationship To The Abbreviation Table

The valence parser is the main path for terminal formula-like labels. Abbreviation/protecting-group definitions act as monovalent terminal tokens for the valence parser. Named groups such as `Me`, `Et`, `Boc`, `Fmoc`, `Ts`, and `TBDMS` are equivalent to terminal atoms that consume one connection site when valence is satisfied. Their internal expansion still uses the manually confirmed templates.

Recognition priority:

1. Simple element labels and implicit-hydrogen rules, such as `N`, `O`, and `Cl`.
2. Valence-driven formula-like parser, such as `CN`, `CF3`, `CO2Cl`, and `CH2COOCH2SO2NHCl`.
3. Named functional-group templates entered alone, such as `Boc`, `Fmoc`, `Ts`, and `TBDMS`. They are recognized as whole templates.
4. Invalid fallback.

This means:

```text
Boc       -> named template, standalone monovalent substituent
CO2Boc    -> C + O + O + Boc, where Boc consumes one connection site on the second O
CH2Boc    -> C + H + H + Boc, where Boc consumes one connection site on C
```

Labels such as `CN`, `CF3`, `COCl`, `CONH2`, and `CO2Et`, whose meaning can be derived from valence rules, go through the valence parser.

## Metadata

When the valence parser succeeds, `meta.labelRecognition` still uses the existing shape and additionally keeps source information for debugging and migration:

```json
{
  "kind": "functional-group",
  "status": "recognized",
  "source": "valence-parser",
  "label": "CO2Cl",
  "canonicalLabel": "CO2Cl",
  "groupKind": "valence-fragment",
  "anchorAtom": "C",
  "formula": "-C(=O)OCl",
  "components": [
    { "label": "C", "kind": "atom" },
    { "label": "O", "kind": "atom", "bondOrderToParent": 2 },
    { "label": "O", "kind": "atom", "bondOrderToParent": 1 },
    { "label": "Cl", "kind": "atom", "bondOrderToParent": 1 }
  ],
  "expansion": {
    "schema": "chemcore.functionalGroupExpansion.v1",
    "connectionKind": "terminal",
    "complete": true,
    "attachments": [
      { "role": "external", "atomId": "c1" }
    ]
  }
}
```

`components` are mainly used for tests, debugging, and export validation, especially for checking local decisions that are easy to make ambiguous, such as `CO2`, `SO2`, `CSO`, and `COS`.

## Regression Cases

The following cases should be covered by Rust unit tests:

```text
CH3                  -> -CH3
CN                   -> -C#N
CF3                  -> -CF3
COCl                 -> -C(=O)Cl
COBr                 -> -C(=O)Br
CONH2                -> -C(=O)NH2
CO2Cl                -> -C(=O)OCl
COOH                 -> -C(=O)OH
CO2Et                -> -C(=O)OCH2CH3
CH2COOCH2SO2NHCl     -> -CH2-C(=O)-O-CH2-S(=O)2-NHCl
CSO                  -> -C(=S)O-
COS                  -> -C(=O)S-
SO2NHCl              -> -S(=O)2NHCl
SOONHCl              -> -S(=O)ONHCl
Na                   -> -Na
MgCl                 -> -MgCl
SiH3                 -> -SiH3
PH2                  -> -PH2
AsH2                 -> -AsH2
CH2Boc               -> -CH2Boc, Boc as monovalent terminal token
```

Also preserve these template-priority cases:

```text
Boc
Fmoc
Ts
TBDMS
TBDPS
Ph
```

They should continue to use named templates, not the ordinary valence parser.

The following cases should remain invalid, avoiding overly broad exceptional valence:

```text
BCl3
BH3
NMe4
NH3
OCl3
OCl4
OH2
OH3
```
