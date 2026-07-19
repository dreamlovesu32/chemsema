//! SMILES parser/writer.
//!
//! Grammar design derived from RDKit Code/GraphMol/SmilesParse/smiles.yy and
//! smiles.ll, commit 0062b670640352ab63d6256be608615e87e1af53,
//! BSD-3-Clause. Reimplemented and modified in Rust for ChemSema.

use crate::{
    canonical::canonical_ranks, sanitize, Atom, Bond, BondDirection, BondKind, ChemistryErrorKind,
    ChiralLigand, Chirality, Molecule,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmilesErrorKind {
    Syntax,
    Unsupported,
    UnclosedBranch,
    UnclosedRing,
    InvalidAtom,
    InvalidBond,
    Valence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmilesError {
    pub kind: SmilesErrorKind,
    pub message: String,
    pub offset: usize,
}

impl SmilesError {
    fn new(kind: SmilesErrorKind, message: impl Into<String>, offset: usize) -> Self {
        Self {
            kind,
            message: message.into(),
            offset,
        }
    }
}

impl fmt::Display for SmilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at byte {})", self.message, self.offset)
    }
}

impl std::error::Error for SmilesError {}

#[derive(Debug, Clone, Copy)]
struct PendingBond {
    kind: BondKind,
    direction: Option<BondDirection>,
    dative_reversed: bool,
    explicit: bool,
}

impl PendingBond {
    fn implicit() -> Self {
        Self {
            kind: BondKind::Single,
            direction: None,
            dative_reversed: false,
            explicit: false,
        }
    }
}

