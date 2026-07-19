use crate::{sanitize, ChemistryError, Molecule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MolecularProperties {
    pub formula: String,
    pub formal_charge: i32,
    pub atom_count: usize,
    pub heavy_atom_count: usize,
    pub component_count: usize,
}

pub fn molecular_properties(molecule: &Molecule) -> Result<MolecularProperties, ChemistryError> {
    let sanitization = sanitize(molecule)?;
    let mut counts = BTreeMap::<String, usize>::new();
    for (index, atom) in molecule.atoms.iter().enumerate() {
        *counts.entry(atom.symbol.clone()).or_default() += 1;
        let attached_hydrogens = usize::from(atom.explicit_hydrogens)
            + usize::from(sanitization.implicit_hydrogens[index]);
        if attached_hydrogens > 0 {
            *counts.entry("H".to_string()).or_default() += attached_hydrogens;
        }
    }
    let formula = hill_formula(&counts);
    Ok(MolecularProperties {
        formula,
        formal_charge: molecule.atoms.iter().map(|atom| atom.charge).sum(),
        atom_count: molecule.atoms.len(),
        heavy_atom_count: molecule
            .atoms
            .iter()
            .filter(|atom| atom.atomic_number > 1)
            .count(),
        component_count: molecule.components().len(),
    })
}

fn hill_formula(counts: &BTreeMap<String, usize>) -> String {
    let mut ordered = Vec::new();
    if counts.contains_key("C") {
        ordered.push("C".to_string());
        if counts.contains_key("H") {
            ordered.push("H".to_string());
        }
        ordered.extend(
            counts
                .keys()
                .filter(|symbol| symbol.as_str() != "C" && symbol.as_str() != "H")
                .cloned(),
        );
    } else {
        ordered.extend(counts.keys().cloned());
    }
    ordered
        .into_iter()
        .map(|symbol| {
            let count = counts[&symbol];
            if count == 1 {
                symbol
            } else {
                format!("{symbol}{count}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn computes_hill_formula_with_implicit_hydrogens_and_charge() {
        let ethanol = molecular_properties(&parse_smiles("CCO").unwrap()).unwrap();
        assert_eq!(ethanol.formula, "C2H6O");
        assert_eq!(ethanol.formal_charge, 0);
        assert_eq!(ethanol.heavy_atom_count, 3);

        let ammonium = molecular_properties(&parse_smiles("[NH4+]").unwrap()).unwrap();
        assert_eq!(ammonium.formula, "H4N");
        assert_eq!(ammonium.formal_charge, 1);
    }

    #[test]
    fn counts_disconnected_components() {
        let salt = molecular_properties(&parse_smiles("[Na+].[Cl-]").unwrap()).unwrap();
        assert_eq!(salt.formula, "ClNa");
        assert_eq!(salt.component_count, 2);
        assert_eq!(salt.formal_charge, 0);
    }
}
