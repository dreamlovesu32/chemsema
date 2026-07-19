use crate::{cip::tetrahedral_descriptor, Atom, BondKind, ChiralLigand, Chirality, Molecule};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChemistryErrorKind {
    InvalidBond,
    DuplicateBond,
    Valence,
    Aromaticity,
    Stereochemistry,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemistryError {
    pub kind: ChemistryErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atom_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bond_index: Option<usize>,
}

impl ChemistryError {
    fn atom(kind: ChemistryErrorKind, atom_index: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            atom_index: Some(atom_index),
            bond_index: None,
        }
    }

    fn bond(kind: ChemistryErrorKind, bond_index: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            atom_index: None,
            bond_index: Some(bond_index),
        }
    }
}

impl fmt::Display for ChemistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(atom) = self.atom_index {
            write!(formatter, " (atom {})", atom + 1)?;
        }
        if let Some(bond) = self.bond_index {
            write!(formatter, " (bond {})", bond + 1)?;
        }
        Ok(())
    }
}

impl std::error::Error for ChemistryError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sanitization {
    pub valence_twice: Vec<u16>,
    pub implicit_hydrogens: Vec<u8>,
    pub aromatic_double_bonds: Vec<usize>,
    pub double_bond_stereo: Vec<DoubleBondStereo>,
    pub tetrahedral_centers: Vec<TetrahedralCenter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DoubleBondConfiguration {
    E,
    Z,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoubleBondStereo {
    pub bond_index: usize,
    pub begin_reference_bond: usize,
    pub end_reference_bond: usize,
    pub configuration: DoubleBondConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TetrahedralCenter {
    pub atom_index: usize,
    pub smiles_parity: Chirality,
    pub neighbor_atoms: Vec<usize>,
    pub explicit_hydrogens: u8,
    pub ligand_order: Vec<ChiralLigand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cip: Option<CipDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipDescriptor {
    R,
    S,
}

pub fn sanitize(molecule: &Molecule) -> Result<Sanitization, ChemistryError> {
    validate_graph(molecule)?;
    let aromatic_double_bonds = kekulize_aromatic(molecule)?;
    let double_bond_stereo = analyze_double_bond_stereo(molecule)?;
    let mut valence_twice = vec![0u16; molecule.atoms.len()];
    for (bond_index, bond) in molecule.bonds.iter().enumerate() {
        let contribution = match bond.kind {
            BondKind::Single => 2,
            BondKind::Double => 4,
            BondKind::Triple => 6,
            BondKind::Quadruple => 8,
            BondKind::Aromatic if aromatic_double_bonds.contains(&bond_index) => 4,
            BondKind::Aromatic => 2,
            // Coordinate bonds do not consume the donor's ordinary covalent
            // valence. Metal oxidation-state and electron-count validation is
            // a separate layer and must not be approximated as a single bond.
            BondKind::Dative => 0,
        };
        valence_twice[bond.begin] += contribution;
        valence_twice[bond.end] += contribution;
    }

    let mut implicit_hydrogens = Vec::with_capacity(molecule.atoms.len());
    for (index, atom) in molecule.atoms.iter().enumerate() {
        let actual_twice = valence_twice[index] + u16::from(atom.explicit_hydrogens) * 2;
        let allowed = allowed_valences(atom);
        if let Some(maximum) = allowed.iter().copied().max() {
            if actual_twice > u16::from(maximum) * 2 {
                return Err(ChemistryError::atom(
                    ChemistryErrorKind::Valence,
                    index,
                    format!(
                        "{} has valence {}, above the supported maximum {}",
                        atom.symbol,
                        format_valence(actual_twice),
                        maximum
                    ),
                ));
            }
        }
        let implicit = if atom.no_implicit || atom.atomic_number == 0 {
            0
        } else {
            allowed
                .iter()
                .copied()
                .find(|target| u16::from(*target) * 2 >= actual_twice)
                .and_then(|target| {
                    let missing = u16::from(target) * 2 - actual_twice;
                    missing.is_multiple_of(2).then_some((missing / 2) as u8)
                })
                .unwrap_or(0)
        };
        implicit_hydrogens.push(implicit);
    }

    let tetrahedral_centers = analyze_tetrahedral_centers(molecule)?;
    Ok(Sanitization {
        valence_twice,
        implicit_hydrogens,
        aromatic_double_bonds: aromatic_double_bonds.into_iter().collect(),
        double_bond_stereo,
        tetrahedral_centers,
    })
}

fn analyze_tetrahedral_centers(
    molecule: &Molecule,
) -> Result<Vec<TetrahedralCenter>, ChemistryError> {
    let mut result = Vec::new();
    for (atom_index, atom) in molecule.atoms.iter().enumerate() {
        let Some(smiles_parity) = atom.chirality else {
            continue;
        };
        if atom.aromatic {
            return Err(ChemistryError::atom(
                ChemistryErrorKind::Stereochemistry,
                atom_index,
                "tetrahedral @/@@ parity is not supported on aromatic atoms",
            ));
        }
        let mut neighbors = molecule
            .bonds
            .iter()
            .filter_map(|bond| {
                if bond.kind == BondKind::Dative {
                    return None;
                }
                if bond.begin == atom_index {
                    Some(bond.end)
                } else if bond.end == atom_index {
                    Some(bond.begin)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        let coordination = neighbors.len() + usize::from(atom.explicit_hydrogens);
        if coordination != 4 || atom.explicit_hydrogens > 1 {
            return Err(ChemistryError::atom(
                ChemistryErrorKind::Stereochemistry,
                atom_index,
                format!("tetrahedral @/@@ parity requires four ligands; found {coordination}"),
            ));
        }
        if atom.chiral_order.len() != 4
            || atom.chiral_order.iter().any(|ligand| match ligand {
                ChiralLigand::Atom(neighbor) => !neighbors.contains(neighbor),
                ChiralLigand::Hydrogen => atom.explicit_hydrogens != 1,
            })
        {
            return Err(ChemistryError::atom(
                ChemistryErrorKind::Stereochemistry,
                atom_index,
                "tetrahedral center is missing its SMILES ligand order",
            ));
        }
        result.push(TetrahedralCenter {
            atom_index,
            smiles_parity,
            neighbor_atoms: neighbors,
            explicit_hydrogens: atom.explicit_hydrogens,
            ligand_order: atom.chiral_order.clone(),
            cip: tetrahedral_descriptor(molecule, atom_index, &atom.chiral_order, smiles_parity),
        });
    }
    Ok(result)
}

fn analyze_double_bond_stereo(
    molecule: &Molecule,
) -> Result<Vec<DoubleBondStereo>, ChemistryError> {
    for (bond_index, bond) in molecule.bonds.iter().enumerate() {
        if bond.direction.is_none() {
            continue;
        }
        if bond.kind != BondKind::Single {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::Stereochemistry,
                bond_index,
                "directional '/' and '\\' markers are only valid on single bonds",
            ));
        }
        let adjacent_double = molecule.bonds.iter().any(|candidate| {
            candidate.kind == BondKind::Double
                && (candidate.begin == bond.begin
                    || candidate.end == bond.begin
                    || candidate.begin == bond.end
                    || candidate.end == bond.end)
        });
        if !adjacent_double {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::Stereochemistry,
                bond_index,
                "directional bond is not adjacent to a double bond",
            ));
        }
    }

    let mut result = Vec::new();
    for (double_index, double) in molecule.bonds.iter().enumerate() {
        if double.kind != BondKind::Double {
            continue;
        }
        let begin = directional_neighbors(molecule, double.begin, double_index);
        let end = directional_neighbors(molecule, double.end, double_index);
        validate_same_end_directions(molecule, double.begin, &begin)?;
        validate_same_end_directions(molecule, double.end, &end)?;
        let (Some((begin_bond, begin_direction)), Some((end_bond, end_direction))) =
            (begin.first().copied(), end.first().copied())
        else {
            continue;
        };
        result.push(DoubleBondStereo {
            bond_index: double_index,
            begin_reference_bond: begin_bond,
            end_reference_bond: end_bond,
            configuration: if begin_direction == end_direction {
                DoubleBondConfiguration::Z
            } else {
                DoubleBondConfiguration::E
            },
        });
    }
    Ok(result)
}

fn directional_neighbors(
    molecule: &Molecule,
    atom: usize,
    excluded_bond: usize,
) -> Vec<(usize, crate::BondDirection)> {
    let mut result = molecule
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(bond_index, bond)| {
            if bond_index == excluded_bond || bond.kind != BondKind::Single {
                return None;
            }
            if bond.begin != atom && bond.end != atom {
                return None;
            }
            oriented_direction(bond, atom).map(|direction| (bond_index, direction))
        })
        .collect::<Vec<_>>();
    result.sort_unstable_by_key(|(bond, _)| *bond);
    result
}

fn validate_same_end_directions(
    molecule: &Molecule,
    atom: usize,
    bonds: &[(usize, crate::BondDirection)],
) -> Result<(), ChemistryError> {
    let Some((_, reference)) = bonds.first().copied() else {
        return Ok(());
    };
    if let Some((bond_index, _)) = bonds
        .iter()
        .skip(1)
        .find(|(_, direction)| *direction == reference)
    {
        return Err(ChemistryError::bond(
            ChemistryErrorKind::Stereochemistry,
            *bond_index,
            format!(
                "directional substituent bonds at atom {} must point to opposite sides",
                atom + 1
            ),
        ));
    }
    let _ = molecule;
    Ok(())
}

fn oriented_direction(bond: &crate::Bond, from: usize) -> Option<crate::BondDirection> {
    match (bond.direction, bond.begin == from) {
        (Some(direction), true) => Some(direction),
        (Some(crate::BondDirection::Up), false) => Some(crate::BondDirection::Down),
        (Some(crate::BondDirection::Down), false) => Some(crate::BondDirection::Up),
        (None, _) => None,
    }
}

fn validate_graph(molecule: &Molecule) -> Result<(), ChemistryError> {
    let mut pairs = BTreeMap::new();
    for (index, bond) in molecule.bonds.iter().enumerate() {
        if bond.begin >= molecule.atoms.len() || bond.end >= molecule.atoms.len() {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::InvalidBond,
                index,
                "bond references an atom outside the molecule",
            ));
        }
        if bond.begin == bond.end {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::InvalidBond,
                index,
                "self bonds are not chemically valid",
            ));
        }
        let pair = if bond.begin < bond.end {
            (bond.begin, bond.end)
        } else {
            (bond.end, bond.begin)
        };
        if let Some(previous) = pairs.insert(pair, index) {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::DuplicateBond,
                index,
                format!("bond duplicates bond {}", previous + 1),
            ));
        }
    }
    Ok(())
}