pub fn parse_smiles(input: &str) -> Result<Molecule, SmilesError> {
    Parser::new(input).parse()
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    molecule: Molecule,
    current: Option<usize>,
    branches: Vec<(usize, usize)>,
    rings: BTreeMap<u32, (usize, PendingBond, usize)>,
    ring_chiral_slots: BTreeMap<u32, (usize, usize)>,
    pending: PendingBond,
    expect_atom: bool,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            molecule: Molecule::new(),
            current: None,
            branches: Vec::new(),
            rings: BTreeMap::new(),
            ring_chiral_slots: BTreeMap::new(),
            pending: PendingBond::implicit(),
            expect_atom: true,
        }
    }

    fn parse(mut self) -> Result<Molecule, SmilesError> {
        if self.input.is_empty() {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "SMILES is empty",
                0,
            ));
        }
        while self.pos < self.input.len() {
            if self.input[self.pos..].starts_with("->") || self.input[self.pos..].starts_with("<-")
            {
                self.read_dative()?;
                continue;
            }
            match self.peek().unwrap() {
                '(' => self.open_branch()?,
                ')' => self.close_branch()?,
                '.' => self.disconnect()?,
                '-' | '=' | '#' | '$' | ':' | '/' | '\\' => self.read_bond()?,
                '<' | '>' => self.read_dative()?,
                '%' | '0'..='9' => self.read_ring()?,
                '[' => {
                    let atom = self.read_bracket_atom()?;
                    self.add_atom(atom)?;
                }
                _ => {
                    let atom = self.read_simple_atom()?;
                    self.add_atom(atom)?;
                }
            }
        }
        if let Some((_, offset)) = self.branches.last() {
            return Err(SmilesError::new(
                SmilesErrorKind::UnclosedBranch,
                "unclosed branch",
                *offset,
            ));
        }
        if let Some((number, (_, _, offset))) = self.rings.first_key_value() {
            return Err(SmilesError::new(
                SmilesErrorKind::UnclosedRing,
                format!("ring {number} is not closed"),
                *offset,
            ));
        }
        if self.expect_atom || self.pending.explicit {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "SMILES ends where an atom was expected",
                self.pos,
            ));
        }
        sanitize(&self.molecule).map_err(|error| {
            let kind = match error.kind {
                ChemistryErrorKind::InvalidBond | ChemistryErrorKind::DuplicateBond => {
                    SmilesErrorKind::InvalidBond
                }
                ChemistryErrorKind::Valence => SmilesErrorKind::Valence,
                ChemistryErrorKind::Aromaticity
                | ChemistryErrorKind::Stereochemistry
                | ChemistryErrorKind::Unsupported => SmilesErrorKind::Unsupported,
            };
            SmilesError::new(kind, error.to_string(), 0)
        })?;
        Ok(self.molecule)
    }

    fn open_branch(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        self.bump();
        let current = self.current.ok_or_else(|| {
            SmilesError::new(
                SmilesErrorKind::Syntax,
                "branch has no preceding atom",
                offset,
            )
        })?;
        if self.expect_atom {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "nested empty branch",
                offset,
            ));
        }
        self.branches.push((current, offset));
        self.expect_atom = true;
        Ok(())
    }

    fn close_branch(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        self.bump();
        if self.expect_atom {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "empty or incomplete branch",
                offset,
            ));
        }
        let (atom, _) = self.branches.pop().ok_or_else(|| {
            SmilesError::new(
                SmilesErrorKind::Syntax,
                "unmatched closing parenthesis",
                offset,
            )
        })?;
        self.current = Some(atom);
        self.pending = PendingBond::implicit();
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        self.bump();
        if self.current.is_none() || self.expect_atom || !self.branches.is_empty() {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "component separator is not between complete atoms",
                offset,
            ));
        }
        self.current = None;
        self.pending = PendingBond::implicit();
        self.expect_atom = true;
        Ok(())
    }

    fn read_bond(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        self.ensure_bond_position(offset)?;
        let ch = self.bump().unwrap();
        let (kind, direction) = match ch {
            '-' => (BondKind::Single, None),
            '=' => (BondKind::Double, None),
            '#' => (BondKind::Triple, None),
            '$' => (BondKind::Quadruple, None),
            ':' => (BondKind::Aromatic, None),
            '/' => (BondKind::Single, Some(BondDirection::Up)),
            '\\' => (BondKind::Single, Some(BondDirection::Down)),
            _ => unreachable!(),
        };
        self.pending = PendingBond {
            kind,
            direction,
            dative_reversed: false,
            explicit: true,
        };
        Ok(())
    }

    fn read_dative(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        self.ensure_bond_position(offset)?;
        let rest = &self.input[self.pos..];
        let dative_reversed = if rest.starts_with("->") {
            false
        } else if rest.starts_with("<-") {
            true
        } else {
            return Err(SmilesError::new(
                SmilesErrorKind::InvalidBond,
                "expected '->' or '<-'",
                offset,
            ));
        };
        self.pos += 2;
        self.pending = PendingBond {
            kind: BondKind::Dative,
            direction: None,
            dative_reversed,
            explicit: true,
        };
        Ok(())
    }

    fn ensure_bond_position(&self, offset: usize) -> Result<(), SmilesError> {
        if self.current.is_none() || self.pending.explicit {
            Err(SmilesError::new(
                SmilesErrorKind::InvalidBond,
                "bond is not after an atom",
                offset,
            ))
        } else {
            Ok(())
        }
    }

    fn read_ring(&mut self) -> Result<(), SmilesError> {
        let offset = self.pos;
        let current = self.current.ok_or_else(|| {
            SmilesError::new(
                SmilesErrorKind::Syntax,
                "ring number has no preceding atom",
                offset,
            )
        })?;
        if self.expect_atom {
            return Err(SmilesError::new(
                SmilesErrorKind::Syntax,
                "ring number appears before an atom",
                offset,
            ));
        }
        let number = if self.peek() == Some('%') {
            self.bump();
            let start = self.pos;
            while matches!(self.peek(), Some('0'..='9')) && self.pos - start < 3 {
                self.bump();
            }
            let digits = &self.input[start..self.pos];
            if digits.len() < 2 {
                return Err(SmilesError::new(
                    SmilesErrorKind::Syntax,
                    "percent ring number requires at least two digits",
                    offset,
                ));
            }
            digits.parse::<u32>().unwrap()
        } else {
            self.bump().unwrap().to_digit(10).unwrap()
        };
        if let Some((begin, opening, _)) = self.rings.remove(&number) {
            if begin == current {
                return Err(SmilesError::new(
                    SmilesErrorKind::InvalidBond,
                    "ring bond cannot connect an atom to itself",
                    offset,
                ));
            }
            let mut pending = merge_ring_bonds(opening, self.pending, offset)?;
            if !pending.explicit
                && self.molecule.atoms[begin].aromatic
                && self.molecule.atoms[current].aromatic
            {
                pending.kind = BondKind::Aromatic;
            }
            if let Some((atom, slot)) = self.ring_chiral_slots.remove(&number) {
                self.molecule.atoms[atom].chiral_order[slot] = ChiralLigand::Atom(current);
            }
            self.push_bond(begin, current, pending);
        } else {
            self.rings.insert(number, (current, self.pending, offset));
            if self.molecule.atoms[current].chirality.is_some() {
                let slot = self.molecule.atoms[current].chiral_order.len();
                self.molecule.atoms[current]
                    .chiral_order
                    .push(ChiralLigand::Atom(usize::MAX));
                self.ring_chiral_slots.insert(number, (current, slot));
            }
        }
        self.pending = PendingBond::implicit();
        Ok(())
    }

    fn read_simple_atom(&mut self) -> Result<Atom, SmilesError> {
        let offset = self.pos;
        let rest = &self.input[self.pos..];
        let token = if rest.starts_with("Cl") || rest.starts_with("Br") {
            &rest[..2]
        } else {
            let ch = rest.chars().next().unwrap();
            if matches!(
                ch,
                'B' | 'C'
                    | 'N'
                    | 'O'
                    | 'P'
                    | 'S'
                    | 'F'
                    | 'I'
                    | 'b'
                    | 'c'
                    | 'n'
                    | 'o'
                    | 'p'
                    | 's'
                    | '*'
            ) {
                &rest[..ch.len_utf8()]
            } else {
                return Err(SmilesError::new(
                    SmilesErrorKind::InvalidAtom,
                    format!("unexpected character '{ch}'"),
                    offset,
                ));
            }
        };
        self.pos += token.len();
        atom_from_symbol(token, false, offset)
    }

    fn read_bracket_atom(&mut self) -> Result<Atom, SmilesError> {
        let offset = self.pos;
        self.bump();
        let content_start = self.pos;
        let close = self.input[self.pos..]
            .find(']')
            .map(|value| self.pos + value)
            .ok_or_else(|| {
                SmilesError::new(SmilesErrorKind::Syntax, "unclosed bracket atom", offset)
            })?;
        let content = &self.input[content_start..close];
        self.pos = close + 1;
        parse_bracket_content(content, content_start)
    }

    fn add_atom(&mut self, mut atom: Atom) -> Result<(), SmilesError> {
        let index = self.molecule.atoms.len();
        if atom.chirality.is_some() {
            if let Some(previous) = self.current {
                atom.chiral_order.push(ChiralLigand::Atom(previous));
            }
            if atom.explicit_hydrogens == 1 {
                atom.chiral_order.push(ChiralLigand::Hydrogen);
            }
        }
        self.molecule.atoms.push(atom);
        if let Some(previous) = self.current {
            let mut pending = self.pending;
            if !pending.explicit
                && self.molecule.atoms[previous].aromatic
                && self.molecule.atoms[index].aromatic
            {
                pending.kind = BondKind::Aromatic;
            }
            self.push_bond(previous, index, pending);
        }
        self.current = Some(index);
        self.pending = PendingBond::implicit();
        self.expect_atom = false;
        Ok(())
    }

    fn push_bond(&mut self, begin: usize, end: usize, pending: PendingBond) {
        let (begin, end) = if pending.kind == BondKind::Dative && pending.dative_reversed {
            (end, begin)
        } else {
            (begin, end)
        };
        self.molecule.bonds.push(Bond {
            begin,
            end,
            kind: pending.kind,
            direction: pending.direction,
        });
        if pending.kind != BondKind::Dative {
            self.append_chiral_neighbor(begin, end);
            self.append_chiral_neighbor(end, begin);
        }
    }

    fn append_chiral_neighbor(&mut self, atom: usize, neighbor: usize) {
        if self.molecule.atoms[atom].chirality.is_some()
            && !self.molecule.atoms[atom]
                .chiral_order
                .contains(&ChiralLigand::Atom(neighbor))
        {
            self.molecule.atoms[atom]
                .chiral_order
                .push(ChiralLigand::Atom(neighbor));
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

fn merge_ring_bonds(
    opening: PendingBond,
    closing: PendingBond,
    offset: usize,
) -> Result<PendingBond, SmilesError> {
    if opening.explicit && closing.explicit && opening.kind != closing.kind {
        return Err(SmilesError::new(
            SmilesErrorKind::InvalidBond,
            "ring closure specifies conflicting bond types",
            offset,
        ));
    }
    Ok(if closing.explicit { closing } else { opening })
}

fn parse_bracket_content(content: &str, base: usize) -> Result<Atom, SmilesError> {
    let bytes = content.as_bytes();
    if bytes.is_empty() {
        return Err(SmilesError::new(
            SmilesErrorKind::InvalidAtom,
            "empty bracket atom",
            base,
        ));
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let isotope =
        if i > 0 {
            Some(content[..i].parse::<u16>().map_err(|_| {
                SmilesError::new(SmilesErrorKind::InvalidAtom, "invalid isotope", base)
            })?)
        } else {
            None
        };
    let symbol_start = i;
    if i >= bytes.len() {
        return Err(SmilesError::new(
            SmilesErrorKind::InvalidAtom,
            "bracket atom has no element",
            base + i,
        ));
    }
    if bytes[i] == b'*' {
        i += 1;
    } else if bytes[i].is_ascii_alphabetic() {
        i += 1;
        if i < bytes.len() && bytes[i].is_ascii_lowercase() {
            i += 1;
        }
    } else {
        return Err(SmilesError::new(
            SmilesErrorKind::InvalidAtom,
            "invalid bracket element",
            base + i,
        ));
    }
    let symbol = &content[symbol_start..i];
    let mut atom = atom_from_symbol(symbol, true, base + symbol_start)?;
    atom.isotope = isotope;
    atom.no_implicit = true;

    if content[i..].starts_with("@@") {
        atom.chirality = Some(Chirality::Clockwise);
        i += 2;
    } else if content[i..].starts_with('@') {
        atom.chirality = Some(Chirality::Anticlockwise);
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'H' {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        atom.explicit_hydrogens = if start == i {
            1
        } else {
            content[start..i].parse::<u8>().map_err(|_| {
                SmilesError::new(
                    SmilesErrorKind::InvalidAtom,
                    "invalid hydrogen count",
                    base + start,
                )
            })?
        };
    }
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        let sign = if bytes[i] == b'+' { 1 } else { -1 };
        let sign_byte = bytes[i];
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let magnitude = if start != i {
            content[start..i].parse::<i32>().unwrap()
        } else {
            let mut count = 1;
            while i < bytes.len() && bytes[i] == sign_byte {
                count += 1;
                i += 1;
            }
            count
        };
        atom.charge = sign * magnitude;
    }
    if i < bytes.len() && bytes[i] == b':' {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if start == i {
            return Err(SmilesError::new(
                SmilesErrorKind::InvalidAtom,
                "atom map requires a number",
                base + i,
            ));
        }
        atom.atom_map = Some(content[start..i].parse::<u32>().map_err(|_| {
            SmilesError::new(
                SmilesErrorKind::InvalidAtom,
                "invalid atom map",
                base + start,
            )
        })?);
    }
    if i != bytes.len() {
        return Err(SmilesError::new(
            SmilesErrorKind::Unsupported,
            format!("unsupported bracket atom suffix '{}'", &content[i..]),
            base + i,
        ));
    }
    Ok(atom)
}

fn atom_from_symbol(symbol: &str, bracketed: bool, offset: usize) -> Result<Atom, SmilesError> {
    let aromatic = symbol
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_lowercase());
    let canonical = if symbol == "*" {
        "*".to_owned()
    } else {
        let mut chars = symbol.chars();
        let first = chars.next().unwrap().to_ascii_uppercase();
        format!("{first}{}", chars.as_str().to_ascii_lowercase())
    };
    let atomic_number = atomic_number(&canonical).ok_or_else(|| {
        SmilesError::new(
            SmilesErrorKind::InvalidAtom,
            format!("unknown element '{symbol}'"),
            offset,
        )
    })?;
    if aromatic && !matches!(symbol, "b" | "c" | "n" | "o" | "p" | "s" | "se" | "as") {
        return Err(SmilesError::new(
            SmilesErrorKind::InvalidAtom,
            format!("unsupported aromatic element '{symbol}'"),
            offset,
        ));
    }
    Ok(Atom {
        atomic_number,
        symbol: canonical,
        isotope: None,
        charge: 0,
        explicit_hydrogens: 0,
        aromatic,
        chirality: None,
        chiral_order: Vec::new(),
        atom_map: None,
        no_implicit: bracketed,
    })
}

fn atomic_number(symbol: &str) -> Option<u8> {
    const ELEMENTS: [&str; 119] = [
        "*", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P",
        "S", "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
        "Ga", "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh",
        "Pd", "Ag", "Cd", "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd",
        "Pm", "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re",
        "Os", "Ir", "Pt", "Au", "Hg", "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th",
        "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db",
        "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
    ];
    ELEMENTS
        .iter()
        .position(|value| *value == symbol)
        .map(|value| value as u8)
}

pub fn write_smiles(molecule: &Molecule) -> Result<String, SmilesError> {
    if molecule.atoms.is_empty() {
        return Err(SmilesError::new(
            SmilesErrorKind::Syntax,
            "molecule has no atoms",
            0,
        ));
    }
    for (index, bond) in molecule.bonds.iter().enumerate() {
        if bond.begin >= molecule.atoms.len()
            || bond.end >= molecule.atoms.len()
            || bond.begin == bond.end
        {
            return Err(SmilesError::new(
                SmilesErrorKind::InvalidBond,
                format!("invalid bond {index}"),
                index,
            ));
        }
    }
    sanitize(molecule).map_err(chemistry_to_smiles_error)?;
    Writer::new(molecule).write()
}

pub fn write_canonical_smiles(molecule: &Molecule) -> Result<String, SmilesError> {
    let sanitization = sanitize(molecule).map_err(chemistry_to_smiles_error)?;
    let ranks = canonical_ranks(molecule).map_err(chemistry_to_smiles_error)?;
    let directions = canonical_double_bond_directions(molecule, &sanitization, &ranks)?;
    Writer::new_ranked(molecule, ranks, directions).write()
}

fn chemistry_to_smiles_error(error: crate::ChemistryError) -> SmilesError {
    let kind = match error.kind {
        ChemistryErrorKind::InvalidBond | ChemistryErrorKind::DuplicateBond => {
            SmilesErrorKind::InvalidBond
        }
        ChemistryErrorKind::Valence => SmilesErrorKind::Valence,
        ChemistryErrorKind::Aromaticity
        | ChemistryErrorKind::Stereochemistry
        | ChemistryErrorKind::Unsupported => SmilesErrorKind::Unsupported,
    };
    SmilesError::new(kind, error.to_string(), 0)
}

struct Writer<'a> {
    molecule: &'a Molecule,
    adjacency: Vec<Vec<(usize, usize)>>,
    tree_edges: BTreeSet<usize>,
    children: Vec<Vec<(usize, usize)>>,
    ring_marks: Vec<Vec<(u32, usize, bool, usize)>>,
    visited: Vec<bool>,
    roots: Vec<usize>,
    ranks: Vec<usize>,
    preorder: Vec<usize>,
    parent: Vec<Option<usize>>,
    directions: Vec<Option<BondDirection>>,
}

impl<'a> Writer<'a> {
    fn new(molecule: &'a Molecule) -> Self {
        Self::new_ranked(
            molecule,
            (0..molecule.atoms.len()).collect(),
            molecule.bonds.iter().map(|bond| bond.direction).collect(),
        )
    }

    fn new_ranked(
        molecule: &'a Molecule,
        ranks: Vec<usize>,
        directions: Vec<Option<BondDirection>>,
    ) -> Self {
        let mut adjacency = vec![Vec::new(); molecule.atoms.len()];
        for (bond_index, bond) in molecule.bonds.iter().enumerate() {
            adjacency[bond.begin].push((bond.end, bond_index));
            adjacency[bond.end].push((bond.begin, bond_index));
        }
        for neighbors in &mut adjacency {
            neighbors.sort_unstable_by_key(|(neighbor, bond)| (ranks[*neighbor], *bond));
        }
        Self {
            molecule,
            adjacency,
            tree_edges: BTreeSet::new(),
            children: vec![Vec::new(); molecule.atoms.len()],
            ring_marks: vec![Vec::new(); molecule.atoms.len()],
            visited: vec![false; molecule.atoms.len()],
            roots: Vec::new(),
            ranks,
            preorder: vec![usize::MAX; molecule.atoms.len()],
            parent: vec![None; molecule.atoms.len()],
            directions,
        }
    }

    fn write(mut self) -> Result<String, SmilesError> {
        let mut candidates = (0..self.molecule.atoms.len()).collect::<Vec<_>>();
        candidates.sort_by_key(|atom| self.ranks[*atom]);
        for root in candidates {
            if !self.visited[root] {
                self.roots.push(root);
                self.build_tree(root, None);
            }
        }
        let mut ring_edges = self
            .molecule
            .bonds
            .iter()
            .enumerate()
            .filter(|(bond_index, _)| !self.tree_edges.contains(bond_index))
            .collect::<Vec<_>>();
        ring_edges.sort_by_key(|(bond_index, bond)| {
            (
                self.ranks[bond.begin].min(self.ranks[bond.end]),
                self.ranks[bond.begin].max(self.ranks[bond.end]),
                *bond_index,
            )
        });
        for (ring_number, (bond_index, bond)) in (1u32..).zip(ring_edges) {
            let (first, second) = if self.preorder[bond.begin] <= self.preorder[bond.end] {
                (bond.begin, bond.end)
            } else {
                (bond.end, bond.begin)
            };
            self.ring_marks[first].push((ring_number, bond_index, true, second));
            self.ring_marks[second].push((ring_number, bond_index, false, first));
        }
        let mut output = String::new();
        for (index, root) in self.roots.iter().copied().enumerate() {
            if index > 0 {
                output.push('.');
            }
            self.write_atom(root, &mut output);
        }
        Ok(output)
    }

    fn build_tree(&mut self, atom: usize, parent_edge: Option<usize>) {
        self.visited[atom] = true;
        self.preorder[atom] = self
            .preorder
            .iter()
            .filter(|value| **value != usize::MAX)
            .count();
        for (neighbor, edge) in self.adjacency[atom].clone() {
            if Some(edge) == parent_edge {
                continue;
            }
            if !self.visited[neighbor] {
                self.tree_edges.insert(edge);
                self.children[atom].push((neighbor, edge));
                self.parent[neighbor] = Some(atom);
                self.build_tree(neighbor, Some(edge));
            }
        }
    }

    fn write_atom(&self, atom_index: usize, output: &mut String) {
        output.push_str(&self.atom_token(atom_index));
        let mut marks = self.ring_marks[atom_index].clone();
        marks.sort_unstable_by_key(|mark| mark.0);
        for (number, bond_index, first, _) in marks {
            if first {
                output.push_str(&self.bond_token(bond_index, atom_index));
            }
            push_ring_number(output, number);
        }
        let children = &self.children[atom_index];
        for (child, edge) in children.iter().skip(1) {
            output.push('(');
            output.push_str(&self.bond_token(*edge, atom_index));
            self.write_atom(*child, output);
            output.push(')');
        }
        if let Some((child, edge)) = children.first() {
            output.push_str(&self.bond_token(*edge, atom_index));
            self.write_atom(*child, output);
        }
    }

    fn atom_token(&self, atom_index: usize) -> String {
        let atom = &self.molecule.atoms[atom_index];
        let Some(mut parity) = atom.chirality else {
            return atom_token(atom, None);
        };
        let mut emitted = Vec::with_capacity(4);
        if let Some(parent) = self.parent[atom_index] {
            emitted.push(ChiralLigand::Atom(parent));
        }
        if atom.explicit_hydrogens == 1 {
            emitted.push(ChiralLigand::Hydrogen);
        }
        let mut marks = self.ring_marks[atom_index].clone();
        marks.sort_unstable_by_key(|mark| mark.0);
        emitted.extend(
            marks
                .into_iter()
                .map(|(_, _, _, neighbor)| ChiralLigand::Atom(neighbor)),
        );
        emitted.extend(
            self.children[atom_index]
                .iter()
                .skip(1)
                .chain(self.children[atom_index].iter().take(1))
                .map(|(neighbor, _)| ChiralLigand::Atom(*neighbor)),
        );
        let permutation = emitted
            .iter()
            .filter_map(|ligand| atom.chiral_order.iter().position(|value| value == ligand))
            .collect::<Vec<_>>();
        if permutation.len() == 4 && crate::cip::permutation_is_odd(&permutation) {
            parity = parity.inverted();
        }
        atom_token(atom, Some(parity))
    }

    fn bond_token(&self, bond_index: usize, from: usize) -> String {
        let bond = &self.molecule.bonds[bond_index];
        if bond.kind == BondKind::Dative {
            return if bond.begin == from { "->" } else { "<-" }.to_owned();
        }
        let direction = match (self.directions[bond_index], bond.begin == from) {
            (Some(direction), true) => Some(direction),
            (Some(BondDirection::Up), false) => Some(BondDirection::Down),
            (Some(BondDirection::Down), false) => Some(BondDirection::Up),
            (None, _) => None,
        };
        match (bond.kind, direction) {
            (BondKind::Single, Some(BondDirection::Up)) => "/",
            (BondKind::Single, Some(BondDirection::Down)) => "\\",
            (BondKind::Single, None) => "",
            (BondKind::Double, _) => "=",
            (BondKind::Triple, _) => "#",
            (BondKind::Quadruple, _) => "$",
            (BondKind::Aromatic, _) => ":",
            (BondKind::Dative, _) => unreachable!(),
        }
        .to_owned()
    }
}

fn atom_token(atom: &Atom, chirality: Option<Chirality>) -> String {
    let organic_subset = matches!(
        atom.symbol.as_str(),
        "B" | "C" | "N" | "O" | "P" | "S" | "F" | "Cl" | "Br" | "I"
    ) && !atom.no_implicit
        && atom.isotope.is_none()
        && atom.charge == 0
        && atom.explicit_hydrogens == 0
        && chirality.is_none()
        && atom.atom_map.is_none();
    if organic_subset {
        return if atom.aromatic {
            atom.symbol.to_ascii_lowercase()
        } else {
            atom.symbol.clone()
        };
    }
    let mut output = String::from("[");
    if let Some(isotope) = atom.isotope {
        output.push_str(&isotope.to_string());
    }
    if atom.aromatic {
        output.push_str(&atom.symbol.to_ascii_lowercase());
    } else {
        output.push_str(&atom.symbol);
    }
    match chirality {
        Some(Chirality::Anticlockwise) => output.push('@'),
        Some(Chirality::Clockwise) => output.push_str("@@"),
        None => {}
    }
    if atom.explicit_hydrogens > 0 {
        output.push('H');
        if atom.explicit_hydrogens > 1 {
            output.push_str(&atom.explicit_hydrogens.to_string());
        }
    }
    if atom.charge != 0 {
        output.push(if atom.charge > 0 { '+' } else { '-' });
        if atom.charge.unsigned_abs() > 1 {
            output.push_str(&atom.charge.unsigned_abs().to_string());
        }
    }
    if let Some(atom_map) = atom.atom_map {
        output.push(':');
        output.push_str(&atom_map.to_string());
    }
    output.push(']');
    output
}

fn canonical_double_bond_directions(
    molecule: &Molecule,
    sanitization: &crate::Sanitization,
    ranks: &[usize],
) -> Result<Vec<Option<BondDirection>>, SmilesError> {
    let mut result = vec![None; molecule.bonds.len()];
    for stereo in &sanitization.double_bond_stereo {
        let double = &molecule.bonds[stereo.bond_index];
        let begin_bond =
            canonical_substituent_bond(molecule, double.begin, stereo.bond_index, ranks)
                .ok_or_else(|| {
                    SmilesError::new(
                        SmilesErrorKind::Unsupported,
                        "double-bond stereo has no explicit begin substituent",
                        0,
                    )
                })?;
        let end_bond = canonical_substituent_bond(molecule, double.end, stereo.bond_index, ranks)
            .ok_or_else(|| {
            SmilesError::new(
                SmilesErrorKind::Unsupported,
                "double-bond stereo has no explicit end substituent",
                0,
            )
        })?;
        set_direction(
            &mut result,
            molecule,
            begin_bond,
            double.begin,
            BondDirection::Up,
        )?;
        let end_direction = if stereo.configuration == crate::DoubleBondConfiguration::Z {
            BondDirection::Up
        } else {
            BondDirection::Down
        };
        set_direction(&mut result, molecule, end_bond, double.end, end_direction)?;
    }
    Ok(result)
}

fn canonical_substituent_bond(
    molecule: &Molecule,
    center: usize,
    excluded: usize,
    ranks: &[usize],
) -> Option<usize> {
    molecule
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| {
            if index == excluded || bond.kind != BondKind::Single {
                return None;
            }
            let neighbor = if bond.begin == center {
                bond.end
            } else if bond.end == center {
                bond.begin
            } else {
                return None;
            };
            Some((ranks[neighbor], index))
        })
        .min()
        .map(|(_, index)| index)
}

