use crate::{Bond, BondKind, ChemistryError, ChemistryErrorKind, Molecule};
use std::collections::BTreeMap;

const MAX_CANONICAL_SEARCH_STATES: usize = 200_000;

pub(crate) fn canonical_ranks(molecule: &Molecule) -> Result<Vec<usize>, ChemistryError> {
    let mut components = Vec::new();
    let mut budget = MAX_CANONICAL_SEARCH_STATES;
    for atoms in molecule.components() {
        let (code, order) = canonical_component(molecule, &atoms, &mut budget)?;
        components.push((code, order));
    }
    components.sort_by(|left, right| left.0.cmp(&right.0));
    let mut ranks = vec![0; molecule.atoms.len()];
    let mut next_rank = 0;
    for (_, order) in components {
        for atom in order {
            ranks[atom] = next_rank;
            next_rank += 1;
        }
    }
    Ok(ranks)
}

fn canonical_component(
    molecule: &Molecule,
    atoms: &[usize],
    budget: &mut usize,
) -> Result<(String, Vec<usize>), ChemistryError> {
    let initial_signatures = atoms
        .iter()
        .map(|atom| atom_invariant(molecule, *atom))
        .collect::<Vec<_>>();
    let colors = rank_signatures(&initial_signatures);
    let mut best = None;
    search(molecule, atoms, colors, budget, &mut best)?;
    best.ok_or_else(|| ChemistryError {
        kind: ChemistryErrorKind::Unsupported,
        message: "canonical atom ranking did not produce a labeling".to_string(),
        atom_index: None,
        bond_index: None,
    })
}

fn search(
    molecule: &Molecule,
    atoms: &[usize],
    colors: Vec<usize>,
    budget: &mut usize,
    best: &mut Option<(String, Vec<usize>)>,
) -> Result<(), ChemistryError> {
    if *budget == 0 {
        return Err(ChemistryError {
            kind: ChemistryErrorKind::Unsupported,
            message: format!(
                "canonical atom ranking exceeded {} symmetry-search states",
                MAX_CANONICAL_SEARCH_STATES
            ),
            atom_index: None,
            bond_index: None,
        });
    }
    *budget -= 1;
    let colors = refine(molecule, atoms, colors);
    let classes = color_classes(atoms, &colors);
    let tied = classes
        .iter()
        .filter(|(_, members)| members.len() > 1)
        .min_by_key(|(color, members)| (members.len(), **color));
    let Some((_, members)) = tied else {
        let mut order = atoms.to_vec();
        order.sort_by_key(|atom| colors[position_of(atoms, *atom)]);
        let code = canonical_code(molecule, &order);
        if best.as_ref().is_none_or(|(best_code, _)| code < *best_code) {
            *best = Some((code, order));
        }
        return Ok(());
    };

    let next_color = colors.iter().copied().max().unwrap_or(0) + 1;
    for atom in members {
        let mut individualized = colors.clone();
        individualized[position_of(atoms, *atom)] = next_color;
        search(molecule, atoms, individualized, budget, best)?;
    }
    Ok(())
}

fn refine(molecule: &Molecule, atoms: &[usize], mut colors: Vec<usize>) -> Vec<usize> {
    loop {
        let signatures = atoms
            .iter()
            .enumerate()
            .map(|(position, atom)| {
                let mut neighbors = molecule
                    .bonds
                    .iter()
                    .filter_map(|bond| {
                        let neighbor = if bond.begin == *atom {
                            Some(bond.end)
                        } else if bond.end == *atom {
                            Some(bond.begin)
                        } else {
                            None
                        }?;
                        atoms
                            .iter()
                            .position(|candidate| *candidate == neighbor)
                            .map(|neighbor_position| {
                                format!(
                                    "{}:{}",
                                    bond_invariant(bond, *atom),
                                    colors[neighbor_position]
                                )
                            })
                    })
                    .collect::<Vec<_>>();
                neighbors.sort();
                format!(
                    "{}|{}|{}",
                    colors[position],
                    atom_invariant(molecule, *atom),
                    neighbors.join(",")
                )
            })
            .collect::<Vec<_>>();
        let refined = rank_signatures(&signatures);
        if refined == colors {
            return colors;
        }
        colors = refined;
    }
}

