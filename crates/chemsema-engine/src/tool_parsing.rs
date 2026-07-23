use crate::{BondVariant, BracketKind};

pub fn parse_bracket_tool_value(value: &str) -> BracketKind {
    match value {
        "square" => BracketKind::Square,
        "curly" => BracketKind::Curly,
        "double-dagger" | "doubleDagger" => BracketKind::DoubleDagger,
        "dagger" => BracketKind::Dagger,
        "circle-plus" | "circlePlus" => BracketKind::CirclePlus,
        "plus" => BracketKind::Plus,
        "radical-cation" | "radicalCation" => BracketKind::RadicalCation,
        "lone-pair" | "lonePair" => BracketKind::LonePair,
        "circle-minus" | "circleMinus" => BracketKind::CircleMinus,
        "minus" => BracketKind::Minus,
        "radical-anion" | "radicalAnion" => BracketKind::RadicalAnion,
        "electron" => BracketKind::Electron,
        _ => BracketKind::Round,
    }
}

pub fn parse_bond_tool_value(value: &str) -> BondVariant {
    match value {
        "double" => BondVariant::Double,
        "triple" => BondVariant::Triple,
        "dashed" => BondVariant::Dashed,
        "dashed-double" => BondVariant::DashedDouble,
        "bold" => BondVariant::Bold,
        "bold-dashed" => BondVariant::BoldDashed,
        "wavy" => BondVariant::Wavy,
        "wedge" => BondVariant::Wedge,
        "hashed-wedge" => BondVariant::HashedWedge,
        "hollow-wedge" => BondVariant::HollowWedge,
        _ => BondVariant::Single,
    }
}