fn set_direction(
    directions: &mut [Option<BondDirection>],
    molecule: &Molecule,
    bond_index: usize,
    from: usize,
    direction: BondDirection,
) -> Result<(), SmilesError> {
    let stored = if molecule.bonds[bond_index].begin == from {
        direction
    } else {
        match direction {
            BondDirection::Up => BondDirection::Down,
            BondDirection::Down => BondDirection::Up,
        }
    };
    if directions[bond_index].is_some_and(|current| current != stored) {
        return Err(SmilesError::new(
            SmilesErrorKind::Unsupported,
            "conjugated double-bond stereo requires a conflicting slash direction",
            0,
        ));
    }
    directions[bond_index] = Some(stored);
    Ok(())
}

fn push_ring_number(output: &mut String, number: u32) {
    if number < 10 {
        output.push(char::from_digit(number, 10).unwrap());
    } else {
        output.push('%');
        output.push_str(&number.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(input: &str) -> Molecule {
        let first = parse_smiles(input).unwrap();
        let output = write_smiles(&first).unwrap();
        let second = parse_smiles(&output).unwrap();
        assert_eq!(first.atoms.len(), second.atoms.len(), "{output}");
        assert_eq!(first.bonds.len(), second.bonds.len(), "{output}");
        second
    }

    #[test]
    fn parses_branches_rings_aromatic_and_components() {
        let molecule = round_trip("CC(=O)Oc1ccccc1C(=O)O.[Na+]");
        assert_eq!(molecule.components().len(), 2);
        assert!(molecule.atoms.iter().any(|atom| atom.aromatic));
        assert!(molecule
            .bonds
            .iter()
            .any(|bond| bond.kind == BondKind::Double));
    }

    #[test]
    fn preserves_isotope_chirality_hydrogen_charge_and_map() {
        let molecule = round_trip("[13C@@H:7](F)(Cl)[NH3+]");
        assert_eq!(molecule.atoms[0].isotope, Some(13));
        assert_eq!(molecule.atoms[0].chirality, Some(Chirality::Clockwise));
        assert_eq!(molecule.atoms[0].atom_map, Some(7));
        assert!(molecule.atoms.iter().any(|atom| atom.charge == 1));
    }

    #[test]
    fn dative_direction_is_semantic() {
        let molecule = parse_smiles("N->[Fe+2]<-N").unwrap();
        assert_eq!((molecule.bonds[0].begin, molecule.bonds[0].end), (0, 1));
        assert_eq!((molecule.bonds[1].begin, molecule.bonds[1].end), (2, 1));
        round_trip("N->[Fe+2]<-N");
    }

    #[test]
    fn reports_offsets() {
        let error = parse_smiles("C1CC").unwrap_err();
        assert_eq!(error.kind, SmilesErrorKind::UnclosedRing);
        assert_eq!(error.offset, 1);
    }

    #[test]
    fn rejects_over_valent_common_atoms() {
        let error = parse_smiles("C(C)(C)(C)(C)C").unwrap_err();
        assert_eq!(error.kind, SmilesErrorKind::Valence);
        assert!(error.message.contains("above the supported maximum"));
    }

    #[test]
    fn canonical_smiles_is_independent_of_input_traversal() {
        let variants = ["CC(O)C(=O)N", "NC(=O)C(O)C", "OC(C)C(N)=O"];
        let outputs = variants
            .iter()
            .map(|value| write_canonical_smiles(&parse_smiles(value).unwrap()).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(outputs.len(), 1, "{outputs:?}");

        let aromatic = ["c1ccccc1O", "Oc1ccccc1"]
            .iter()
            .map(|value| write_canonical_smiles(&parse_smiles(value).unwrap()).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(aromatic.len(), 1, "{aromatic:?}");
    }

    #[test]
    fn canonical_isomeric_smiles_normalizes_tetrahedral_traversal() {
        let same_enantiomer = ["N[C@@H](C)C(=O)O", "C[C@H](N)C(=O)O", "O=C(O)[C@H](C)N"]
            .iter()
            .map(|value| write_canonical_smiles(&parse_smiles(value).unwrap()).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(same_enantiomer.len(), 1, "{same_enantiomer:?}");

        let opposite = write_canonical_smiles(&parse_smiles("N[C@H](C)C(=O)O").unwrap()).unwrap();
        assert_ne!(same_enantiomer.first().unwrap(), &opposite);
    }

    #[test]
    fn ring_closure_ligands_keep_open_smiles_chiral_order() {
        let equivalent = ["FC1C[C@](Br)(Cl)CCC1", "[C@]1(Br)(Cl)CCCC(F)C1"]
            .iter()
            .map(|value| write_canonical_smiles(&parse_smiles(value).unwrap()).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(equivalent.len(), 1, "{equivalent:?}");
    }

    #[test]
    fn canonical_isomeric_smiles_normalizes_double_bond_slashes() {
        let equivalent = ["F/C=C/F", "F\\C=C\\F"]
            .iter()
            .map(|value| write_canonical_smiles(&parse_smiles(value).unwrap()).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(equivalent.len(), 1, "{equivalent:?}");
        let cis = write_canonical_smiles(&parse_smiles("F/C=C\\F").unwrap()).unwrap();
        assert_ne!(equivalent.first().unwrap(), &cis);
    }

    #[test]
    fn reverse_traversal_inverts_directional_bond_token() {
        let molecule = Molecule {
            atoms: vec![
                atom_from_symbol("F", false, 0).unwrap(),
                atom_from_symbol("C", false, 0).unwrap(),
                atom_from_symbol("C", false, 0).unwrap(),
            ],
            bonds: vec![
                Bond {
                    begin: 1,
                    end: 0,
                    kind: BondKind::Single,
                    direction: Some(BondDirection::Up),
                },
                Bond {
                    begin: 1,
                    end: 2,
                    kind: BondKind::Double,
                    direction: None,
                },
            ],
        };
        assert_eq!(write_smiles(&molecule).unwrap(), "F\\C=C");
    }

    #[test]
    fn parses_multiring_iron_complex_with_dative_bonds() {
        let input = "[Fe+2]12345678(<-C9(P(c%10c([H])c([H])c([H])c([H])c%10[H])c%10c([H])c([H])c([H])c([H])c%10[H])=C->1([H])C->2([H])=C->3([H])C49[H])<-C1(P(c2c([H])c([H])c([H])c([H])c2[H])c2c([H])c([H])c([H])c([H])c2[H])=C->5([H])C->6([H])=C->7([H])C81[H]";
        let molecule = parse_smiles(input).unwrap();
        assert_eq!(molecule.atoms[0].symbol, "Fe");
        assert!(
            molecule
                .bonds
                .iter()
                .filter(|bond| bond.kind == BondKind::Dative)
                .count()
                >= 8
        );
        let output = write_smiles(&molecule).unwrap();
        let reparsed = parse_smiles(&output).unwrap();
        assert_eq!(reparsed.atoms.len(), molecule.atoms.len());
        assert_eq!(reparsed.bonds.len(), molecule.bonds.len());
    }
}