fn rank_signatures(signatures: &[String]) -> Vec<usize> {
    let mut unique = signatures.to_vec();
    unique.sort();
    unique.dedup();
    signatures
        .iter()
        .map(|signature| unique.binary_search(signature).unwrap())
        .collect()
}

fn color_classes(atoms: &[usize], colors: &[usize]) -> BTreeMap<usize, Vec<usize>> {
    let mut classes = BTreeMap::<usize, Vec<usize>>::new();
    for (position, atom) in atoms.iter().enumerate() {
        classes.entry(colors[position]).or_default().push(*atom);
    }
    classes
}

fn canonical_code(molecule: &Molecule, order: &[usize]) -> String {
    let positions = order
        .iter()
        .enumerate()
        .map(|(position, atom)| (*atom, position))
        .collect::<BTreeMap<_, _>>();
    let atoms = order
        .iter()
        .map(|atom| atom_invariant(molecule, *atom))
        .collect::<Vec<_>>()
        .join(";");
    let mut bonds = molecule
        .bonds
        .iter()
        .filter_map(|bond| {
            let left = positions.get(&bond.begin).copied()?;
            let right = positions.get(&bond.end).copied()?;
            let (first, second, from) = if left <= right {
                (left, right, bond.begin)
            } else {
                (right, left, bond.end)
            };
            Some(format!("{first}-{second}:{}", bond_invariant(bond, from)))
        })
        .collect::<Vec<_>>();
    bonds.sort();
    format!("{atoms}|{}", bonds.join(";"))
}

fn atom_invariant(molecule: &Molecule, atom: usize) -> String {
    let value = &molecule.atoms[atom];
    let degree = molecule
        .bonds
        .iter()
        .filter(|bond| bond.begin == atom || bond.end == atom)
        .count();
    format!(
        "{:03}:{:05}:{:+04}:{}:{}:{}:{}:{}:{}",
        value.atomic_number,
        value.isotope.unwrap_or(0),
        value.charge,
        value.explicit_hydrogens,
        u8::from(value.aromatic),
        u8::from(value.chirality.is_some()),
        value.atom_map.unwrap_or(0),
        u8::from(value.no_implicit),
        degree
    )
}

fn bond_invariant(bond: &Bond, from: usize) -> String {
    let kind = match bond.kind {
        BondKind::Single => 1,
        BondKind::Double => 2,
        BondKind::Triple => 3,
        BondKind::Quadruple => 4,
        BondKind::Aromatic => 5,
        BondKind::Dative => 6,
    };
    // Slash direction and @/@@ are traversal-relative. Absolute stereo is
    // normalized by the SMILES writer after constitutional ranking.
    let direction = 0;
    let dative = usize::from(bond.kind == BondKind::Dative && bond.begin != from);
    format!("{kind}:{direction}:{dative}")
}

fn position_of(atoms: &[usize], atom: usize) -> usize {
    atoms
        .iter()
        .position(|candidate| *candidate == atom)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn ranks_are_invariant_to_input_atom_order() {
        let first = parse_smiles("CC(O)C(=O)N").unwrap();
        let second = parse_smiles("NC(=O)C(O)C").unwrap();
        let first_ranks = canonical_ranks(&first).unwrap();
        let second_ranks = canonical_ranks(&second).unwrap();
        assert_eq!(
            canonical_code(&first, &order_from_ranks(&first_ranks)),
            canonical_code(&second, &order_from_ranks(&second_ranks))
        );
    }

    fn order_from_ranks(ranks: &[usize]) -> Vec<usize> {
        let mut order = (0..ranks.len()).collect::<Vec<_>>();
        order.sort_by_key(|atom| ranks[*atom]);
        order
    }
}