fn kekulize_aromatic(molecule: &Molecule) -> Result<BTreeSet<usize>, ChemistryError> {
    let aromatic_edges = molecule
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| (bond.kind == BondKind::Aromatic).then_some(index))
        .collect::<Vec<_>>();
    for &bond_index in &aromatic_edges {
        let bond = &molecule.bonds[bond_index];
        if !molecule.atoms[bond.begin].aromatic || !molecule.atoms[bond.end].aromatic {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::Aromaticity,
                bond_index,
                "an aromatic bond must connect two aromatic atoms",
            ));
        }
        if !aromatic_path_exists_without_edge(molecule, bond.begin, bond.end, bond_index) {
            return Err(ChemistryError::bond(
                ChemistryErrorKind::Aromaticity,
                bond_index,
                "aromatic bonds must belong to a ring",
            ));
        }
    }

    let mut incident = vec![Vec::new(); molecule.atoms.len()];
    for &bond_index in &aromatic_edges {
        let bond = &molecule.bonds[bond_index];
        incident[bond.begin].push(bond_index);
        incident[bond.end].push(bond_index);
    }
    for (index, atom) in molecule.atoms.iter().enumerate() {
        if atom.aromatic && incident[index].is_empty() {
            return Err(ChemistryError::atom(
                ChemistryErrorKind::Aromaticity,
                index,
                "aromatic atom is not part of an aromatic system",
            ));
        }
    }

    let required = molecule
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(index, atom)| aromatic_atom_needs_double_bond(atom).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut matched = BTreeSet::new();
    let mut chosen = BTreeSet::new();
    if !match_aromatic_atoms(molecule, &incident, &required, &mut matched, &mut chosen) {
        let atom = required
            .iter()
            .find(|atom| !matched.contains(atom))
            .copied()
            .or_else(|| required.iter().next().copied())
            .unwrap_or(0);
        return Err(ChemistryError::atom(
            ChemistryErrorKind::Aromaticity,
            atom,
            "aromatic system cannot be kekulized with the declared atoms and charges",
        ));
    }
    Ok(chosen)
}

