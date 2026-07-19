use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Molecule {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
}

impl Molecule {
    pub fn new() -> Self {
        Self {
            atoms: Vec::new(),
            bonds: Vec::new(),
        }
    }

    pub fn components(&self) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        let mut seen = vec![false; self.atoms.len()];
        let mut adjacency = vec![Vec::new(); self.atoms.len()];
        for bond in &self.bonds {
            adjacency[bond.begin].push(bond.end);
            adjacency[bond.end].push(bond.begin);
        }
        for root in 0..self.atoms.len() {
            if seen[root] {
                continue;
            }
            let mut stack = vec![root];
            let mut component = Vec::new();
            seen[root] = true;
            while let Some(atom) = stack.pop() {
                component.push(atom);
                for &neighbor in &adjacency[atom] {
                    if !seen[neighbor] {
                        seen[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
            component.sort_unstable();
            result.push(component);
        }
        result
    }
}

impl Default for Molecule {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Atom {
    pub atomic_number: u8,
    pub symbol: String,
    pub isotope: Option<u16>,
    pub charge: i32,
    pub explicit_hydrogens: u8,
    pub aromatic: bool,
    pub chirality: Option<Chirality>,
    /// Ligands in the order in which they occur around this atom in the
    /// source SMILES. This makes `@`/`@@` a graph property instead of tying
    /// it to one particular traversal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chiral_order: Vec<ChiralLigand>,
    pub atom_map: Option<u32>,
    pub no_implicit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Chirality {
    Clockwise,
    Anticlockwise,
}

impl Chirality {
    pub fn inverted(self) -> Self {
        match self {
            Self::Clockwise => Self::Anticlockwise,
            Self::Anticlockwise => Self::Clockwise,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum ChiralLigand {
    Atom(usize),
    Hydrogen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bond {
    pub begin: usize,
    pub end: usize,
    pub kind: BondKind,
    pub direction: Option<BondDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BondKind {
    Single,
    Double,
    Triple,
    Quadruple,
    Aromatic,
    Dative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BondDirection {
    Up,
    Down,
}
