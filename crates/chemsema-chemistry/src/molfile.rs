use crate::{
    layout_2d, sanitize, BondKind, ChiralLigand, Chirality, LayoutOptions, Molecule, Point2,
};
use std::collections::{BTreeMap, BTreeSet};

type BondStereoAssignments = BTreeMap<usize, (usize, u8)>;
type HydrogenStereoAssignments = BTreeMap<usize, u8>;
type TetrahedralStereoAssignments = (BondStereoAssignments, HydrogenStereoAssignments);

pub fn write_molfile_v2000(molecule: &Molecule) -> Result<String, String> {
    if molecule.atoms.is_empty() {
        return Err("molecule has no atoms".to_string());
    }
    let sanitization = sanitize(molecule).map_err(|error| error.to_string())?;
    let mut points = layout_2d(
        molecule,
        LayoutOptions {
            bond_length: 1.5,
            component_gap: 3.0,
            relaxation_steps: 120,
        },
    );
    project_double_bond_stereo(molecule, &sanitization, &mut points)?;
    let explicit_hydrogen_count = molecule
        .atoms
        .iter()
        .map(|atom| atom.explicit_hydrogens as usize)
        .sum::<usize>();
    let atom_count = molecule.atoms.len() + explicit_hydrogen_count;
    let bond_count = molecule.bonds.len() + explicit_hydrogen_count;
    if atom_count > 999 || bond_count > 999 {
        return Err("V2000 InChI bridge supports at most 999 atoms and bonds".to_string());
    }
    let mut output = String::from("ChemSema\n  ChemSema 1.0\n\n");
    output.push_str(&format!(
        "{:>3}{:>3}  0  0{:>3}  0            999 V2000\n",
        atom_count,
        bond_count,
        usize::from(!sanitization.tetrahedral_centers.is_empty())
    ));
    for (atom, point) in molecule.atoms.iter().zip(&points) {
        output.push_str(&atom_line(*point, &atom.symbol));
    }
    let mut hydrogen_positions = Vec::new();
    for (atom_index, atom) in molecule.atoms.iter().enumerate() {
        for hydrogen_index in 0..atom.explicit_hydrogens {
            let angle = std::f64::consts::TAU * (hydrogen_index as f64 + 1.0)
                / (atom.explicit_hydrogens as f64 + 1.0);
            let point = Point2 {
                x: points[atom_index].x + 0.85 * angle.cos(),
                y: points[atom_index].y + 0.85 * angle.sin(),
            };
            hydrogen_positions.push((atom_index, point));
            output.push_str(&atom_line(point, "H"));
        }
    }
    let (bond_stereo, hydrogen_stereo) =
        project_tetrahedral_stereo(molecule, &sanitization, &points, &hydrogen_positions)?;
    for (bond_index, bond) in molecule.bonds.iter().enumerate() {
        let bond_type = match bond.kind {
            BondKind::Single => 1,
            BondKind::Double => 2,
            BondKind::Triple => 3,
            BondKind::Aromatic => {
                if sanitization.aromatic_double_bonds.contains(&bond_index) {
                    2
                } else {
                    1
                }
            }
            BondKind::Quadruple => {
                return Err("V2000 InChI bridge does not support quadruple bonds".to_string())
            }
            BondKind::Dative => {
                return Err(
                    "standard InChI generation does not silently convert dative bonds; a metal-disconnection policy is required"
                        .to_string(),
                )
            }
        };
        let (begin, end, stereo) = if let Some((center, code)) = bond_stereo.get(&bond_index) {
            let other = if bond.begin == *center {
                bond.end
            } else {
                bond.begin
            };
            (*center, other, *code)
        } else {
            (bond.begin, bond.end, 0)
        };
        output.push_str(&format!(
            "{:>3}{:>3}{:>3}{:>3}  0  0  0\n",
            begin + 1,
            end + 1,
            bond_type,
            stereo
        ));
    }
    for (offset, (parent, _)) in hydrogen_positions.iter().enumerate() {
        output.push_str(&format!(
            "{:>3}{:>3}{:>3}{:>3}  0  0  0\n",
            parent + 1,
            molecule.atoms.len() + offset + 1,
            1,
            hydrogen_stereo.get(&offset).copied().unwrap_or(0)
        ));
    }
    let charges = molecule
        .atoms
        .iter()
        .enumerate()
        .filter(|(_, atom)| atom.charge != 0)
        .map(|(index, atom)| (index + 1, atom.charge))
        .collect::<Vec<_>>();
    for chunk in charges.chunks(8) {
        output.push_str(&format!("M  CHG{:>3}", chunk.len()));
        for (atom, charge) in chunk {
            output.push_str(&format!("{:>4}{:>4}", atom, charge));
        }
        output.push('\n');
    }
    let isotopes = molecule
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(index, atom)| atom.isotope.map(|isotope| (index + 1, isotope)))
        .collect::<Vec<_>>();
    for chunk in isotopes.chunks(8) {
        output.push_str(&format!("M  ISO{:>3}", chunk.len()));
        for (atom, isotope) in chunk {
            output.push_str(&format!("{:>4}{:>4}", atom, isotope));
        }
        output.push('\n');
    }
    output.push_str("M  END\n");
    Ok(output)
}

