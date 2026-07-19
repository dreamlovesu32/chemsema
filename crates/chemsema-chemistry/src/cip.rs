use crate::{BondKind, ChiralLigand, Chirality, CipDescriptor, Molecule};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
struct LigandKey(Vec<Vec<(u8, u16)>>);

impl Ord for LigandKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for LigandKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct PathState {
    atom: usize,
    previous: usize,
    visited: BTreeSet<usize>,
}

pub(crate) fn tetrahedral_descriptor(
    molecule: &Molecule,
    center: usize,
    order: &[ChiralLigand],
    parity: Chirality,
) -> Option<CipDescriptor> {
    if order.len() != 4 {
        return None;
    }
    let keys = order
        .iter()
        .map(|ligand| ligand_key(molecule, center, *ligand))
        .collect::<Vec<_>>();
    for left in 0..keys.len() {
        for right in left + 1..keys.len() {
            if keys[left] == keys[right] {
                return None;
            }
        }
    }
    let mut priority = (0..4).collect::<Vec<_>>();
    priority.sort_by(|left, right| keys[*right].cmp(&keys[*left]));
    let target = [priority[3], priority[0], priority[1], priority[2]];
    let odd = permutation_is_odd(&target);
    let normalized = if odd { parity.inverted() } else { parity };
    Some(match normalized {
        // With the lowest-priority ligand used as the viewing ligand,
        // OpenSMILES anticlockwise order corresponds to CIP R.
        Chirality::Anticlockwise => CipDescriptor::R,
        Chirality::Clockwise => CipDescriptor::S,
    })
}

fn ligand_key(molecule: &Molecule, center: usize, ligand: ChiralLigand) -> LigandKey {
    if ligand == ChiralLigand::Hydrogen {
        return LigandKey(vec![vec![(1, 0)]]);
    }
    let ChiralLigand::Atom(root) = ligand else {
        unreachable!()
    };
    let mut visited = BTreeSet::new();
    visited.insert(center);
    visited.insert(root);
    let mut frontier = vec![PathState {
        atom: root,
        previous: center,
        visited,
    }];
    let mut spheres = vec![vec![atom_descriptor(molecule, root)]];
    let limit = molecule.atoms.len().saturating_mul(2).saturating_add(4);
    for _ in 0..limit {
        let mut descriptors = Vec::new();
        let mut next = Vec::new();
        for state in frontier {
            for bond in &molecule.bonds {
                let neighbor = if bond.begin == state.atom {
                    bond.end
                } else if bond.end == state.atom {
                    bond.begin
                } else {
                    continue;
                };
                if neighbor == state.previous || bond.kind == BondKind::Dative {
                    continue;
                }
                let descriptor = atom_descriptor(molecule, neighbor);
                let multiplicity = bond_multiplicity(bond.kind);
                for _ in 0..multiplicity {
                    descriptors.push(descriptor);
                }
                if !state.visited.contains(&neighbor) {
                    let mut visited = state.visited.clone();
                    visited.insert(neighbor);
                    next.push(PathState {
                        atom: neighbor,
                        previous: state.atom,
                        visited,
                    });
                }
            }
        }
        if descriptors.is_empty() {
            break;
        }
        descriptors.sort_unstable_by(|left, right| right.cmp(left));
        spheres.push(descriptors);
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    LigandKey(spheres)
}

fn atom_descriptor(molecule: &Molecule, atom: usize) -> (u8, u16) {
    let value = &molecule.atoms[atom];
    (value.atomic_number, value.isotope.unwrap_or(0))
}

fn bond_multiplicity(kind: BondKind) -> usize {
    match kind {
        BondKind::Single => 1,
        BondKind::Double | BondKind::Aromatic => 2,
        BondKind::Triple => 3,
        BondKind::Quadruple => 4,
        BondKind::Dative => 0,
    }
}

pub(crate) fn permutation_is_odd(order: &[usize]) -> bool {
    let mut inversions = 0;
    for left in 0..order.len() {
        for right in left + 1..order.len() {
            inversions += usize::from(order[left] > order[right]);
        }
    }
    inversions % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn assigns_common_tetrahedral_cip_descriptors() {
        let l = parse_smiles("N[C@@H](C)C(=O)O").unwrap();
        let d = parse_smiles("N[C@H](C)C(=O)O").unwrap();
        assert_eq!(
            crate::sanitize(&l).unwrap().tetrahedral_centers[0].cip,
            Some(CipDescriptor::S)
        );
        assert_eq!(
            crate::sanitize(&d).unwrap().tetrahedral_centers[0].cip,
            Some(CipDescriptor::R)
        );
    }
}
