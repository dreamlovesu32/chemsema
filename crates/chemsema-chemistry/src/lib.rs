//! ChemSema's dependency-light chemical semantics core.
//!
//! The SMILES grammar and lexer in this crate are a clean Rust reimplementation
//! informed by RDKit's Code/GraphMol/SmilesParse/smiles.yy and smiles.ll at
//! commit 0062b670640352ab63d6256be608615e87e1af53. RDKit is BSD-3-Clause;
//! see LICENSES/BSD-3-Clause-RDKit.txt and THIRD_PARTY_NOTICES.md.
//! This implementation has been substantially redesigned for ChemSema and does
//! not link to RDKit or require Python.

mod canonical;
mod cip;
mod layout;
mod model;
mod molfile;
mod properties;
mod sanitize;
mod smiles;

pub use layout::{layout_2d, LayoutOptions, Point2};
pub use model::{Atom, Bond, BondDirection, BondKind, ChiralLigand, Chirality, Molecule};
pub use molfile::write_molfile_v2000;
pub use properties::{molecular_properties, MolecularProperties};
pub use sanitize::{
    sanitize, ChemistryError, ChemistryErrorKind, CipDescriptor, DoubleBondConfiguration,
    DoubleBondStereo, Sanitization, TetrahedralCenter,
};
pub use smiles::{
    parse_smiles, write_canonical_smiles, write_smiles, SmilesError, SmilesErrorKind,
};