fn aromatic_path_exists_without_edge(
    molecule: &Molecule,
    begin: usize,
    end: usize,
    excluded_bond: usize,
) -> bool {
    let mut seen = BTreeSet::from([begin]);
    let mut stack = vec![begin];
    while let Some(atom) = stack.pop() {
        for (bond_index, bond) in molecule.bonds.iter().enumerate() {
            if bond_index == excluded_bond || bond.kind != BondKind::Aromatic {
                continue;
            }
            let neighbor = if bond.begin == atom {
                Some(bond.end)
            } else if bond.end == atom {
                Some(bond.begin)
            } else {
                None
            };
            let Some(neighbor) = neighbor else {
                continue;
            };
            if neighbor == end {
                return true;
            }
            if seen.insert(neighbor) {
                stack.push(neighbor);
            }
        }
    }
    false
}

fn aromatic_atom_needs_double_bond(atom: &Atom) -> bool {
    if !atom.aromatic {
        return false;
    }
    match atom.atomic_number {
        5 | 6 => atom.charge >= 0,
        7 | 15 | 33 => atom.explicit_hydrogens == 0 && atom.charge >= 0,
        8 | 16 | 34 => false,
        _ => false,
    }
}

fn match_aromatic_atoms(
    molecule: &Molecule,
    incident: &[Vec<usize>],
    required: &BTreeSet<usize>,
    matched: &mut BTreeSet<usize>,
    chosen: &mut BTreeSet<usize>,
) -> bool {
    let Some(atom) = required
        .iter()
        .find(|atom| !matched.contains(atom))
        .copied()
    else {
        return true;
    };
    let mut candidates = incident[atom].clone();
    candidates.sort_unstable();
    for bond_index in candidates {
        let bond = &molecule.bonds[bond_index];
        let neighbor = if bond.begin == atom {
            bond.end
        } else {
            bond.begin
        };
        if !required.contains(&neighbor) || matched.contains(&neighbor) {
            continue;
        }
        matched.insert(atom);
        matched.insert(neighbor);
        chosen.insert(bond_index);
        if match_aromatic_atoms(molecule, incident, required, matched, chosen) {
            return true;
        }
        chosen.remove(&bond_index);
        matched.remove(&neighbor);
        matched.remove(&atom);
    }
    false
}

