use crate::Molecule;
use std::f64::consts::{FRAC_PI_3, PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOptions {
    pub bond_length: f64,
    pub component_gap: f64,
    pub relaxation_steps: usize,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            bond_length: 54.0,
            component_gap: 108.0,
            relaxation_steps: 120,
        }
    }
}

/// Generates deterministic initial coordinates. The API is intentionally
/// separate so a fuller RDKit-derived depiction port can replace this stage.
pub fn layout_2d(molecule: &Molecule, options: LayoutOptions) -> Vec<Point2> {
    if molecule.atoms.is_empty() {
        return Vec::new();
    }
    let adjacency = adjacency(molecule);
    let components = molecule.components();
    let mut points = vec![Point2 { x: 0.0, y: 0.0 }; molecule.atoms.len()];
    let mut placed = vec![false; molecule.atoms.len()];
    let mut component_offset = 0.0;
    for component in components {
        let root = component[0];
        points[root] = Point2 {
            x: component_offset,
            y: 0.0,
        };
        placed[root] = true;
        let mut stack = vec![(root, None, 0.0)];
        while let Some((atom, parent, incoming_angle)) = stack.pop() {
            let unplaced = adjacency[atom]
                .iter()
                .copied()
                .filter(|neighbor| !placed[*neighbor])
                .collect::<Vec<_>>();
            let count = unplaced.len();
            for (index, neighbor) in unplaced.into_iter().enumerate() {
                let angle = if parent.is_none() {
                    if count == 1 {
                        0.0
                    } else {
                        TAU * index as f64 / count as f64
                    }
                } else {
                    incoming_angle
                        + PI
                        + (index as f64 - count.saturating_sub(1) as f64 / 2.0) * FRAC_PI_3
                };
                points[neighbor] = Point2 {
                    x: points[atom].x + options.bond_length * angle.cos(),
                    y: points[atom].y + options.bond_length * angle.sin(),
                };
                placed[neighbor] = true;
                stack.push((neighbor, Some(atom), angle));
            }
        }
        relax_component(molecule, &component, &mut points, options);
        let (min_x, max_x) = component
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |range, atom| {
                (range.0.min(points[*atom].x), range.1.max(points[*atom].x))
            });
        let shift = component_offset - min_x;
        for atom in &component {
            points[*atom].x += shift;
        }
        component_offset += max_x - min_x + options.component_gap;
    }
    points
}

fn adjacency(molecule: &Molecule) -> Vec<Vec<usize>> {
    let mut result = vec![Vec::new(); molecule.atoms.len()];
    for bond in &molecule.bonds {
        result[bond.begin].push(bond.end);
        result[bond.end].push(bond.begin);
    }
    for neighbors in &mut result {
        neighbors.sort_unstable();
        neighbors.dedup();
    }
    result
}

fn relax_component(
    molecule: &Molecule,
    component: &[usize],
    points: &mut [Point2],
    options: LayoutOptions,
) {
    if component.len() < 3 {
        return;
    }
    let member = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut forces = vec![Point2 { x: 0.0, y: 0.0 }; points.len()];
    for step in 0..options.relaxation_steps {
        for atom in component {
            forces[*atom] = Point2 { x: 0.0, y: 0.0 };
        }
        for bond in &molecule.bonds {
            if !member.contains(&bond.begin) {
                continue;
            }
            let dx = points[bond.end].x - points[bond.begin].x;
            let dy = points[bond.end].y - points[bond.begin].y;
            let distance = (dx * dx + dy * dy).sqrt().max(0.001);
            let strength = (distance - options.bond_length) * 0.08;
            let (fx, fy) = (strength * dx / distance, strength * dy / distance);
            forces[bond.begin].x += fx;
            forces[bond.begin].y += fy;
            forces[bond.end].x -= fx;
            forces[bond.end].y -= fy;
        }
        for (position, a) in component.iter().enumerate() {
            for b in component.iter().skip(position + 1) {
                let dx = points[*b].x - points[*a].x;
                let dy = points[*b].y - points[*a].y;
                let distance2 = (dx * dx + dy * dy).max(16.0);
                let distance = distance2.sqrt();
                let strength = options.bond_length * options.bond_length * 0.035 / distance2;
                let (fx, fy) = (strength * dx / distance, strength * dy / distance);
                forces[*a].x -= fx;
                forces[*a].y -= fy;
                forces[*b].x += fx;
                forces[*b].y += fy;
            }
        }
        let cooling = 1.0 - step as f64 / options.relaxation_steps.max(1) as f64;
        for atom in component.iter().skip(1) {
            points[*atom].x += forces[*atom].x.clamp(-4.0, 4.0) * cooling;
            points[*atom].y += forces[*atom].y.clamp(-4.0, 4.0) * cooling;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_smiles;

    #[test]
    fn layout_is_finite_and_separates_components() {
        let molecule = parse_smiles("c1ccccc1.CC").unwrap();
        let points = layout_2d(&molecule, LayoutOptions::default());
        assert_eq!(points.len(), molecule.atoms.len());
        assert!(points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite()));
        let components = molecule.components();
        let first_max = components[0]
            .iter()
            .map(|atom| points[*atom].x)
            .fold(f64::NEG_INFINITY, f64::max);
        let second_min = components[1]
            .iter()
            .map(|atom| points[*atom].x)
            .fold(f64::INFINITY, f64::min);
        assert!(second_min > first_max);
    }
}