fn project_tetrahedral_stereo(
    molecule: &Molecule,
    sanitization: &crate::Sanitization,
    points: &[Point2],
    hydrogen_positions: &[(usize, Point2)],
) -> Result<TetrahedralStereoAssignments, String> {
    let mut bond_result = BTreeMap::new();
    let mut hydrogen_result = BTreeMap::new();
    let mut used_bonds = BTreeSet::new();
    for center in &sanitization.tetrahedral_centers {
        let hydrogen_offset = hydrogen_positions
            .iter()
            .position(|(parent, _)| *parent == center.atom_index);
        let bond_candidates = center
            .ligand_order
            .iter()
            .filter_map(|ligand| {
                let ChiralLigand::Atom(neighbor) = ligand else {
                    return None;
                };
                molecule.bonds.iter().enumerate().find_map(|(index, bond)| {
                    (bond.kind == BondKind::Single
                        && ((bond.begin == center.atom_index && bond.end == *neighbor)
                            || (bond.end == center.atom_index && bond.begin == *neighbor))
                        && !used_bonds.contains(&index))
                    .then_some(index)
                })
            })
            .collect::<Vec<_>>();
        let chosen_bond = if hydrogen_offset.is_none() {
            bond_candidates.first().copied()
        } else {
            None
        };
        if hydrogen_offset.is_none() && chosen_bond.is_none() {
            return Err(format!(
                "cannot project tetrahedral center {} onto an unused single stereobond",
                center.atom_index + 1
            ));
        }
        let chosen_ligand = if hydrogen_offset.is_some() {
            ChiralLigand::Hydrogen
        } else {
            let bond = &molecule.bonds[chosen_bond.unwrap()];
            ChiralLigand::Atom(if bond.begin == center.atom_index {
                bond.end
            } else {
                bond.begin
            })
        };
        let volume =
            tetrahedral_volume(molecule, points, hydrogen_positions, center, chosen_ligand)?;
        if volume.abs() < 1e-10 {
            return Err(format!(
                "cannot project tetrahedral center {} from degenerate 2D coordinates",
                center.atom_index + 1
            ));
        }
        let wants_negative = center.smiles_parity == Chirality::Anticlockwise;
        let code = if (volume < 0.0) == wants_negative {
            1
        } else {
            6
        };
        if let Some(offset) = hydrogen_offset {
            hydrogen_result.insert(offset, code);
        } else {
            let bond = chosen_bond.unwrap();
            used_bonds.insert(bond);
            bond_result.insert(bond, (center.atom_index, code));
        }
    }
    Ok((bond_result, hydrogen_result))
}

fn tetrahedral_volume(
    molecule: &Molecule,
    points: &[Point2],
    hydrogen_positions: &[(usize, Point2)],
    center: &crate::TetrahedralCenter,
    elevated: ChiralLigand,
) -> Result<f64, String> {
    let origin = points[center.atom_index];
    let ligand_point = |ligand: ChiralLigand| -> Result<[f64; 3], String> {
        let point = match ligand {
            ChiralLigand::Atom(atom) => points[atom],
            ChiralLigand::Hydrogen => hydrogen_positions
                .iter()
                .find_map(|(parent, point)| (*parent == center.atom_index).then_some(*point))
                .ok_or_else(|| "explicit stereochemical hydrogen is missing".to_string())?,
        };
        Ok([
            point.x - origin.x,
            -(point.y - origin.y),
            f64::from(ligand == elevated),
        ])
    };
    let vectors = center
        .ligand_order
        .iter()
        .map(|ligand| ligand_point(*ligand))
        .collect::<Result<Vec<_>, _>>()?;
    let a = subtract3(vectors[1], vectors[0]);
    let b = subtract3(vectors[2], vectors[0]);
    let c = subtract3(vectors[3], vectors[0]);
    let _ = molecule;
    Ok(dot3(a, cross3(b, c)))
}

