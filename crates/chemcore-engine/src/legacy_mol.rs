use crate::Vector;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct LegacyMol {
    pub(crate) atoms: Vec<LegacyAtom>,
    pub(crate) bonds: Vec<LegacyBond>,
    pub(crate) sgroups: Vec<LegacySgroup>,
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyAtom {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) symbol: String,
    pub(crate) charge: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyBond {
    pub(crate) begin: usize,
    pub(crate) end: usize,
    pub(crate) order: u8,
    pub(crate) stereo: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacySgroup {
    pub(crate) kind: String,
    pub(crate) atoms: Vec<usize>,
    pub(crate) label: String,
    pub(crate) bonds: Vec<usize>,
    pub(crate) vectors: BTreeMap<usize, Vector>,
}

pub(crate) fn parse_molblock(molblock: &str) -> Option<LegacyMol> {
    let normalized = molblock.replace('\r', "");
    let lines: Vec<&str> = normalized.lines().collect();
    if lines.len() < 4 {
        return None;
    }

    let counts_line = lines.get(3).copied().unwrap_or_default();
    let atom_count = parse_i32(slice_ascii(counts_line, 0, 3))?.max(0) as usize;
    let bond_count = parse_i32(slice_ascii(counts_line, 3, 6))?.max(0) as usize;

    let mut atoms = Vec::with_capacity(atom_count);
    let mut bonds = Vec::with_capacity(bond_count);
    let mut charges = BTreeMap::new();
    let mut sgroups: BTreeMap<String, LegacySgroup> = BTreeMap::new();

    for index in 0..atom_count {
        let line = lines.get(4 + index).copied().unwrap_or_default();
        atoms.push(LegacyAtom {
            x: parse_f64(slice_ascii(line, 0, 10)).unwrap_or(0.0),
            y: parse_f64(slice_ascii(line, 10, 20)).unwrap_or(0.0),
            symbol: {
                let symbol = slice_ascii(line, 31, 34).trim();
                if symbol.is_empty() {
                    "C".to_string()
                } else {
                    symbol.to_string()
                }
            },
            charge: 0,
        });
    }

    for index in 0..bond_count {
        let line = lines
            .get(4 + atom_count + index)
            .copied()
            .unwrap_or_default();
        let begin = parse_i32(slice_ascii(line, 0, 3)).unwrap_or(1).saturating_sub(1) as usize;
        let end = parse_i32(slice_ascii(line, 3, 6)).unwrap_or(1).saturating_sub(1) as usize;
        let order = parse_i32(slice_ascii(line, 6, 9)).unwrap_or(1).max(1) as u8;
        let stereo = parse_i32(slice_ascii(line, 9, 12)).unwrap_or(0).max(0) as u8;
        bonds.push(LegacyBond {
            begin,
            end,
            order,
            stereo,
        });
    }

    for line in lines.iter().skip(4 + atom_count + bond_count) {
        if line.starts_with("M  CHG") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let pair_count = parts.get(2).and_then(|value| parse_i32(value)).unwrap_or(0).max(0) as usize;
            for pair_index in 0..pair_count {
                let atom_index = parts
                    .get(3 + pair_index * 2)
                    .and_then(|value| parse_i32(value))
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                let charge = parts
                    .get(4 + pair_index * 2)
                    .and_then(|value| parse_i32(value))
                    .unwrap_or(0);
                charges.insert(atom_index, charge);
            }
            continue;
        }

        if line.starts_with("M  STY") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let pair_count = parts.get(2).and_then(|value| parse_i32(value)).unwrap_or(0).max(0) as usize;
            for pair_index in 0..pair_count {
                let sgroup_id = parts.get(3 + pair_index * 2).copied().unwrap_or_default();
                let sgroup_type = parts.get(4 + pair_index * 2).copied().unwrap_or_default();
                sgroups
                    .entry(sgroup_id.to_string())
                    .and_modify(|entry| entry.kind = sgroup_type.to_string())
                    .or_insert_with(|| LegacySgroup {
                        kind: sgroup_type.to_string(),
                        atoms: Vec::new(),
                        label: String::new(),
                        bonds: Vec::new(),
                        vectors: BTreeMap::new(),
                    });
            }
            continue;
        }

        if line.starts_with("M  SAL") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let sgroup_id = parts.get(2).copied().unwrap_or_default();
            let count = parts.get(3).and_then(|value| parse_i32(value)).unwrap_or(0).max(0) as usize;
            let entry = sgroups.entry(sgroup_id.to_string()).or_insert_with(|| LegacySgroup {
                kind: String::new(),
                atoms: Vec::new(),
                label: String::new(),
                bonds: Vec::new(),
                vectors: BTreeMap::new(),
            });
            for item_index in 0..count {
                let atom_index = parts
                    .get(4 + item_index)
                    .and_then(|value| parse_i32(value))
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                entry.atoms.push(atom_index);
            }
            continue;
        }

        if line.starts_with("M  SBL") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let sgroup_id = parts.get(2).copied().unwrap_or_default();
            let count = parts.get(3).and_then(|value| parse_i32(value)).unwrap_or(0).max(0) as usize;
            let entry = sgroups.entry(sgroup_id.to_string()).or_insert_with(|| LegacySgroup {
                kind: String::new(),
                atoms: Vec::new(),
                label: String::new(),
                bonds: Vec::new(),
                vectors: BTreeMap::new(),
            });
            for item_index in 0..count {
                let bond_index = parts
                    .get(4 + item_index)
                    .and_then(|value| parse_i32(value))
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                entry.bonds.push(bond_index);
            }
            continue;
        }

        if line.starts_with("M  SMT") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let sgroup_id = parts.get(2).copied().unwrap_or_default();
            let label = parts
                .iter()
                .skip(3)
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
                .replace("\\s^", "")
                .replace("\\n", "");
            sgroups
                .entry(sgroup_id.to_string())
                .and_modify(|entry| entry.label = label.clone())
                .or_insert_with(|| LegacySgroup {
                    kind: String::new(),
                    atoms: Vec::new(),
                    label,
                    bonds: Vec::new(),
                    vectors: BTreeMap::new(),
                });
            continue;
        }

        if line.starts_with("M  SBV") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let sgroup_id = parts.get(2).copied().unwrap_or_default();
            let bond_index = parts
                .get(3)
                .and_then(|value| parse_i32(value))
                .unwrap_or(1)
                .saturating_sub(1) as usize;
            let vector = Vector::new(
                parts.get(4).and_then(|value| parse_f64(value)).unwrap_or(0.0),
                parts.get(5).and_then(|value| parse_f64(value)).unwrap_or(0.0),
            );
            sgroups
                .entry(sgroup_id.to_string())
                .and_modify(|entry| {
                    entry.vectors.insert(bond_index, vector);
                })
                .or_insert_with(|| {
                    let mut vectors = BTreeMap::new();
                    vectors.insert(bond_index, vector);
                    LegacySgroup {
                        kind: String::new(),
                        atoms: Vec::new(),
                        label: String::new(),
                        bonds: Vec::new(),
                        vectors,
                    }
                });
        }
    }

    for (atom_index, charge) in charges {
        if let Some(atom) = atoms.get_mut(atom_index) {
            atom.charge = charge;
        }
    }

    if atoms.is_empty() {
        return None;
    }
    let min_x = atoms.iter().map(|atom| atom.x).fold(f64::INFINITY, f64::min);
    let max_x = atoms.iter().map(|atom| atom.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = atoms.iter().map(|atom| atom.y).fold(f64::INFINITY, f64::min);
    let max_y = atoms.iter().map(|atom| atom.y).fold(f64::NEG_INFINITY, f64::max);

    Some(LegacyMol {
        atoms,
        bonds,
        sgroups: sgroups.into_values().collect(),
        min_x,
        max_x,
        min_y,
        max_y,
    })
}

fn slice_ascii(input: &str, start: usize, end: usize) -> &str {
    input.get(start..end).unwrap_or_default()
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse().ok()
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}
