# Chemical Semantics Kernel

ChemSema treats the Rust chemistry crate as the authority for molecular graph
meaning. The viewer, desktop host, CLI, and format adapters consume the same
results and must not implement separate valence, aromaticity, hydrogen, or
canonicalization rules.

## Sanitization Pipeline

`chemsema-chemistry::sanitize` performs these steps before an imported SMILES
can mutate a document or an identifier can be generated:

1. validate atom references, self bonds, and duplicate bonds;
2. validate aromatic bonds and require every aromatic bond to belong to a ring;
3. find a deterministic Kekulé matching for supported aromatic atoms;
4. calculate bond-order valence and reject unsupported over-valent states;
5. derive implicit hydrogens from element, charge, aromatic state, explicit
   hydrogens, and the selected valence ladder;
6. validate directional bond markers and normalize complete alkene direction
   pairs to E/Z;
7. preserve the OpenSMILES ligand order for `@`/`@@`, require four ligands,
   and assign R/S when the implemented tetrahedral CIP comparison can
   distinguish all four ligands.

Coordinate bonds do not consume ordinary covalent valence. Transition-metal
oxidation-state and electron-count validation is deliberately separate; a
dative bond is never approximated as a single bond merely to make a downstream
algorithm accept it.

The initial supported valence model covers the common organic subset, charged
B/N/O states, Si, expanded-valence P/S, and halogen valence ladders. Other
elements remain structurally representable but are not falsely reported as
fully valence-validated.

## Canonical SMILES

ChemSema calculates input-order-independent atom ranks using iterative graph
refinement plus bounded symmetry-class individualization. Components and ring
closures are emitted from those ranks. Tetrahedral parity is rewritten against
the emitted neighbor order, including ring-closure ligands, and normalized E/Z
groups receive deterministic slash directions. The result reports
`canonical: true`.

Canonical SMILES is toolkit-specific: ChemSema guarantees stability for this
algorithm and protocol version, not byte-for-byte identity with RDKit or
another toolkit. A bounded search protects the editor from pathological highly
symmetric graphs. If the bound is exceeded, or stereo normalization is not yet
available, ChemSema returns an isomeric non-canonical SMILES with
`canonical: false` and a machine-readable `canonicalReason` instead of making a
false canonical claim. Advanced stereochemical classes outside the stated
boundary still return an explicit unsupported result.

## Stereo Boundary

Directional single bonds adjacent to a double bond are interpreted together.
For example, `F/C=C/F` normalizes to E and `F/C=C\F` to Z. Orphan directional
bonds are rejected. Tetrahedral `@`/`@@` centers retain both parity and the
OpenSMILES ligand order. Equivalent traversals are rewritten to one canonical
isomeric SMILES. The sanitizer reports R/S for ordinary tetrahedral centers
whose four ligands are distinguishable by the implemented recursive CIP
comparison. It does not claim the full 2013 CIP system: pseudoasymmetry,
sequence-rule recursion through stereogenic units, axial/allene, square-planar,
trigonal-bipyramidal, and octahedral descriptors remain unsupported.

Normalized E/Z groups are projected to deterministic opposite 2D geometries
before the official InChI library is called; native regression tests require E
and Z inputs to produce different InChI `/b` layers. Tetrahedral centers are
projected to one configuration-consistent V2000 wedge bond; native regression
tests compare L- and D-alanine against the official InChI `/t` and `/m` layers.
SMILES import draws the same single wedge, and structure analysis can recover
the center from the wedge and current 2D geometry without source-SMILES parity.
Dative bonds are rejected for the same integrity reason until a
metal-disconnection/reconnected-layer policy is defined.

## Analysis Metadata

The `chemistry` command retains `value` as the copyable identifier and also
reports semantic metadata where applicable:

- `canonical`, `canonicalReason`, and `isomeric`;
- per-atom `implicitHydrogens`;
- normalized `doubleBondStereo` and validated `tetrahedralCenters`, including
  `cip` when assignment is supported;
- `properties.formula`, `formalCharge`, atom counts, and component count.

Imported semantic metadata is provenance, not an immutable override. If an
atom, charge, or displayed aromatic bond order changes, analysis falls back to
the current editable molecular graph.