fn subtract3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn project_double_bond_stereo(
    molecule: &Molecule,
    sanitization: &crate::Sanitization,
    points: &mut [Point2],
) -> Result<(), String> {
    for stereo in &sanitization.double_bond_stereo {
        let double = &molecule.bonds[stereo.bond_index];
        let begin = points[double.begin];
        let end = points[double.end];
        let dx = end.x - begin.x;
        let dy = end.y - begin.y;
        let length = dx.hypot(dy);
        if length <= f64::EPSILON {
            return Err("cannot project E/Z stereo from a zero-length double bond".to_string());
        }
        let axis = Point2 {
            x: dx / length,
            y: dy / length,
        };
        let normal = Point2 {
            x: -axis.y,
            y: axis.x,
        };
        let end_same_side = stereo.configuration == crate::DoubleBondConfiguration::Z;
        project_endpoint_substituents(
            molecule,
            points,
            double.begin,
            double.end,
            stereo.begin_reference_bond,
            begin,
            Point2 {
                x: -axis.x,
                y: -axis.y,
            },
            normal,
        );
        project_endpoint_substituents(
            molecule,
            points,
            double.end,
            double.begin,
            stereo.end_reference_bond,
            end,
            axis,
            if end_same_side {
                normal
            } else {
                Point2 {
                    x: -normal.x,
                    y: -normal.y,
                }
            },
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn project_endpoint_substituents(
    molecule: &Molecule,
    points: &mut [Point2],
    center: usize,
    opposite_double_atom: usize,
    reference_bond: usize,
    origin: Point2,
    outward: Point2,
    reference_side: Point2,
) {
    let neighbors = molecule
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(bond_index, bond)| {
            let neighbor = if bond.begin == center {
                Some(bond.end)
            } else if bond.end == center {
                Some(bond.begin)
            } else {
                None
            }?;
            (neighbor != opposite_double_atom).then_some((bond_index, neighbor))
        })
        .collect::<Vec<_>>();
    for (bond_index, neighbor) in neighbors {
        let side = if bond_index == reference_bond {
            reference_side
        } else {
            Point2 {
                x: -reference_side.x,
                y: -reference_side.y,
            }
        };
        points[neighbor] = Point2 {
            x: origin.x + outward.x * 1.05 + side.x * 0.8,
            y: origin.y + outward.y * 1.05 + side.y * 0.8,
        };
    }
}

fn atom_line(point: Point2, symbol: &str) -> String {
    format!(
        "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
        point.x, -point.y, 0.0, symbol
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn writes_disconnected_charged_isotopic_v2000() {
        let molecule = parse_smiles("[13CH3-].[Na+]").unwrap();
        let molfile = write_molfile_v2000(&molecule).unwrap();
        assert!(molfile.contains("  5  3"));
        assert!(molfile.contains("M  CHG  2"));
        assert!(molfile.contains("M  ISO  1"));
        assert!(molfile.ends_with("M  END\n"));
    }

    #[test]
    fn dative_bonds_fail_instead_of_becoming_single_bonds() {
        let molecule = parse_smiles("N->[Fe+2]").unwrap();
        let error = write_molfile_v2000(&molecule).unwrap_err();
        assert!(error.contains("does not silently convert dative bonds"));
    }

    #[test]
    fn double_bond_stereo_is_projected_to_opposite_geometries() {
        let molecule = parse_smiles("F/C=C/F").unwrap();
        let trans = write_molfile_v2000(&molecule).unwrap();
        let cis = write_molfile_v2000(&parse_smiles("F/C=C\\F").unwrap()).unwrap();
        assert_ne!(trans, cis);
    }

    #[test]
    fn tetrahedral_smiles_is_projected_to_opposite_wedges() {
        let left = write_molfile_v2000(&parse_smiles("N[C@@H](C)C(=O)O").unwrap()).unwrap();
        let right = write_molfile_v2000(&parse_smiles("N[C@H](C)C(=O)O").unwrap()).unwrap();
        assert_ne!(left, right);
        assert!(left.lines().any(|line| line
            .get(9..12)
            .is_some_and(|field| field.trim() == "1" || field.trim() == "6")));
    }
}