fn allowed_valences(atom: &Atom) -> &'static [u8] {
    match (atom.atomic_number, atom.charge) {
        (1, _) => &[1],
        (5, -1) => &[4],
        (5, _) => &[3],
        (6, _) => &[4],
        (7, 1..) => &[4],
        (7, _) => &[3],
        (8, 1..) => &[3],
        (8, -10..=-1) => &[1],
        (8, _) => &[2],
        (9, _) => &[1],
        (14, _) => &[4],
        (15, 1..) => &[4, 6],
        (15, _) => &[3, 5],
        (16, 1..) => &[3, 5],
        (16, -10..=-1) => &[1, 3, 5],
        (16, _) => &[2, 4, 6],
        (17 | 35 | 53, _) => &[1, 3, 5, 7],
        _ => &[],
    }
}

fn format_valence(value_twice: u16) -> String {
    if value_twice.is_multiple_of(2) {
        (value_twice / 2).to_string()
    } else {
        format!("{}.5", value_twice / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn derives_implicit_hydrogens_for_organic_subset() {
        let report = sanitize(&parse_smiles("CCO").unwrap()).unwrap();
        assert_eq!(report.implicit_hydrogens, vec![3, 2, 1]);
    }

    #[test]
    fn kekulizes_benzene_and_fused_aromatic_systems() {
        let benzene = sanitize(&parse_smiles("c1ccccc1").unwrap()).unwrap();
        assert_eq!(benzene.aromatic_double_bonds.len(), 3);
        assert_eq!(benzene.implicit_hydrogens, vec![1; 6]);

        let naphthalene = sanitize(&parse_smiles("c1ccc2ccccc2c1").unwrap()).unwrap();
        assert_eq!(naphthalene.aromatic_double_bonds.len(), 5);
        assert_eq!(naphthalene.implicit_hydrogens.iter().sum::<u8>(), 8);
    }

    #[test]
    fn rejects_non_kekulizable_aromatic_system() {
        let error = parse_smiles("c1cccc1").unwrap_err();
        assert!(error.message.contains("cannot be kekulized"));
    }

    #[test]
    fn rejects_acyclic_aromatic_bonds() {
        let error = parse_smiles("cc").unwrap_err();
        assert!(error.message.contains("must belong to a ring"));
    }

    #[test]
    fn accepts_common_charged_and_expanded_valence_structures() {
        for smiles in [
            "[NH4+]",
            "[O-][N+](=O)c1ccccc1",
            "[B-](F)(F)(F)F",
            "O=S(=O)(O)O",
            "OP(=O)(O)O",
            "[Na+].[O-]C(=O)C",
            "[nH]1cccc1",
            "o1cccc1",
        ] {
            let molecule = parse_smiles(smiles).unwrap_or_else(|error| panic!("{smiles}: {error}"));
            sanitize(&molecule).unwrap_or_else(|error| panic!("{smiles}: {error}"));
        }
    }

    #[test]
    fn rejects_neutral_quaternary_nitrogen() {
        let error = parse_smiles("N(C)(C)(C)C").unwrap_err();
        assert_eq!(error.kind, crate::SmilesErrorKind::Valence);
    }

    #[test]
    fn normalizes_double_bond_direction_markers_to_e_and_z() {
        let trans = sanitize(&parse_smiles("F/C=C/F").unwrap()).unwrap();
        let cis = sanitize(&parse_smiles("F/C=C\\F").unwrap()).unwrap();
        assert_eq!(
            trans.double_bond_stereo[0].configuration,
            DoubleBondConfiguration::E
        );
        assert_eq!(
            cis.double_bond_stereo[0].configuration,
            DoubleBondConfiguration::Z
        );
    }

    #[test]
    fn rejects_orphan_directional_bonds() {
        let error = parse_smiles("C/C").unwrap_err();
        assert_eq!(error.kind, crate::SmilesErrorKind::Unsupported);
        assert!(error.message.contains("not adjacent to a double bond"));
    }

    #[test]
    fn validates_tetrahedral_coordination_without_claiming_cip() {
        let valid = sanitize(&parse_smiles("N[C@@H](C)C(=O)O").unwrap()).unwrap();
        assert_eq!(valid.tetrahedral_centers.len(), 1);
        assert_eq!(valid.tetrahedral_centers[0].explicit_hydrogens, 1);

        let error = parse_smiles("[C@](F)(Cl)Br").unwrap_err();
        assert_eq!(error.kind, crate::SmilesErrorKind::Unsupported);
        assert!(error.message.contains("requires four ligands"));
    }
}
