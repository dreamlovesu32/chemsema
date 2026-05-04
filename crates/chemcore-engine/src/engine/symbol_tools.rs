use super::{SymbolOrbitAnchor, SymbolOrbitMode};
use crate::{angle_between, BracketKind, Point};

pub(super) fn bracket_kind_name(kind: BracketKind) -> &'static str {
    match kind {
        BracketKind::Round => "round",
        BracketKind::Square => "square",
        BracketKind::Curly => "curly",
        BracketKind::DoubleDagger => "double-dagger",
        BracketKind::Dagger => "dagger",
        BracketKind::CirclePlus => "circle-plus",
        BracketKind::Plus => "plus",
        BracketKind::RadicalCation => "radical-cation",
        BracketKind::LonePair => "lone-pair",
        BracketKind::CircleMinus => "circle-minus",
        BracketKind::Minus => "minus",
        BracketKind::RadicalAnion => "radical-anion",
        BracketKind::Electron => "electron",
    }
}

pub(super) fn bracket_symbol_metrics(
    kind: BracketKind,
    line_width: f64,
) -> crate::CdxmlSymbolMetrics {
    crate::cdxml_symbol_metrics_for_line_width(bracket_kind_name(kind), line_width)
}

pub(super) fn symbol_orbit_point(anchor: SymbolOrbitAnchor, pointer: Point) -> Point {
    let angle = angle_between(anchor.point, pointer).to_radians();
    let (rx, ry) = match anchor.mode {
        SymbolOrbitMode::Endpoint => (13.0, 13.0),
        SymbolOrbitMode::Label => (13.0, 8.0),
    };
    Point::new(
        anchor.point.x + angle.cos() * rx,
        anchor.point.y + angle.sin() * ry,
    )
}

pub(super) fn round_point(point: Point) -> Point {
    Point::new(crate::round2(point.x), crate::round2(point.y))
}
