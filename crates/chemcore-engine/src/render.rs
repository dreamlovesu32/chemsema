use crate::{
    legacy_mol::{parse_molblock, LegacyAtom, LegacyBond as LegacyMolBond, LegacyMol},
    Bond, BondLinePattern, BondLineWeight, ChemcoreDocument, DoubleBondPlacement, LabelRun, Node,
    ObjectPayload, Point, ResourceData, SceneObject, Vector, DEFAULT_BOND_LENGTH,
    DEFAULT_BOND_STROKE, EPSILON,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};

const VIEWER_BOND_STROKE: f64 = 0.85;
const DOUBLE_BOND_OFFSET: f64 = 2.85;
const TRIPLE_BOND_OFFSET: f64 = 2.9;
const DOUBLE_BOND_SIDE_INSET: f64 = 1.4;
const DOUBLE_BOND_SIDE_INSET_RATIO: f64 = 0.14;
const HASH_WEDGE_SPACING: f64 = 3.2;
const HASH_WEDGE_START_OFFSET: f64 = 1.95;
const HASH_WEDGE_END_INSET: f64 = 0.18;
const HASH_BLACK_SEGMENT_LENGTH: f64 = 0.5;
const HASH_TARGET_GAP_LENGTH: f64 = 0.65;
const HASH_WEDGE_EDGE_OVERDRAW: f64 = 0.28;
const HASH_MULTI_BOND_RETREAT_GAP: f64 = 0.45;
const SOLID_WEDGE_END_INSET: f64 = 0.55;
const SOLID_WEDGE_HALF_WIDTH: f64 = 1.8;
const CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES: f64 = 162.0;
const CHEMCORE_INK: &str = "#000000";
const KNOCKOUT_FILL: &str = "#ffffff";
const DASHED_BOND_PATTERN: [f64; 2] = [3.2, 2.4];
const BOLD_BOND_WIDTH_FACTOR: f64 = 1.55;
const BOLD_BOND_MIN_EXTRA_WIDTH: f64 = 1.0;
const MAIN_CONTACT_MITER_LIMIT: f64 = 4.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderRole {
    DocumentBond,
    DocumentGraphic,
    DocumentKnockout,
    DocumentText,
    HoverEndpoint,
    HoverBondCenter,
    PreviewBond,
    PreviewEnd,
    SelectionBond,
    SelectionNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RenderPrimitive {
    Line {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        from: Point,
        to: Point,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
    },
    Circle {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        center: Point,
        radius: f64,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Polygon {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        points: Vec<Point>,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Rect {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rx: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ry: Option<f64>,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(rename = "fillGradient", default, skip_serializing_if = "Option::is_none")]
        fill_gradient: Option<JsonValue>,
    },
    Polyline {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        points: Vec<Point>,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(rename = "lineCap", default, skip_serializing_if = "Option::is_none")]
        line_cap: Option<String>,
        #[serde(rename = "lineJoin", default, skip_serializing_if = "Option::is_none")]
        line_join: Option<String>,
    },
    Text {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        x: f64,
        y: f64,
        text: String,
        #[serde(rename = "fontSize")]
        font_size: f64,
        #[serde(rename = "fontFamily", default, skip_serializing_if = "Option::is_none")]
        font_family: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(rename = "textAnchor", default, skip_serializing_if = "Option::is_none")]
        text_anchor: Option<String>,
        #[serde(rename = "lineHeight", default, skip_serializing_if = "Option::is_none")]
        line_height: Option<f64>,
        #[serde(rename = "preserveLines", default)]
        preserve_lines: bool,
        #[serde(rename = "boxWidth", default, skip_serializing_if = "Option::is_none")]
        box_width: Option<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        runs: Vec<crate::LabelRun>,
    },
}

#[derive(Debug, Clone, Copy)]
struct RectBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl RectBox {
    fn expanded(self, margin: f64) -> Self {
        Self {
            x1: self.x1 - margin,
            y1: self.y1 - margin,
            x2: self.x2 + margin,
            y2: self.y2 + margin,
        }
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.x1 && point.x <= self.x2 && point.y >= self.y1 && point.y <= self.y2
    }
}

#[derive(Debug, Clone, Copy)]
struct LineGeometry {
    point: Point,
    direction: Vector,
    shared: Point,
    length: f64,
    offset_distance: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ArrowHeadGeometry {
    length: f64,
    width: f64,
    head_full: bool,
}

#[derive(Debug, Default)]
struct MainBondContactKernel {
    active_endpoints: BTreeSet<MainBondEndpointKey>,
    endpoint_profiles: BTreeMap<MainBondEndpointKey, Vec<Point>>,
    endpoint_retreats: BTreeMap<MainBondEndpointKey, f64>,
    patches: Vec<MainBondContactPatch>,
}

impl MainBondContactKernel {
    fn uses_endpoint(&self, bond_id: &str, node_id: &str) -> bool {
        self.active_endpoints
            .contains(&MainBondEndpointKey::new(bond_id, node_id))
    }

    fn endpoint_profile(&self, bond_id: &str, node_id: &str) -> Option<Vec<Point>> {
        self.endpoint_profiles
            .get(&MainBondEndpointKey::new(bond_id, node_id))
            .cloned()
    }

    fn endpoint_retreat(&self, bond_id: &str, node_id: &str) -> f64 {
        self.endpoint_retreats
            .get(&MainBondEndpointKey::new(bond_id, node_id))
            .copied()
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MainBondEndpointKey {
    bond_id: String,
    node_id: String,
}

impl MainBondEndpointKey {
    fn new(bond_id: &str, node_id: &str) -> Self {
        Self {
            bond_id: bond_id.to_string(),
            node_id: node_id.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct MainBondContactPatch {
    points: Vec<Point>,
}

#[derive(Debug, Clone, Copy)]
struct MainContourJoin {
    first: Point,
    second: Point,
}

impl MainContourJoin {
    fn midpoint(self) -> Point {
        midpoint(self.first, self.second)
    }
}

#[derive(Debug, Clone, Copy)]
struct MainBondContour {
    base: Point,
    direction: Vector,
    extent: f64,
    half_width: f64,
}

#[derive(Debug, Clone, Copy)]
struct MainBondEndpointGeometry<'a> {
    bond: &'a Bond,
    center: Point,
    axis: Vector,
    base_plus: Point,
    base_minus: Point,
    contour_plus: MainBondContour,
    contour_minus: MainBondContour,
}

#[derive(Debug, Clone)]
struct TwoBondMainContact {
    first_inner_side: f64,
    second_inner_side: f64,
    first_inner: Point,
    first_outer: Point,
    second_inner: Point,
    second_outer: Point,
    bridge: Option<Vec<Point>>,
}

impl MainBondEndpointGeometry<'_> {
    fn base_for_side(self, side: f64) -> Point {
        if side >= 0.0 {
            self.base_plus
        } else {
            self.base_minus
        }
    }

    fn contour_for_side(self, side: f64) -> MainBondContour {
        if side >= 0.0 {
            self.contour_plus
        } else {
            self.contour_minus
        }
    }
}

pub fn render_document(document: &ChemcoreDocument) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let mut objects: Vec<&SceneObject> = document.objects.iter().filter(|object| object.visible).collect();
    objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));

    for object in objects {
        match object.object_type.as_str() {
            "molecule" => render_molecule_object(&mut out, document, object),
            "line" => render_line_object(&mut out, document, object),
            "text" => render_text_object(&mut out, document, object),
            "shape" => render_shape_object(&mut out, document, object),
            _ => {}
        }
    }

    out
}

fn build_main_bond_contact_kernel<'a>(
    document: &ChemcoreDocument,
    object: &SceneObject,
    bonds: &'a [Bond],
    node_map: &BTreeMap<&'a str, &'a Node>,
) -> MainBondContactKernel {
    let mut kernel = MainBondContactKernel::default();

    for node in node_map.values() {
        if node
            .label
            .as_ref()
            .is_some_and(|label| label.has_visible_text())
        {
            continue;
        }

        let incident_bonds: Vec<&Bond> = bonds
            .iter()
            .filter(|bond| bond.begin == node.id || bond.end == node.id)
            .collect();
        if incident_bonds.len() < 2 {
            continue;
        }

        let has_hash_obstacle = incident_bonds
            .iter()
            .any(|bond| is_hash_contact_obstacle(bond));
        if has_hash_obstacle {
            if incident_bonds.len() > 2 {
                for bond in &incident_bonds {
                    if is_hash_contact_obstacle(bond) {
                        continue;
                    }
                    let endpoint_key = MainBondEndpointKey::new(&bond.id, &node.id);
                    let retreat = HASH_MULTI_BOND_RETREAT_GAP
                        * (bond_stroke_width(document, object, bond) / VIEWER_BOND_STROKE);
                    kernel
                        .endpoint_retreats
                        .entry(endpoint_key)
                        .and_modify(|current| *current = current.max(retreat))
                        .or_insert(retreat);
                }
            }
            continue;
        }

        let mut geometries = Vec::with_capacity(incident_bonds.len());
        for bond in incident_bonds {
            let stroke_width = bond_stroke_width(document, object, bond);
            if let Some(geometry) =
                main_bond_endpoint_geometry(object, node_map, bond, node.id.as_str(), stroke_width)
            {
                geometries.push(geometry);
            }
        }
        if geometries.len() < 2 {
            continue;
        }

        for geometry in &geometries {
            kernel
                .active_endpoints
                .insert(MainBondEndpointKey::new(&geometry.bond.id, &node.id));
        }

        match geometries.len() {
            2 => {
                if let Some(contact) = two_bond_main_contact(geometries[0], geometries[1]) {
                    for (geometry, inner_side, inner_point, outer_point) in [
                        (
                            geometries[0],
                            contact.first_inner_side,
                            contact.first_inner,
                            contact.first_outer,
                        ),
                        (
                            geometries[1],
                            contact.second_inner_side,
                            contact.second_inner,
                            contact.second_outer,
                        ),
                    ] {
                        let endpoint_key = MainBondEndpointKey::new(&geometry.bond.id, &node.id);
                        if supports_main_bond_polygon_endpoint_cap(geometry.bond) {
                            kernel.endpoint_profiles.insert(
                                endpoint_key,
                                main_bond_endpoint_profile(inner_side, inner_point, outer_point),
                            );
                        } else {
                            push_two_bond_contact_triangles(
                                &mut kernel.patches,
                                geometry,
                                inner_side,
                                inner_point,
                                outer_point,
                            );
                        }
                    }
                    if let Some(points) = contact.bridge {
                        kernel.patches.push(MainBondContactPatch { points });
                    }
                }
            }
            _ => append_multi_bond_main_contact(&mut kernel, &geometries, &node.id),
        }
    }

    kernel
}

fn render_main_bond_contact_patches(
    out: &mut Vec<RenderPrimitive>,
    kernel: &MainBondContactKernel,
    stroke: &str,
    object_id: Option<String>,
) {
    for patch in &kernel.patches {
        push_polygon(
            out,
            patch.points.clone(),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentBond,
            object_id.clone(),
        );
    }
}

fn main_bond_endpoint_geometry<'a>(
    object: &SceneObject,
    node_map: &BTreeMap<&'a str, &'a Node>,
    bond: &'a Bond,
    node_id: &'a str,
    stroke_width: f64,
) -> Option<MainBondEndpointGeometry<'a>> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let (center, other) = if node_id == bond.begin {
        (begin, end)
    } else if node_id == bond.end {
        (end, begin)
    } else {
        return None;
    };
    let forward = Vector::new(other.x - center.x, other.y - center.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let axis = forward.normalized();
    let normal = Vector::new(-axis.y, axis.x);

    if let Some(stereo_kind) = bond_stereo_kind(bond) {
        let narrow_half_width = solid_wedge_tip_half_width(stroke_width);
        let wide_half_width = solid_wedge_half_width(stroke_width);
        return match stereo_kind {
            BondStereoKind::SolidWedgeEnd | BondStereoKind::HashedWedgeEnd if node_id == bond.end => {
                let base_plus =
                    Point::new(center.x + normal.x * wide_half_width, center.y + normal.y * wide_half_width);
                let base_minus =
                    Point::new(center.x - normal.x * wide_half_width, center.y - normal.y * wide_half_width);
                let tip_plus =
                    Point::new(other.x + normal.x * narrow_half_width, other.y + normal.y * narrow_half_width);
                let tip_minus =
                    Point::new(other.x - normal.x * narrow_half_width, other.y - normal.y * narrow_half_width);
                let plus_direction = Vector::new(tip_plus.x - base_plus.x, tip_plus.y - base_plus.y);
                let minus_direction = Vector::new(tip_minus.x - base_minus.x, tip_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: wide_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: wide_half_width,
                    },
                })
            }
            BondStereoKind::SolidWedgeBegin | BondStereoKind::HashedWedgeBegin if node_id == bond.begin => {
                let base_plus =
                    Point::new(center.x + normal.x * wide_half_width, center.y + normal.y * wide_half_width);
                let base_minus =
                    Point::new(center.x - normal.x * wide_half_width, center.y - normal.y * wide_half_width);
                let tip_plus =
                    Point::new(other.x + normal.x * narrow_half_width, other.y + normal.y * narrow_half_width);
                let tip_minus =
                    Point::new(other.x - normal.x * narrow_half_width, other.y - normal.y * narrow_half_width);
                let plus_direction = Vector::new(tip_plus.x - base_plus.x, tip_plus.y - base_plus.y);
                let minus_direction = Vector::new(tip_minus.x - base_minus.x, tip_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: wide_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: wide_half_width,
                    },
                })
            }
            BondStereoKind::SolidWedgeEnd
            | BondStereoKind::SolidWedgeBegin
            | BondStereoKind::HashedWedgeEnd
            | BondStereoKind::HashedWedgeBegin => {
                let base_plus =
                    Point::new(center.x + normal.x * narrow_half_width, center.y + normal.y * narrow_half_width);
                let base_minus =
                    Point::new(center.x - normal.x * narrow_half_width, center.y - normal.y * narrow_half_width);
                let cap_plus =
                    Point::new(other.x + normal.x * wide_half_width, other.y + normal.y * wide_half_width);
                let cap_minus =
                    Point::new(other.x - normal.x * wide_half_width, other.y - normal.y * wide_half_width);
                let plus_direction = Vector::new(cap_plus.x - base_plus.x, cap_plus.y - base_plus.y);
                let minus_direction = Vector::new(cap_minus.x - base_minus.x, cap_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: narrow_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: narrow_half_width,
                    },
                })
            }
        };
    }

    if !supports_main_bond_contact_shape(bond) {
        return None;
    }

    let half_width = line_weight_stroke_width(stroke_width, bond.line_weights.main) * 0.5;
    let base_plus = Point::new(center.x + normal.x * half_width, center.y + normal.y * half_width);
    let base_minus = Point::new(center.x - normal.x * half_width, center.y - normal.y * half_width);
    Some(MainBondEndpointGeometry {
        bond,
        center,
        axis,
        base_plus,
        base_minus,
        contour_plus: MainBondContour {
            base: base_plus,
            direction: axis,
            extent: length,
            half_width,
        },
        contour_minus: MainBondContour {
            base: base_minus,
            direction: axis,
            extent: length,
            half_width,
        },
    })
}

fn supports_main_bond_contact_shape(bond: &Bond) -> bool {
    if bond.stereo.is_some() {
        return false;
    }
    if bond.order == 1 {
        return bond.line_styles.left == BondLinePattern::Solid
            && bond.line_styles.right == BondLinePattern::Solid
            && bond.line_weights.left == BondLineWeight::Normal
            && bond.line_weights.right == BondLineWeight::Normal
            && matches!(
                (bond.line_styles.main, bond.line_weights.main),
                (BondLinePattern::Solid, BondLineWeight::Normal)
                    | (BondLinePattern::Dashed, BondLineWeight::Normal)
                    | (BondLinePattern::Solid, BondLineWeight::Bold)
                    | (BondLinePattern::Dashed, BondLineWeight::Bold)
            );
    }
    if bond.order == 2 {
        return side_double_placement(bond).is_some()
            && bond.line_styles.main == BondLinePattern::Solid
            && matches!(
                bond.line_weights.main,
                BondLineWeight::Normal | BondLineWeight::Bold
            );
    }
    if bond.order == 3 {
        return bond.double.is_none()
            && bond.line_styles.main == BondLinePattern::Solid
            && bond.line_styles.left == BondLinePattern::Solid
            && bond.line_styles.right == BondLinePattern::Solid
            && bond.line_weights.main == BondLineWeight::Normal
            && bond.line_weights.left == BondLineWeight::Normal
            && bond.line_weights.right == BondLineWeight::Normal;
    }
    false
}

fn supports_main_bond_polygon_endpoint_cap(bond: &Bond) -> bool {
    if let Some(stereo_kind) = bond_stereo_kind(bond) {
        return matches!(
            stereo_kind,
            BondStereoKind::SolidWedgeBegin
                | BondStereoKind::SolidWedgeEnd
                | BondStereoKind::HashedWedgeBegin
                | BondStereoKind::HashedWedgeEnd
        );
    }
    supports_main_bond_contact_shape(bond)
}

fn two_bond_main_contact(
    first: MainBondEndpointGeometry<'_>,
    second: MainBondEndpointGeometry<'_>,
) -> Option<TwoBondMainContact> {
    if main_contact_is_straight_through(first.axis, second.axis) {
        return None;
    }

    let Some(first_inner_side) = main_contact_side(first.axis, second.axis) else {
        return None;
    };
    let Some(second_inner_side) = main_contact_side(second.axis, first.axis) else {
        return None;
    };

    let inner = bounded_main_contour_intersection(
        first.contour_for_side(first_inner_side),
        second.contour_for_side(second_inner_side),
    );
    let outer = bounded_main_contour_intersection(
        first.contour_for_side(-first_inner_side),
        second.contour_for_side(-second_inner_side),
    );

    let bridge = compact_polygon_points(vec![inner.first, outer.first, outer.second, inner.second]);
    let bridge = if bridge.len() >= 3 && polygon_area_signed(&bridge).abs() > 1.0e-4 {
        Some(bridge)
    } else {
        None
    };

    Some(TwoBondMainContact {
        first_inner_side,
        second_inner_side,
        first_inner: inner.first,
        first_outer: outer.first,
        second_inner: inner.second,
        second_outer: outer.second,
        bridge,
    })
}

fn main_bond_endpoint_profile(inner_side: f64, inner: Point, outer: Point) -> Vec<Point> {
    if inner_side >= 0.0 {
        vec![inner, outer]
    } else {
        vec![outer, inner]
    }
}

fn endpoint_profile_global(profile: Option<Vec<Point>>, reverse: bool, default_profile: Vec<Point>) -> Vec<Point> {
    let points = if let Some(mut profile) = profile {
        if reverse {
            profile.reverse();
        }
        profile
    } else {
        default_profile
    };
    compact_polygon_points(points)
}

fn bond_polygon_from_endpoint_profiles(start_profile: Vec<Point>, end_profile: Vec<Point>) -> Vec<Point> {
    let mut points = Vec::with_capacity(start_profile.len() + end_profile.len());
    if let Some(first) = start_profile.first().copied() {
        points.push(first);
    }
    for point in end_profile {
        if points
            .last()
            .is_some_and(|last| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        points.push(point);
    }
    let mut start_tail: Vec<Point> = start_profile.into_iter().skip(1).collect();
    start_tail.reverse();
    for point in start_tail {
        if points
            .last()
            .is_some_and(|last| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        points.push(point);
    }
    compact_polygon_points(points)
}

fn push_two_bond_contact_triangles(
    out: &mut Vec<MainBondContactPatch>,
    geometry: MainBondEndpointGeometry<'_>,
    inner_side: f64,
    inner: Point,
    outer: Point,
) {
    for points in [
        compact_polygon_points(vec![geometry.center, geometry.base_for_side(inner_side), inner]),
        compact_polygon_points(vec![
            geometry.center,
            outer,
            geometry.base_for_side(-inner_side),
        ]),
    ] {
        if points.len() >= 3 && polygon_area_signed(&points).abs() > 1.0e-4 {
            out.push(MainBondContactPatch { points });
        }
    }
}

fn append_multi_bond_main_contact(
    kernel: &mut MainBondContactKernel,
    geometries: &[MainBondEndpointGeometry<'_>],
    node_id: &str,
) {
    if geometries.len() < 3 {
        return;
    }
    let mut ordered = geometries.to_vec();
    ordered.sort_by(|a, b| axis_angle(a.axis).total_cmp(&axis_angle(b.axis)));

    let mut ring_intersections = Vec::with_capacity(ordered.len());
    for index in 0..ordered.len() {
        let current = ordered[index];
        let next = ordered[(index + 1) % ordered.len()];
        let Some(current_side) = main_contact_side(current.axis, next.axis) else {
            return;
        };
        let Some(next_side) = main_contact_side(next.axis, current.axis) else {
            return;
        };
        ring_intersections.push(
            extended_main_contour_intersection(
                current.contour_for_side(current_side),
                next.contour_for_side(next_side),
            )
            .midpoint(),
        );
    }

    for (index, current) in ordered.iter().enumerate() {
        let previous = ordered[(index + ordered.len() - 1) % ordered.len()];
        let next = ordered[(index + 1) % ordered.len()];
        let Some(previous_side) = main_contact_side(current.axis, previous.axis) else {
            continue;
        };
        let Some(next_side) = main_contact_side(current.axis, next.axis) else {
            continue;
        };
        let previous_intersection = ring_intersections[(index + ordered.len() - 1) % ordered.len()];
        let next_intersection = ring_intersections[index];

        if supports_main_bond_polygon_endpoint_cap(current.bond) {
            let mut profile = if previous_side > 0.0 && next_side < 0.0 {
                vec![previous_intersection, current.center, next_intersection]
            } else if next_side > 0.0 && previous_side < 0.0 {
                vec![next_intersection, current.center, previous_intersection]
            } else if previous_side >= next_side {
                vec![previous_intersection, current.center, next_intersection]
            } else {
                vec![next_intersection, current.center, previous_intersection]
            };
            profile = compact_polygon_points(profile);
            if profile.len() >= 3 && polygon_area_signed(&profile).abs() > 1.0e-4 {
                kernel.endpoint_profiles.insert(
                    MainBondEndpointKey::new(&current.bond.id, node_id),
                    profile,
                );
            }
            continue;
        }

        let points = compact_polygon_points(vec![current.center, previous_intersection, next_intersection]);
        if points.len() >= 3 && polygon_area_signed(&points).abs() > 1.0e-4 {
            kernel.patches.push(MainBondContactPatch { points });
        }
    }
}

fn main_contact_side(axis: Vector, other_axis: Vector) -> Option<f64> {
    let normal = Vector::new(-axis.y, axis.x);
    let side_value = normal.x * other_axis.x + normal.y * other_axis.y;
    let side = side_value.signum();
    if side.abs() <= EPSILON {
        None
    } else {
        Some(side)
    }
}

fn main_contact_is_straight_through(axis: Vector, other_axis: Vector) -> bool {
    vector_cross(axis, other_axis).abs() <= 1.0e-6 && vector_dot(axis, other_axis) < 0.0
}

fn bond_ray_angle_degrees(axis: Vector, other_axis: Vector) -> f64 {
    vector_dot(axis.normalized(), other_axis.normalized())
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

fn center_double_skips_extension(axis: Vector, other_axis: Vector) -> bool {
    bond_ray_angle_degrees(axis, other_axis) > CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES
}

fn bond_ray_is_acute(axis: Vector, other_axis: Vector) -> bool {
    vector_dot(axis.normalized(), other_axis.normalized()) > EPSILON
}

fn extended_main_contour_intersection(first: MainBondContour, second: MainBondContour) -> MainContourJoin {
    if let Some((intersection, _, _)) =
        line_intersection_with_parameters(first.base, first.direction, second.base, second.direction)
    {
        return MainContourJoin {
            first: intersection,
            second: intersection,
        };
    }
    bounded_main_contour_intersection(first, second)
}

fn bounded_main_contour_intersection(first: MainBondContour, second: MainBondContour) -> MainContourJoin {
    let max_distance = (first.half_width.max(second.half_width) * MAIN_CONTACT_MITER_LIMIT)
        .min(first.extent.min(second.extent) * 0.38);
    let fallback_distance = max_distance.min(
        (first.extent.min(second.extent) * 0.18).max(first.half_width.max(second.half_width) * 2.5),
    );
    let fallback_first = point_on_main_contour(first, fallback_distance);
    let fallback_second = point_on_main_contour(second, fallback_distance);

    let Some((intersection, _, _)) = line_intersection_with_parameters(
        first.base,
        first.direction,
        second.base,
        second.direction,
    ) else {
        return MainContourJoin {
            first: fallback_first,
            second: fallback_second,
        };
    };

    let first_unit = first.direction.normalized();
    let second_unit = second.direction.normalized();
    let first_projection =
        vector_dot(Vector::new(intersection.x - first.base.x, intersection.y - first.base.y), first_unit);
    let second_projection = vector_dot(
        Vector::new(intersection.x - second.base.x, intersection.y - second.base.y),
        second_unit,
    );
    if first_projection.abs() <= max_distance && second_projection.abs() <= max_distance {
        return MainContourJoin {
            first: intersection,
            second: intersection,
        };
    }

    let clamped_first =
        point_on_main_contour(first, first_projection.clamp(-max_distance, max_distance));
    let clamped_second =
        point_on_main_contour(second, second_projection.clamp(-max_distance, max_distance));
    MainContourJoin {
        first: clamped_first,
        second: clamped_second,
    }
}

fn point_on_main_contour(contour: MainBondContour, distance: f64) -> Point {
    let length = contour.direction.length();
    if length <= EPSILON {
        return contour.base;
    }
    let unit = contour.direction.normalized();
    Point::new(
        contour.base.x + unit.x * distance,
        contour.base.y + unit.y * distance,
    )
}

fn midpoint(first: Point, second: Point) -> Point {
    Point::new((first.x + second.x) * 0.5, (first.y + second.y) * 0.5)
}

fn compact_polygon_points(points: Vec<Point>) -> Vec<Point> {
    let mut out = Vec::new();
    for point in points {
        if out
            .last()
            .is_some_and(|last: &Point| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        out.push(point);
    }
    if out.len() >= 2
        && out
            .first()
            .zip(out.last())
            .is_some_and(|(first, last)| first.distance(*last) <= 1.0e-6)
    {
        out.pop();
    }
    out
}

fn polygon_area_signed(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area * 0.5
}

fn axis_angle(axis: Vector) -> f64 {
    axis.y.atan2(axis.x)
}

fn vector_dot(first: Vector, second: Vector) -> f64 {
    first.x * second.x + first.y * second.y
}

fn vector_cross(first: Vector, second: Vector) -> f64 {
    first.x * second.y - first.y * second.x
}

fn render_molecule_object(out: &mut Vec<RenderPrimitive>, document: &ChemcoreDocument, object: &SceneObject) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemcore.molecule.fragment2d" =>
        {
            let node_map: BTreeMap<&str, &Node> = fragment
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect();
            let stroke = molecule_stroke(document, object);
            let object_id = Some(object.id.clone());
            let contact_kernel =
                build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);

            for bond in &fragment.bonds {
                render_fragment_bond(
                    out,
                    document,
                    object,
                    &contact_kernel,
                    &fragment.bonds,
                    &node_map,
                    bond,
                    &stroke,
                    object_id.clone(),
                );
            }
            render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());

            for node in &fragment.nodes {
                render_fragment_label(out, document, object, node, object_id.clone());
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

fn render_line_object(out: &mut Vec<RenderPrimitive>, document: &ChemcoreDocument, object: &SceneObject) {
    let points = payload_points(&object.payload, "points");
    if points.len() < 2 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#222222".to_string());
    let stroke_width = style
        .and_then(|value| style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width")))
        .unwrap_or(1.6);
    let line_cap = style
        .and_then(|value| style_string(value, "lineCap"))
        .unwrap_or_else(|| "round".to_string());
    let line_join = style
        .and_then(|value| style_string(value, "lineJoin"))
        .unwrap_or_else(|| "round".to_string());
    let mut shaft_points = points.clone();
    let object_id = Some(object.id.clone());

    if payload_string(&object.payload, "head").as_deref() == Some("end") {
        if let Some(arrow_head) = payload_arrow_head(&object.payload, "arrowHead") {
            if arrow_head.length > 0.0 && shaft_points.len() >= 2 {
                let from = shaft_points[shaft_points.len() - 2];
                let to = shaft_points[shaft_points.len() - 1];
                let shaft_end = arrow_shaft_end(from, to, arrow_head);
                if let Some(last) = shaft_points.last_mut() {
                    *last = shaft_end;
                }
                push_polyline(
                    out,
                    shaft_points,
                    &stroke,
                    stroke_width,
                    Vec::new(),
                    Some(line_cap.clone()),
                    Some(line_join.clone()),
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
                push_polygon(
                    out,
                    arrow_head_points(from, to, arrow_head),
                    &stroke,
                    &stroke,
                    stroke_width,
                    RenderRole::DocumentGraphic,
                    object_id,
                );
                return;
            }
        }
    }

    push_polyline(
        out,
        shaft_points,
        &stroke,
        stroke_width,
        Vec::new(),
        Some(line_cap),
        Some(line_join),
        RenderRole::DocumentGraphic,
        object_id,
    );
}

fn render_text_object(out: &mut Vec<RenderPrimitive>, document: &ChemcoreDocument, object: &SceneObject) {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let font_size = payload_number(&object.payload, "fontSize")
        .or_else(|| style.and_then(|value| style_number(value, "fontSize").or_else(|| style_number(value, "font_size"))))
        .unwrap_or(12.0);
    let line_height = payload_number(&object.payload, "lineHeight").unwrap_or(15.0);
    let align = payload_string(&object.payload, "align").unwrap_or_else(|| "left".to_string());
    let text_anchor = text_anchor(&align);
    let font_family = style.and_then(|value| style_string(value, "fontFamily"));
    let fill = style.and_then(|value| style_string(value, "fill"));
    let object_id = Some(object.id.clone());

    if payload_bool(&object.payload, "preserveLines").unwrap_or(false) {
        let runs = payload_runs(&object.payload, "runs");
        if !runs.is_empty() {
            for (index, line_runs) in split_runs_by_line(&runs).into_iter().enumerate() {
                if line_runs.is_empty() {
                    continue;
                }
                push_text(
                    out,
                    tx,
                    ty + font_size * 0.82 + index as f64 * line_height,
                    String::new(),
                    font_size,
                    font_family.clone(),
                    fill.clone(),
                    Some(text_anchor.clone()),
                    line_runs,
                    object_id.clone(),
                );
            }
            return;
        }
        for (index, line) in split_preserved_text_lines(&payload_string(&object.payload, "text").unwrap_or_default())
            .into_iter()
            .enumerate()
        {
            push_text(
                out,
                tx,
                ty + font_size * 0.82 + index as f64 * line_height,
                line,
                font_size,
                font_family.clone(),
                fill.clone(),
                Some(text_anchor.clone()),
                Vec::new(),
                object_id.clone(),
            );
        }
        return;
    }

    let box_width = payload_box_width(&object.payload, "box").unwrap_or(160.0);
    for (index, line) in wrap_text_lines(
        &payload_string(&object.payload, "text").unwrap_or_default(),
        box_width,
        font_size,
    )
    .into_iter()
    .enumerate()
    {
        push_text(
            out,
            tx,
            ty + font_size * 0.82 + index as f64 * line_height,
            line,
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            Vec::new(),
            object_id.clone(),
        );
    }
}

fn render_shape_object(out: &mut Vec<RenderPrimitive>, document: &ChemcoreDocument, object: &SceneObject) {
    let [tx, ty] = object.transform.translate;
    let Some([_, _, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let fill = style.and_then(|value| style_nullable_string(value, "fill"));
    let stroke = style.and_then(|value| style_nullable_string(value, "stroke"));
    let stroke_width = style
        .and_then(|value| style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width")))
        .unwrap_or(1.0);
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let fill_gradient = style.and_then(|value| value.get("fillGradient").cloned()).filter(|value| !value.is_null());
    let corner_radius = payload_number(&object.payload, "cornerRadius").filter(|value| *value > 0.0);

    out.push(RenderPrimitive::Rect {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        x: tx,
        y: ty,
        width,
        height,
        fill,
        stroke,
        stroke_width,
        rx: corner_radius,
        ry: corner_radius,
        dash_array,
        fill_gradient,
    });
}

#[derive(Debug, Clone)]
struct LegacyLabelMetrics {
    visible: bool,
    pad: f64,
    label: String,
}

#[derive(Debug, Clone)]
struct LegacyCollapsedGroup {
    label: String,
    centroid: Point,
    connections: Vec<usize>,
}

fn render_legacy_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    molblock: &str,
) {
    let Some(parsed) = parse_molblock(molblock) else {
        return;
    };
    let bbox = object
        .payload
        .bbox
        .unwrap_or([0.0, 0.0, (parsed.max_x - parsed.min_x).max(1.0), (parsed.max_y - parsed.min_y).max(1.0)]);
    let atom_points: Vec<Point> = parsed
        .atoms
        .iter()
        .map(|atom| legacy_map_atom_point(&parsed, atom, bbox[2], bbox[3], object))
        .collect();
    let (collapsed_groups, hidden_atoms, hidden_bonds) =
        legacy_build_collapsed_groups(&parsed, &atom_points);
    let label_metrics: Vec<LegacyLabelMetrics> = parsed
        .atoms
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if hidden_atoms.contains(&index) {
                LegacyLabelMetrics {
                    visible: false,
                    pad: 0.0,
                    label: String::new(),
                }
            } else {
                legacy_label_metrics(&parsed, index)
            }
        })
        .collect();
    let stroke = molecule_stroke(document, object);
    let stroke_width = legacy_bond_stroke_width(document, object);
    let font_size = legacy_label_font_size(document, object);
    let font_family = legacy_label_font_family(document, object);
    let object_id = Some(object.id.clone());

    for (bond_index, bond) in parsed.bonds.iter().enumerate() {
        if hidden_bonds.contains(&bond_index) {
            continue;
        }
        render_legacy_bond(
            out,
            &parsed,
            bond,
            &atom_points,
            &hidden_atoms,
            &label_metrics,
            &stroke,
            stroke_width,
            object_id.clone(),
        );
    }

    for group in &collapsed_groups {
        for &outside_atom in &group.connections {
            if let Some(from) = atom_points.get(outside_atom).copied() {
                push_line(
                    out,
                    from,
                    group.centroid,
                    &stroke,
                    stroke_width,
                    Vec::new(),
                    RenderRole::DocumentBond,
                    object_id.clone(),
                );
            }
        }
        push_text(
            out,
            group.centroid.x,
            group.centroid.y,
            group.label.clone(),
            font_size,
            Some(font_family.clone()),
            Some(stroke.clone()),
            None,
            Vec::new(),
            object_id.clone(),
        );
    }

    for (index, atom) in parsed.atoms.iter().enumerate() {
        if hidden_atoms.contains(&index) {
            continue;
        }
        let metrics = &label_metrics[index];
        if !metrics.visible {
            continue;
        }
        let point = atom_points[index];
        let offset = legacy_atom_label_offset(&parsed, index);
        push_text(
            out,
            point.x + offset.x,
            point.y + offset.y,
            metrics.label.clone(),
            font_size,
            Some(font_family.clone()),
            Some(stroke.clone()),
            None,
            Vec::new(),
            object_id.clone(),
        );
        let _ = atom;
    }
}

fn legacy_map_atom_point(
    parsed: &LegacyMol,
    atom: &LegacyAtom,
    bbox_width: f64,
    bbox_height: f64,
    object: &SceneObject,
) -> Point {
    let width = (parsed.max_x - parsed.min_x).max(1.0);
    let height = (parsed.max_y - parsed.min_y).max(1.0);
    let scale = (bbox_width / width).min(bbox_height / height);
    let offset_x = (bbox_width - width * scale) / 2.0;
    let offset_y = (bbox_height - height * scale) / 2.0;
    Point::new(
        object.transform.translate[0] + (atom.x - parsed.min_x) * scale + offset_x,
        object.transform.translate[1] + (parsed.max_y - atom.y) * scale + offset_y,
    )
}

fn legacy_atom_degree(parsed: &LegacyMol, atom_index: usize) -> usize {
    parsed
        .bonds
        .iter()
        .filter(|bond| bond.begin == atom_index || bond.end == atom_index)
        .count()
}

fn legacy_atom_needs_label(parsed: &LegacyMol, atom_index: usize) -> bool {
    parsed
        .atoms
        .get(atom_index)
        .is_some_and(|atom| atom.symbol != "C" || legacy_atom_degree(parsed, atom_index) == 0)
}

fn legacy_bond_neighbors<'a>(
    parsed: &'a LegacyMol,
    atom_index: usize,
    excluded_atom_index: usize,
) -> Vec<&'a LegacyAtom> {
    parsed
        .bonds
        .iter()
        .filter_map(|bond| {
            if bond.begin == atom_index && bond.end != excluded_atom_index {
                parsed.atoms.get(bond.end)
            } else if bond.end == atom_index && bond.begin != excluded_atom_index {
                parsed.atoms.get(bond.begin)
            } else {
                None
            }
        })
        .collect()
}

fn legacy_format_atom_label(atom: &LegacyAtom) -> String {
    let mut label = atom.symbol.clone();
    match atom.charge {
        1 => label.push('+'),
        value if value > 1 => label.push_str(&format!("{value}+")),
        -1 => label.push('−'),
        value if value < -1 => label.push_str(&format!("{}−", value.abs())),
        _ => {}
    }
    label
}

fn legacy_label_metrics(parsed: &LegacyMol, atom_index: usize) -> LegacyLabelMetrics {
    if !legacy_atom_needs_label(parsed, atom_index) {
        return LegacyLabelMetrics {
            visible: false,
            pad: 0.0,
            label: String::new(),
        };
    }
    let label = legacy_format_atom_label(&parsed.atoms[atom_index]);
    let width = 12.0_f64.max(label.chars().count() as f64 * 6.2 + 8.0);
    LegacyLabelMetrics {
        visible: true,
        pad: width / 2.0 - 2.0,
        label,
    }
}

fn legacy_atom_label_offset(parsed: &LegacyMol, atom_index: usize) -> Vector {
    let Some(atom) = parsed.atoms.get(atom_index) else {
        return Vector::new(0.0, 0.0);
    };
    let neighbors = legacy_bond_neighbors(parsed, atom_index, usize::MAX);
    if neighbors.is_empty() {
        return Vector::new(0.0, 0.0);
    }
    let (vx, vy) = neighbors.iter().fold((0.0, 0.0), |(sum_x, sum_y), neighbor| {
        (sum_x + neighbor.x - atom.x, sum_y + neighbor.y - atom.y)
    });
    let length = vx.hypot(vy).max(1.0);
    Vector::new((-vx / length) * 4.5, (vy / length) * 4.5)
}

fn legacy_line_endpoints_with_label_padding(
    start: Point,
    end: Point,
    start_pad: f64,
    end_pad: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y).normalized();
    (
        Point::new(start.x + direction.x * start_pad, start.y + direction.y * start_pad),
        Point::new(end.x - direction.x * end_pad, end.y - direction.y * end_pad),
    )
}

fn legacy_choose_double_bond_side(parsed: &LegacyMol, bond: &LegacyMolBond) -> f64 {
    let Some(begin) = parsed.atoms.get(bond.begin) else {
        return -1.0;
    };
    let Some(end) = parsed.atoms.get(bond.end) else {
        return -1.0;
    };
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy).max(1.0);
    let normal_x = -dy / length;
    let normal_y = dx / length;

    let mut score = 0.0;
    for neighbor in legacy_bond_neighbors(parsed, bond.begin, bond.end) {
        score += (neighbor.x - begin.x) * normal_x + (neighbor.y - begin.y) * normal_y;
    }
    for neighbor in legacy_bond_neighbors(parsed, bond.end, bond.begin) {
        score += (neighbor.x - end.x) * normal_x + (neighbor.y - end.y) * normal_y;
    }
    if score >= 0.0 { -1.0 } else { 1.0 }
}

fn render_legacy_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    atom_points: &[Point],
    hidden_atoms: &BTreeSet<usize>,
    label_metrics: &[LegacyLabelMetrics],
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    if hidden_atoms.contains(&bond.begin) || hidden_atoms.contains(&bond.end) {
        return;
    }
    let Some(start_point) = atom_points.get(bond.begin).copied() else {
        return;
    };
    let Some(end_point) = atom_points.get(bond.end).copied() else {
        return;
    };
    let start_pad = label_metrics.get(bond.begin).map(|metrics| metrics.pad).unwrap_or(0.0);
    let end_pad = label_metrics.get(bond.end).map(|metrics| metrics.pad).unwrap_or(0.0);

    if bond.stereo == 1 {
        let (start, end) =
            legacy_line_endpoints_with_label_padding(start_point, end_point, start_pad, end_pad);
        let points = compute_solid_wedge_points(
            start,
            end,
            if end_pad > 0.0 { SOLID_WEDGE_END_INSET } else { 0.0 },
            legacy_wide_contact_directions(parsed, bond, bond.end, atom_points, hidden_atoms),
            stroke_width,
        );
        push_polygon(
            out,
            points,
            stroke,
            stroke,
            stroke_width,
            RenderRole::DocumentBond,
            object_id,
        );
        return;
    }
    if bond.stereo == 6 {
        let (start, end) =
            legacy_line_endpoints_with_label_padding(start_point, end_point, start_pad, end_pad);
        for (segment_start, segment_end, segment_width) in
            compute_hashed_wedge_segments(start, end, stroke_width)
        {
            push_line(
                out,
                segment_start,
                segment_end,
                stroke,
                segment_width,
                Vec::new(),
                RenderRole::DocumentBond,
                object_id.clone(),
            );
        }
        return;
    }

    match bond.order {
        2 => render_legacy_double_bond(
            out,
            parsed,
            bond,
            start_point,
            end_point,
            start_pad > 0.0,
            end_pad > 0.0,
            stroke,
            stroke_width,
            object_id,
        ),
        3.. => render_legacy_triple_bond(
            out,
            parsed,
            bond,
            start_point,
            end_point,
            start_pad > 0.0,
            end_pad > 0.0,
            stroke,
            stroke_width,
            object_id,
        ),
        _ => push_line(
            out,
            start_point,
            end_point,
            stroke,
            stroke_width,
            Vec::new(),
            RenderRole::DocumentBond,
            object_id,
        ),
    }
}

fn render_legacy_double_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    start: Point,
    end: Point,
    align_start: bool,
    align_end: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let degree_a = legacy_atom_degree(parsed, bond.begin);
    let degree_b = legacy_atom_degree(parsed, bond.end);
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let double_offset = double_bond_offset_distance(start, end, stroke_width);
    if degree_a == 1 || degree_b == 1 {
        let (normal_x, normal_y) = unit_normal(start, end);
        for offset in [-double_offset / 2.0, double_offset / 2.0] {
            push_line(
                out,
                Point::new(start.x + normal_x * offset, start.y + normal_y * offset),
                Point::new(end.x + normal_x * offset, end.y + normal_y * offset),
                stroke,
                stroke_width,
                Vec::new(),
                RenderRole::DocumentBond,
                object_id.clone(),
            );
        }
        return;
    }

    let side = legacy_choose_double_bond_side(parsed, bond);
    let (normal_x, normal_y) = unit_normal(start, end);
    let length = start.distance(end);
    let side_inset = (DOUBLE_BOND_SIDE_INSET * scale).max(length * DOUBLE_BOND_SIDE_INSET_RATIO);
    push_line(
        out,
        start,
        end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id.clone(),
    );
    let offset_start = Point::new(
        start.x + normal_x * double_offset * side,
        start.y + normal_y * double_offset * side,
    );
    let offset_end = Point::new(
        end.x + normal_x * double_offset * side,
        end.y + normal_y * double_offset * side,
    );
    let (short_start, short_end) = inset_bond_segment(
        offset_start,
        offset_end,
        if align_start { 0.0 } else { side_inset },
        if align_end { 0.0 } else { side_inset },
    );
    push_line(
        out,
        short_start,
        short_end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id,
    );
}

fn render_legacy_triple_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    start: Point,
    end: Point,
    align_start: bool,
    align_end: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let degree_a = legacy_atom_degree(parsed, bond.begin);
    let degree_b = legacy_atom_degree(parsed, bond.end);
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let triple_offset = triple_bond_offset_distance(start, end, stroke_width);
    let length = start.distance(end);
    let side_inset = (DOUBLE_BOND_SIDE_INSET * scale).max(length * DOUBLE_BOND_SIDE_INSET_RATIO);
    let (normal_x, normal_y) = unit_normal(start, end);

    push_line(
        out,
        start,
        end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id.clone(),
    );

    for side in [1.0, -1.0] {
        let offset_start = Point::new(
            start.x + normal_x * triple_offset * side,
            start.y + normal_y * triple_offset * side,
        );
        let offset_end = Point::new(
            end.x + normal_x * triple_offset * side,
            end.y + normal_y * triple_offset * side,
        );
        let (short_start, short_end) = inset_bond_segment(
            offset_start,
            offset_end,
            if degree_a == 1 || align_start { 0.0 } else { side_inset },
            if degree_b == 1 || align_end { 0.0 } else { side_inset },
        );
        push_line(
            out,
            short_start,
            short_end,
            stroke,
            stroke_width,
            Vec::new(),
            RenderRole::DocumentBond,
            object_id.clone(),
        );
    }
}

fn legacy_has_visible_atom_label(
    parsed: &LegacyMol,
    atom_index: usize,
    hidden_atoms: &BTreeSet<usize>,
) -> bool {
    !hidden_atoms.contains(&atom_index) && legacy_atom_needs_label(parsed, atom_index)
}

fn legacy_is_side_double(parsed: &LegacyMol, bond: &LegacyMolBond) -> bool {
    bond.order == 2
        && legacy_atom_degree(parsed, bond.begin) > 1
        && legacy_atom_degree(parsed, bond.end) > 1
}

fn legacy_is_wide_contact_candidate(parsed: &LegacyMol, bond: &LegacyMolBond) -> bool {
    (bond.order == 1 && bond.stereo == 0) || legacy_is_side_double(parsed, bond)
}

fn legacy_wide_contact_directions(
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    wide_atom_index: usize,
    atom_points: &[Point],
    hidden_atoms: &BTreeSet<usize>,
) -> Vec<Vector> {
    if legacy_has_visible_atom_label(parsed, wide_atom_index, hidden_atoms) {
        return Vec::new();
    }
    let Some(wide_point) = atom_points.get(wide_atom_index).copied() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for other_bond in &parsed.bonds {
        if std::ptr::eq(other_bond, bond) || !legacy_is_wide_contact_candidate(parsed, other_bond) {
            continue;
        }
        if other_bond.begin != wide_atom_index && other_bond.end != wide_atom_index {
            continue;
        }
        let other_atom_index = if other_bond.begin == wide_atom_index {
            other_bond.end
        } else {
            other_bond.begin
        };
        if hidden_atoms.contains(&other_atom_index) {
            continue;
        }
        if let Some(other_point) = atom_points.get(other_atom_index).copied() {
            let vector = Vector::new(other_point.x - wide_point.x, other_point.y - wide_point.y);
            if vector.length() > 1.0e-6 {
                out.push(vector);
            }
        }
    }
    out
}

fn legacy_build_collapsed_groups(
    parsed: &LegacyMol,
    atom_points: &[Point],
) -> (Vec<LegacyCollapsedGroup>, BTreeSet<usize>, BTreeSet<usize>) {
    let mut collapsed_groups = Vec::new();
    let mut hidden_atoms = BTreeSet::new();
    let mut hidden_bonds = BTreeSet::new();

    for sgroup in &parsed.sgroups {
        if sgroup.kind != "SUP" || sgroup.label.trim().is_empty() || sgroup.atoms.is_empty() {
            continue;
        }
        let group_atoms: BTreeSet<usize> = sgroup.atoms.iter().copied().collect();
        let mut connections = Vec::new();
        for (bond_index, bond) in parsed.bonds.iter().enumerate() {
            let begin_inside = group_atoms.contains(&bond.begin);
            let end_inside = group_atoms.contains(&bond.end);
            if begin_inside && end_inside {
                hidden_bonds.insert(bond_index);
            } else if begin_inside || end_inside {
                connections.push(if begin_inside { bond.end } else { bond.begin });
                hidden_bonds.insert(bond_index);
            }
        }
        hidden_atoms.extend(group_atoms.iter().copied());

        let points: Vec<Point> = sgroup
            .atoms
            .iter()
            .filter_map(|index| atom_points.get(*index).copied())
            .collect();
        if points.is_empty() {
            continue;
        }

        let mut anchor_point = None;
        let mut direction = None;
        for bond_index in &sgroup.bonds {
            let Some(bond) = parsed.bonds.get(*bond_index) else {
                continue;
            };
            let inside_atom_index = if group_atoms.contains(&bond.begin) {
                Some(bond.begin)
            } else if group_atoms.contains(&bond.end) {
                Some(bond.end)
            } else {
                None
            };
            let Some(inside_atom_index) = inside_atom_index else {
                continue;
            };
            anchor_point = atom_points.get(inside_atom_index).copied();
            if let Some(vector) = sgroup.vectors.get(bond_index) {
                direction = Some(Vector::new(vector.x, -vector.y));
            }
            break;
        }

        let anchor_point = anchor_point.unwrap_or_else(|| Point::new(
            points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
            points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
        ));
        let direction = direction.unwrap_or_else(|| {
            if let Some(outside_atom) = connections.first().and_then(|index| atom_points.get(*index)).copied() {
                Vector::new(anchor_point.x - outside_atom.x, anchor_point.y - outside_atom.y)
            } else {
                Vector::new(0.0, -1.0)
            }
        });
        let unit = direction.normalized();
        let label_width = 20.0_f64.max(sgroup.label.chars().count() as f64 * 7.1 + 8.0);
        collapsed_groups.push(LegacyCollapsedGroup {
            label: sgroup.label.clone(),
            centroid: Point::new(
                anchor_point.x + unit.x * (label_width * 0.45 + 7.0),
                anchor_point.y + unit.y * 10.0,
            ),
            connections,
        });
    }

    (collapsed_groups, hidden_atoms, hidden_bonds)
}

fn legacy_bond_stroke_width(document: &ChemcoreDocument, object: &SceneObject) -> f64 {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_number(style, "strokeWidth").or_else(|| style_number(style, "stroke_width")))
        .unwrap_or(VIEWER_BOND_STROKE)
}

fn legacy_label_font_family(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "fontFamily"))
        .unwrap_or_else(|| "Arial".to_string())
}

fn legacy_label_font_size(document: &ChemcoreDocument, object: &SceneObject) -> f64 {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_number(style, "fontSize").or_else(|| style_number(style, "font_size")))
        .unwrap_or(11.0)
}

fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let Some(label) = node.label.as_ref() else {
        return;
    };
    if !label.has_visible_text() {
        return;
    }

    let font_size = fragment_label_font_size(label);
    let text_anchor = text_anchor(label.align.as_deref().unwrap_or("left"));
    let font_family = label.font_family.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fontFamily"))
    });
    let fill = label.fill.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fill"))
    });
    if let Some(box_value) = label_box_world(node, object) {
        out.push(RenderPrimitive::Rect {
            role: RenderRole::DocumentKnockout,
            object_id: object_id.clone(),
            x: box_value.x1,
            y: box_value.y1,
            width: (box_value.x2 - box_value.x1).max(0.0),
            height: (box_value.y2 - box_value.y1).max(0.0),
            fill: Some(document.document.page.background.clone()),
            stroke: None,
            stroke_width: 0.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        });
    }

    let lines = fragment_label_lines(label);
    if lines.is_empty() {
        return;
    }
    let world_position = fragment_label_position_world(label, object);
    if lines.len() == 1 {
        push_text(
            out,
            world_position.x,
            world_position.y,
            String::new(),
            font_size,
            font_family,
            fill,
            Some(text_anchor),
            fragment_label_runs_for_line(label, 0, &lines[0]),
            object_id,
        );
        return;
    }

    let label_box = label_box_world(node, object);
    let line_height = label_box
        .map(|box_value| (box_value.y2 - box_value.y1) / lines.len() as f64)
        .unwrap_or(font_size * 1.05);
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - line_height * 0.82);
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = box_top + line_height * index as f64 + line_height * 0.82;
        push_text(
            out,
            world_position.x,
            baseline_y,
            String::new(),
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            fragment_label_runs_for_line(label, index, line),
            object_id.clone(),
        );
    }
}

fn render_fragment_bond(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    stroke: &str,
    object_id: Option<String>,
) {
    let Some(begin) = node_map.get(bond.begin.as_str()).copied() else {
        return;
    };
    let Some(end) = node_map.get(bond.end.as_str()).copied() else {
        return;
    };
    let stroke_width = bond_stroke_width(document, object, bond);
    let mut start = world_point(object, begin);
    let mut finish = world_point(object, end);
    let begin_box = label_box_world(begin, object);
    let end_box = label_box_world(end, object);
    let begin_has_label = begin.label.as_ref().is_some_and(|label| label.has_visible_text());
    let end_has_label = end.label.as_ref().is_some_and(|label| label.has_visible_text());

    start = clip_point_out_of_box(start, finish, begin_box, 1.8);
    finish = clip_point_out_of_box(finish, start, end_box, 1.8);

    if let Some(stereo) = bond_stereo_kind(bond) {
        render_stereo_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            stereo,
            start,
            finish,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    if bond.order == 2 {
        render_double_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            start,
            finish,
            begin_box,
            end_box,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    if bond.order >= 3 {
        render_triple_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            start,
            finish,
            begin_box,
            end_box,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    render_fragment_line(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        finish,
        begin_box,
        end_box,
        true,
        stroke,
        stroke_width,
        line_pattern_dash_array(bond.line_styles.main),
        bond.line_weights.main,
        object_id,
    );
}

fn render_double_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let side_mode = bond.double.as_ref().map(|double| double.placement);
    let double_offset = double_bond_offset_distance(start, end, stroke_width);

    match side_mode {
        Some(DoubleBondPlacement::Left) | Some(DoubleBondPlacement::Right) => {
            let side = if side_mode == Some(DoubleBondPlacement::Left) {
                1.0
            } else {
                -1.0
            };
            render_fragment_line(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                true,
                stroke,
                stroke_width,
                line_pattern_dash_array(bond.line_styles.main),
                bond.line_weights.main,
                object_id.clone(),
            );
            render_outer_bond_lines(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                begin_has_label,
                end_has_label,
                stroke,
                stroke_width,
                object_id,
                &[side],
                double_offset,
            );
        }
        _ => {
            render_center_double_bond_lines(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                stroke,
                stroke_width,
                object_id,
                double_offset,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_center_double_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    double_offset: f64,
) {
    let (normal_x, normal_y) = unit_normal(start, end);
    for (line_side, offset, pattern, weight) in [
        (-1.0, -double_offset / 2.0, bond.line_styles.left, bond.line_weights.left),
        (1.0, double_offset / 2.0, bond.line_styles.right, bond.line_weights.right),
    ] {
        let line_start = Point::new(start.x + normal_x * offset, start.y + normal_y * offset);
        let line_end = Point::new(end.x + normal_x * offset, end.y + normal_y * offset);
        let start_endpoint_profile = center_double_endpoint_profile_for_line_side(
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            &bond.begin,
            line_side,
            stroke_width,
            weight,
        );
        let end_endpoint_profile = center_double_endpoint_profile_for_line_side(
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            &bond.end,
            line_side,
            stroke_width,
            weight,
        );
        render_fragment_line_with_profiles(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            line_start,
            line_end,
            begin_box,
            end_box,
            false,
            stroke,
            stroke_width,
            line_pattern_dash_array(pattern),
            weight,
            object_id.clone(),
            false,
            start_endpoint_profile,
            end_endpoint_profile,
        );
    }
}

fn render_triple_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let triple_offset = triple_bond_offset_distance(start, end, stroke_width);

    render_fragment_line(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        begin_box,
        end_box,
        true,
        stroke,
        stroke_width,
        line_pattern_dash_array(bond.line_styles.main),
        bond.line_weights.main,
        object_id.clone(),
    );

    render_outer_bond_lines(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        begin_box,
        end_box,
        begin_has_label,
        end_has_label,
        stroke,
        stroke_width,
        object_id,
        &[1.0, -1.0],
        triple_offset,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_outer_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    sides: &[f64],
    offset_distance: f64,
) {
    let length = start.distance(end);
    let is_side_double = side_double_placement(bond).is_some();
    let side_inset = if is_side_double {
        offset_distance * (3.0f64).sqrt() / 3.0
    } else {
        (DOUBLE_BOND_SIDE_INSET * (stroke_width / VIEWER_BOND_STROKE))
            .max(length * DOUBLE_BOND_SIDE_INSET_RATIO)
    };
    let begin_terminal = fragment_node_degree(bonds, &bond.begin) <= 1;
    let end_terminal = fragment_node_degree(bonds, &bond.end) <= 1;
    let (normal_x, normal_y) = unit_normal(start, end);

    for side in sides {
        let line_pattern = outer_line_pattern(bond, *side);
        let line_weight = outer_line_weight(bond, *side);
        let offset_start = Point::new(
            start.x + normal_x * offset_distance * *side,
            start.y + normal_y * offset_distance * *side,
        );
        let offset_end = Point::new(
            end.x + normal_x * offset_distance * *side,
            end.y + normal_y * offset_distance * *side,
        );
        let start_endpoint_profile = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            *side,
            stroke_width,
        );
        let end_endpoint_profile = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            *side,
            stroke_width,
        );
        let (short_start, short_end) = inset_bond_segment(
            offset_start,
            offset_end,
            if start_endpoint_profile.is_some() || begin_has_label || begin_terminal {
                0.0
            } else {
                side_inset
            },
            if end_endpoint_profile.is_some() || end_has_label || end_terminal {
                0.0
            } else {
                side_inset
            },
        );
        render_fragment_line_with_profiles(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            short_start,
            short_end,
            begin_box,
            end_box,
            false,
            stroke,
            stroke_width,
            line_pattern_dash_array(line_pattern),
            line_weight,
            object_id.clone(),
            false,
            start_endpoint_profile,
            end_endpoint_profile,
        );
    }
}

fn render_stereo_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    stereo: BondStereoKind,
    start: Point,
    end: Point,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    match stereo {
        BondStereoKind::SolidWedgeEnd => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.end,
                start,
                end,
                if end_has_label { SOLID_WEDGE_END_INSET } else { 0.0 },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.end),
            );
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
        }
        BondStereoKind::SolidWedgeBegin => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.begin,
                end,
                start,
                if begin_has_label { SOLID_WEDGE_END_INSET } else { 0.0 },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
        }
        BondStereoKind::HashedWedgeEnd => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.end,
                start,
                end,
                if end_has_label { SOLID_WEDGE_END_INSET } else { 0.0 },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.end),
            );
            push_bond_polygon(out, &bond.id, points.clone(), stroke, stroke, 0.0, object_id.clone());
            for knockout in compute_fragment_hashed_wedge_knockout_polygons(&points, stroke_width) {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
        }
        BondStereoKind::HashedWedgeBegin => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.begin,
                end,
                start,
                if begin_has_label { SOLID_WEDGE_END_INSET } else { 0.0 },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_bond_polygon(out, &bond.id, points.clone(), stroke, stroke, 0.0, object_id.clone());
            for knockout in compute_fragment_hashed_wedge_knockout_polygons(&points, stroke_width) {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
        }
    }
}

fn compute_solid_wedge_points(
    start: Point,
    end: Point,
    end_inset: f64,
    wide_contact_directions: Vec<Vector>,
    stroke_width: f64,
) -> Vec<Point> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    let width = solid_wedge_half_width(stroke_width);
    let tip_plus = Point::new(start.x + normal.x * tip_half_width, start.y + normal.y * tip_half_width);
    let tip_minus = Point::new(start.x - normal.x * tip_half_width, start.y - normal.y * tip_half_width);
    let cap_inset = end_inset.min(length * 0.22);
    let cap_center = Point::new(end.x - unit.x * cap_inset, end.y - unit.y * cap_inset);
    let cap_plus = Point::new(cap_center.x + normal.x * width, cap_center.y + normal.y * width);
    let cap_minus = Point::new(cap_center.x - normal.x * width, cap_center.y - normal.y * width);

    let contacts = contact_entries(&wide_contact_directions, normal);
    let has_plus = contacts.iter().any(|entry| entry.side > 0.0);
    let has_minus = contacts.iter().any(|entry| entry.side < 0.0);

    if has_plus && has_minus {
        let plus = contacts
            .iter()
            .filter(|entry| entry.side > 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        let minus = contacts
            .iter()
            .filter(|entry| entry.side < 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let (Some(plus), Some(minus)) = (plus, minus) {
            let plus_intersection = line_intersection(
                tip_plus,
                Vector::new(cap_plus.x - tip_plus.x, cap_plus.y - tip_plus.y),
                far_side_contact_line_point(end, plus.direction, start, stroke_width),
                plus.direction,
            )
            .unwrap_or(cap_plus);
            let minus_intersection = line_intersection(
                tip_minus,
                Vector::new(cap_minus.x - tip_minus.x, cap_minus.y - tip_minus.y),
                far_side_contact_line_point(end, minus.direction, start, stroke_width),
                minus.direction,
            )
            .unwrap_or(cap_minus);
            return vec![tip_plus, plus_intersection, minus_intersection, tip_minus];
        }
    }

    if has_plus || has_minus {
        let side = if has_plus { 1.0 } else { -1.0 };
        let contact = contacts
            .iter()
            .filter(|entry| entry.side == side)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let Some(contact) = contact {
            let plus_intersection = line_intersection(
                tip_plus,
                Vector::new(cap_plus.x - tip_plus.x, cap_plus.y - tip_plus.y),
                far_side_contact_line_point(end, contact.direction, start, stroke_width),
                contact.direction,
            )
            .unwrap_or(cap_plus);
            let minus_intersection = line_intersection(
                tip_minus,
                Vector::new(cap_minus.x - tip_minus.x, cap_minus.y - tip_minus.y),
                far_side_contact_line_point(end, contact.direction, start, stroke_width),
                contact.direction,
            )
            .unwrap_or(cap_minus);
            return vec![tip_plus, plus_intersection, minus_intersection, tip_minus];
        }
    }

    vec![tip_plus, cap_plus, cap_minus, tip_minus]
}

fn compute_fragment_solid_wedge_points(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    wide_node_id: &str,
    start: Point,
    end: Point,
    end_inset: f64,
    stroke_width: f64,
    allow_endpoint_contacts: bool,
) -> Vec<Point> {
    let narrow_node_id = if wide_node_id == bond.begin {
        bond.end.as_str()
    } else {
        bond.begin.as_str()
    };
    let start_retreat = contact_kernel.endpoint_retreat(&bond.id, narrow_node_id);
    let mut end_retreat = if is_hashed_wedge_bond(bond) {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, wide_node_id)
    };
    if is_hashed_wedge_bond(bond) && allow_endpoint_contacts {
        end_retreat = end_retreat.max(
            endpoint_retreat_against_center_double_outer_line(
                object,
                bonds,
                node_map,
                bond,
                wide_node_id,
                end,
                Vector::new(start.x - end.x, start.y - end.y).normalized(),
                solid_wedge_half_width(stroke_width),
                stroke_width,
            )
            .unwrap_or(0.0),
        );
    }
    let (start, end) = apply_segment_endpoint_retreats(start, end, start_retreat, end_retreat);
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    let width = solid_wedge_half_width(stroke_width);
    let tip_plus = Point::new(start.x + normal.x * tip_half_width, start.y + normal.y * tip_half_width);
    let tip_minus = Point::new(start.x - normal.x * tip_half_width, start.y - normal.y * tip_half_width);
    let cap_inset = end_inset.min(length * 0.22);
    let cap_center = Point::new(end.x - unit.x * cap_inset, end.y - unit.y * cap_inset);
    let cap_plus = Point::new(cap_center.x + normal.x * width, cap_center.y + normal.y * width);
    let cap_minus = Point::new(cap_center.x - normal.x * width, cap_center.y - normal.y * width);
    let start_profile = endpoint_profile_global(
        if start_retreat > EPSILON {
            None
        } else {
            contact_kernel.endpoint_profile(&bond.id, narrow_node_id)
        },
        false,
        vec![tip_plus, tip_minus],
    );

    if end_retreat <= EPSILON {
        if let Some(profile) = contact_kernel.endpoint_profile(&bond.id, wide_node_id) {
            let end_profile = endpoint_profile_global(Some(profile), true, vec![cap_plus, cap_minus]);
            return bond_polygon_from_endpoint_profiles(start_profile, end_profile);
        }
    }

    if is_hashed_wedge_bond(bond) {
        return bond_polygon_from_endpoint_profiles(start_profile, vec![cap_plus, cap_minus]);
    }

    if allow_endpoint_contacts {
        if let Some((join_plus, join_minus)) = solid_wedge_cap_points(
            object,
            bonds,
            node_map,
            bond,
            wide_node_id,
            tip_plus,
            tip_minus,
            end,
            cap_plus,
            cap_minus,
            stroke_width,
        ) {
            return bond_polygon_from_endpoint_profiles(
                start_profile,
                vec![join_plus, join_minus],
            );
        }
    }

    let points = compute_solid_wedge_points(
        start,
        end,
        end_inset,
        if allow_endpoint_contacts {
            wide_contact_directions(object, bonds, node_map, bond, wide_node_id)
        } else {
            Vec::new()
        },
        stroke_width,
    );
    if points.len() == 4 {
        return bond_polygon_from_endpoint_profiles(start_profile, vec![points[1], points[2]]);
    }
    points
}

fn compute_fragment_hashed_wedge_knockout_polygons(
    polygon: &[Point],
    stroke_width: f64,
) -> Vec<Vec<Point>> {
    if polygon.len() != 4 {
        return Vec::new();
    }
    let tip_plus = polygon[0];
    let cap_plus = polygon[1];
    let cap_minus = polygon[2];
    let tip_minus = polygon[3];
    let tip_center = midpoint(tip_plus, tip_minus);
    let cap_center = midpoint(cap_plus, cap_minus);
    let direction = Vector::new(cap_center.x - tip_center.x, cap_center.y - tip_center.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }

    let mut knockouts = Vec::new();
    for (gap_start, gap_end) in hashed_wedge_gap_intervals(length, stroke_width) {
        let t0 = (gap_start / length).clamp(0.0, 1.0);
        let t1 = (gap_end / length).clamp(0.0, 1.0);
        let mut top_start = lerp_point(tip_plus, cap_plus, t0);
        let mut top_end = lerp_point(tip_plus, cap_plus, t1);
        let mut bottom_end = lerp_point(tip_minus, cap_minus, t1);
        let mut bottom_start = lerp_point(tip_minus, cap_minus, t0);
        let overdraw = HASH_WEDGE_EDGE_OVERDRAW * (stroke_width / VIEWER_BOND_STROKE);
        for (upper, lower) in [(&mut top_start, &mut bottom_start), (&mut top_end, &mut bottom_end)] {
            let mid = Point::new((upper.x + lower.x) * 0.5, (upper.y + lower.y) * 0.5);
            let upper_out = Vector::new(upper.x - mid.x, upper.y - mid.y).normalized();
            let lower_out = Vector::new(lower.x - mid.x, lower.y - mid.y).normalized();
            upper.x += upper_out.x * overdraw;
            upper.y += upper_out.y * overdraw;
            lower.x += lower_out.x * overdraw;
            lower.y += lower_out.y * overdraw;
        }
        knockouts.push(compact_polygon_points(vec![top_start, top_end, bottom_end, bottom_start]));
    }
    knockouts
}

fn compute_bold_bond_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    allow_start_contacts: bool,
    allow_end_contacts: bool,
    start_endpoint_profile: Option<Vec<Point>>,
    end_endpoint_profile: Option<Vec<Point>>,
) -> Vec<Point> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) / 2.0;
    let start_profile = if let Some(profile) = start_endpoint_profile {
        endpoint_profile_global(Some(profile), false, Vec::new())
    } else if allow_start_contacts {
        let (start_plus, start_minus) = bold_band_cap_points(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            start,
            unit,
            normal,
            half_width,
            end,
            stroke_width,
        );
        vec![start_plus, start_minus]
    } else {
        vec![
            Point::new(start.x + normal.x * half_width, start.y + normal.y * half_width),
            Point::new(start.x - normal.x * half_width, start.y - normal.y * half_width),
        ]
    };
    let end_profile = if let Some(profile) = end_endpoint_profile {
        endpoint_profile_global(Some(profile), true, Vec::new())
    } else if allow_end_contacts {
        let (end_plus, end_minus) = bold_band_cap_points(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            end,
            Vector::new(-unit.x, -unit.y),
            normal,
            half_width,
            start,
            stroke_width,
        );
        vec![end_plus, end_minus]
    } else {
        vec![
            Point::new(end.x + normal.x * half_width, end.y + normal.y * half_width),
            Point::new(end.x - normal.x * half_width, end.y - normal.y * half_width),
        ]
    };
    if length <= 1.0e-6 {
        return bond_polygon_from_endpoint_profiles(start_profile, end_profile);
    }
    bond_polygon_from_endpoint_profiles(start_profile, end_profile)
}

fn is_hash_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.line_styles.main == BondLinePattern::Dashed
        && bond.line_weights.main == BondLineWeight::Bold
}

fn is_hashed_wedge_bond(bond: &Bond) -> bool {
    matches!(
        bond_stereo_kind(bond),
        Some(BondStereoKind::HashedWedgeBegin | BondStereoKind::HashedWedgeEnd)
    )
}

fn is_hash_contact_obstacle(bond: &Bond) -> bool {
    is_hash_bond(bond) || is_hashed_wedge_bond(bond)
}

fn apply_segment_endpoint_retreats(
    start: Point,
    end: Point,
    start_retreat: f64,
    end_retreat: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return (start, end);
    }
    let unit = direction.normalized();
    let max_total = length * 0.8;
    let desired_total = start_retreat.max(0.0) + end_retreat.max(0.0);
    let scale = if desired_total > max_total && desired_total > EPSILON {
        max_total / desired_total
    } else {
        1.0
    };
    let start_shift = start_retreat.max(0.0) * scale;
    let end_shift = end_retreat.max(0.0) * scale;
    (
        Point::new(start.x + unit.x * start_shift, start.y + unit.y * start_shift),
        Point::new(end.x - unit.x * end_shift, end.y - unit.y * end_shift),
    )
}

fn bold_band_cap_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    normal: Vector,
    half_width: f64,
    interior_point: Point,
    stroke_width: f64,
) -> (Point, Point) {
    let base_plus = Point::new(endpoint.x + normal.x * half_width, endpoint.y + normal.y * half_width);
    let base_minus = Point::new(endpoint.x - normal.x * half_width, endpoint.y - normal.y * half_width);
    if is_hash_bond(bond) {
        return (base_plus, base_minus);
    }
    if let Some(join_plus) = bold_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        endpoint,
        forward,
        normal,
        half_width,
        1.0,
        stroke_width,
    ) {
        if let Some(join_minus) = bold_edge_join_point(
            object,
            bonds,
            node_map,
            bond,
            shared_node_id,
            endpoint,
            forward,
            normal,
            half_width,
            -1.0,
            stroke_width,
        ) {
            return (join_plus, join_minus);
        }
    }

    let mut contact_directions = Vec::new();
    if let Some(shared_node) = node_map.get(shared_node_id).copied() {
        let shared_point = world_point(object, shared_node);
        for other_bond in bonds {
            if other_bond.id == bond.id || !is_wide_contact_candidate(other_bond) {
                continue;
            }
            if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
                continue;
            }
            if has_joinable_main_line(other_bond) {
                continue;
            }
            let other_node_id = if other_bond.begin == shared_node_id {
                other_bond.end.as_str()
            } else {
                other_bond.begin.as_str()
            };
            let Some(other_node) = node_map.get(other_node_id).copied() else {
                continue;
            };
            let other_point = world_point(object, other_node);
            let vector = Vector::new(other_point.x - shared_point.x, other_point.y - shared_point.y);
            if vector.length() > 1.0e-6 {
                contact_directions.push(vector);
            }
        }
    }
    let contacts = contact_entries(&contact_directions, normal);
    let has_plus = contacts.iter().any(|entry| entry.side > 0.0);
    let has_minus = contacts.iter().any(|entry| entry.side < 0.0);

    if has_plus && has_minus {
        let plus = contacts
            .iter()
            .filter(|entry| entry.side > 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        let minus = contacts
            .iter()
            .filter(|entry| entry.side < 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let (Some(plus), Some(minus)) = (plus, minus) {
            let plus_intersection = line_intersection(
                base_plus,
                forward,
                far_side_contact_line_point(endpoint, plus.direction, interior_point, stroke_width),
                plus.direction,
            )
            .unwrap_or(base_plus);
            let minus_intersection = line_intersection(
                base_minus,
                forward,
                far_side_contact_line_point(endpoint, minus.direction, interior_point, stroke_width),
                minus.direction,
            )
            .unwrap_or(base_minus);
            return (plus_intersection, minus_intersection);
        }
    }

    if has_plus || has_minus {
        let side = if has_plus { 1.0 } else { -1.0 };
        let contact = contacts
            .iter()
            .filter(|entry| entry.side == side)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let Some(contact) = contact {
            let plus_intersection = line_intersection(
                base_plus,
                forward,
                far_side_contact_line_point(endpoint, contact.direction, interior_point, stroke_width),
                contact.direction,
            )
            .unwrap_or(base_plus);
            let minus_intersection = line_intersection(
                base_minus,
                forward,
                far_side_contact_line_point(endpoint, contact.direction, interior_point, stroke_width),
                contact.direction,
            )
            .unwrap_or(base_minus);
            return (plus_intersection, minus_intersection);
        }
    }

    (base_plus, base_minus)
}

#[allow(clippy::too_many_arguments)]
fn solid_wedge_cap_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    tip_plus: Point,
    tip_minus: Point,
    endpoint: Point,
    cap_plus: Point,
    cap_minus: Point,
    stroke_width: f64,
) -> Option<(Point, Point)> {
    if let Some(join_points) =
        wide_endpoint_join_points_against_main_lines(object, bonds, node_map, bond, shared_node_id, stroke_width)
    {
        return Some(join_points);
    }
    let join_plus = solid_wedge_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        tip_plus,
        endpoint,
        cap_plus,
        stroke_width,
    )?;
    let join_minus = solid_wedge_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        tip_minus,
        endpoint,
        cap_minus,
        stroke_width,
    )?;
    Some((join_plus, join_minus))
}

#[allow(clippy::too_many_arguments)]
fn solid_wedge_edge_join_point(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    tip: Point,
    endpoint: Point,
    cap_point: Point,
    stroke_width: f64,
) -> Option<Point> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let edge_direction = Vector::new(cap_point.x - tip.x, cap_point.y - tip.y);
    if edge_direction.length() <= EPSILON {
        return None;
    }
    let mut best: Option<(Point, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if has_joinable_main_line(other_bond) {
            if let Some(other_line) =
                main_bond_cap_line_for_endpoint(object, node_map, other_bond, shared_node_id)
            {
                let Some((intersection, t, _u)) = line_intersection_with_parameters(
                    tip,
                    edge_direction,
                    other_line.point,
                    other_line.direction,
                ) else {
                    continue;
                };
                if t < 0.65 {
                    continue;
                }
                let endpoint_distance = intersection.distance(endpoint);
                let max_join_distance = other_line.length.min(edge_direction.length()) * 0.45;
                if endpoint_distance
                    > (stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)) * 4.5)
                        .max(max_join_distance)
                {
                    continue;
                }
                if best
                    .as_ref()
                    .is_none_or(|(_, best_distance)| endpoint_distance < *best_distance)
                {
                    best = Some((intersection, endpoint_distance));
                }
                continue;
            }
        }
        for other_side in main_bond_candidate_sides(other_bond) {
            let Some(other_line) = main_bond_boundary_line_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)),
            ) else {
                continue;
            };
            let Some((intersection, t, u)) = line_intersection_with_parameters(
                tip,
                edge_direction,
                other_line.point,
                other_line.direction,
            ) else {
                continue;
            };
            if t < 0.65 || u < -0.2 {
                continue;
            }
            let endpoint_distance = intersection.distance(endpoint);
            let max_join_distance = other_line.length.min(edge_direction.length()) * 0.45;
            if endpoint_distance
                > (stroke_width.max(other_line.offset_distance) * 4.5).max(max_join_distance)
            {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|(_, best_distance)| endpoint_distance < *best_distance)
            {
                best = Some((intersection, endpoint_distance));
            }
        }
    }
    best.map(|(point, _)| point)
}

#[allow(clippy::too_many_arguments)]
fn bold_edge_join_point(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    normal: Vector,
    half_width: f64,
    side: f64,
    stroke_width: f64,
) -> Option<Point> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current_point = Point::new(
        endpoint.x + normal.x * half_width * side,
        endpoint.y + normal.y * half_width * side,
    );
    let mut best: Option<(Point, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if solid_joinable_main_line(other_bond) {
            continue;
        }
        if has_joinable_main_line(other_bond) {
            if let Some(other_line) =
                main_bond_cap_line_for_endpoint(object, node_map, other_bond, shared_node_id)
            {
                let Some((intersection, t, _u)) = line_intersection_with_parameters(
                    current_point,
                    forward,
                    other_line.point,
                    other_line.direction,
                ) else {
                    continue;
                };
                let min_backtrack = -(half_width * 2.5).max(stroke_width);
                if t < min_backtrack {
                    continue;
                }
                let distance = intersection.distance(endpoint);
                if distance > other_line.length.max(forward.length()) * 0.45 {
                    continue;
                }
                if best.as_ref().is_none_or(|(_, best_distance)| distance < *best_distance) {
                    best = Some((intersection, distance));
                }
                continue;
            }
        }
        for other_side in main_bond_candidate_sides(other_bond) {
            let Some(other_line) = main_bond_boundary_line_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)),
            ) else {
                continue;
            };
            let Some((intersection, t, u)) = line_intersection_with_parameters(
                current_point,
                forward,
                other_line.point,
                other_line.direction,
            ) else {
                continue;
            };
            if t < -0.2 || u < -0.2 {
                continue;
            }
            let distance = intersection.distance(endpoint);
            if distance > other_line.length.max(forward.length()) * 0.45 {
                continue;
            }
            if best.as_ref().is_none_or(|(_, best_distance)| distance < *best_distance) {
                best = Some((intersection, distance));
            }
        }
    }
    best.map(|(point, _)| point)
}

fn compute_hashed_wedge_segments(
    start: Point,
    end: Point,
    stroke_width: f64,
) -> Vec<(Point, Point, f64)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start_gap = HASH_WEDGE_START_OFFSET.min(length * 0.3);
    let end_gap = HASH_WEDGE_END_INSET.min(length * 0.08);
    let usable = (length - start_gap - end_gap).max(0.01);
    let steps = ((usable / HASH_WEDGE_SPACING).round() as usize + 1).max(2);
    let spacing = if steps > 1 {
        usable / (steps - 1) as f64
    } else {
        usable
    };
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let mut segments = Vec::new();
    for index in 0..steps {
        let dist = start_gap + spacing * index as f64;
        if dist > length - end_gap + 1.0e-6 {
            break;
        }
        let progress = if steps > 1 {
            index as f64 / (steps - 1) as f64
        } else {
            1.0
        };
        let half_width = if index == 0 {
            0.42
        } else {
            0.16 + progress * 1.72
        } * scale;
        let center = Point::new(start.x + unit.x * dist, start.y + unit.y * dist);
        let segment_width = if index == 0 { 0.82 } else { 0.72 } * scale;
        if index == 0 {
            segments.push((
                Point::new(center.x, center.y - half_width),
                Point::new(center.x, center.y + half_width),
                segment_width,
            ));
        } else {
            segments.push((
                Point::new(center.x - normal.x * half_width, center.y - normal.y * half_width),
                Point::new(center.x + normal.x * half_width, center.y + normal.y * half_width),
                segment_width,
            ));
        }
    }
    segments
}

fn lerp_point(from: Point, to: Point, t: f64) -> Point {
    Point::new(
        from.x + (to.x - from.x) * t,
        from.y + (to.y - from.y) * t,
    )
}

fn has_joinable_main_line(bond: &Bond) -> bool {
    if bond.stereo.is_some() || bond.line_weights.main != BondLineWeight::Normal {
        return false;
    }
    if bond.order == 1 || bond.order >= 3 {
        return true;
    }
    bond.order == 2 && side_double_placement(bond).is_some()
}

fn is_joinable_main_line_render(
    bond: &Bond,
    allow_bold_contacts: bool,
    line_weight: BondLineWeight,
) -> bool {
    allow_bold_contacts && line_weight == BondLineWeight::Normal && has_joinable_main_line(bond)
}

fn boundary_lines_from_endpoint(
    endpoint: Point,
    forward: Vector,
    half_width: f64,
) -> Option<[LineGeometry; 2]> {
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    Some([
        LineGeometry {
            point: Point::new(endpoint.x + normal.x * half_width, endpoint.y + normal.y * half_width),
            direction: unit,
            shared: endpoint,
            length,
            offset_distance: half_width,
        },
        LineGeometry {
            point: Point::new(endpoint.x - normal.x * half_width, endpoint.y - normal.y * half_width),
            direction: unit,
            shared: endpoint,
            length,
            offset_distance: half_width,
        },
    ])
}

fn main_line_boundary_lines_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    let center_line = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    boundary_lines_from_endpoint(center_line.shared, center_line.direction, stroke_width * 0.5)
}

fn wide_boundary_line_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    let lines = wide_boundary_lines_for_endpoint(object, node_map, bond, shared_node_id, stroke_width);
    if lines.len() != 2 {
        return None;
    }
    Some([lines[0], lines[1]])
}

#[derive(Debug, Clone, Copy)]
struct BoundaryJoinCandidate {
    point: Point,
    score: f64,
    t: f64,
    u: f64,
}

fn boundary_line_join_candidate(current: &LineGeometry, other: &LineGeometry) -> Option<BoundaryJoinCandidate> {
    let (intersection, t, u) = line_intersection_with_parameters(
        current.point,
        current.direction,
        other.point,
        other.direction,
    )?;
    let min_current = -(current.offset_distance * 4.0).max(0.85);
    let min_other = -(other.offset_distance * 4.0).max(0.85);
    if t < min_current || u < min_other {
        return None;
    }
    let distance = intersection.distance(current.shared);
    let max_join_distance =
        current.length.min(other.length) * 0.55 + current.offset_distance.max(other.offset_distance) * 4.0;
    if distance > max_join_distance {
        return None;
    }
    Some(BoundaryJoinCandidate {
        point: intersection,
        score: distance,
        t,
        u,
    })
}

fn is_trivial_boundary_assignment(candidates: [BoundaryJoinCandidate; 2]) -> bool {
    candidates
        .iter()
        .all(|candidate| candidate.t.abs() <= 1.0e-4 && candidate.u.abs() <= 1.0e-4)
}

fn paired_boundary_line_join_points(
    current: [LineGeometry; 2],
    other: [LineGeometry; 2],
) -> Option<([Point; 2], f64)> {
    let direct = boundary_line_join_candidate(&current[0], &other[0])
        .zip(boundary_line_join_candidate(&current[1], &other[1]))
        .map(|(plus, minus)| ([plus.point, minus.point], plus.score + minus.score, [plus, minus]));
    let swapped = boundary_line_join_candidate(&current[0], &other[1])
        .zip(boundary_line_join_candidate(&current[1], &other[0]))
        .map(|(plus, minus)| ([plus.point, minus.point], plus.score + minus.score, [plus, minus]));
    match (direct, swapped) {
        (Some(a), Some(b)) => {
            if is_trivial_boundary_assignment(a.2) && !is_trivial_boundary_assignment(b.2) {
                Some((b.0, b.1))
            } else if is_trivial_boundary_assignment(b.2) && !is_trivial_boundary_assignment(a.2) {
                Some((a.0, a.1))
            } else if a.1 <= b.1 {
                Some((a.0, a.1))
            } else {
                Some((b.0, b.1))
            }
        }
        (Some(a), None) => Some((a.0, a.1)),
        (None, Some(b)) => Some((b.0, b.1)),
        (None, None) => None,
    }
}

fn extended_boundary_line_join_points(
    current: [LineGeometry; 2],
    other: [LineGeometry; 2],
) -> Option<([Point; 2], f64)> {
    let direct = line_intersection(current[0].point, current[0].direction, other[0].point, other[0].direction)
        .zip(line_intersection(
            current[1].point,
            current[1].direction,
            other[1].point,
            other[1].direction,
        ))
        .map(|(plus, minus)| {
            let score = plus.distance(current[0].shared) + minus.distance(current[1].shared);
            ([plus, minus], score)
        });
    let swapped = line_intersection(current[0].point, current[0].direction, other[1].point, other[1].direction)
        .zip(line_intersection(
            current[1].point,
            current[1].direction,
            other[0].point,
            other[0].direction,
        ))
        .map(|(plus, minus)| {
            let score = plus.distance(current[0].shared) + minus.distance(current[1].shared);
            ([plus, minus], score)
        });

    match (direct, swapped) {
        (Some(a), Some(b)) => {
            if a.1 <= b.1 {
                Some(a)
            } else {
                Some(b)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn wide_endpoint_join_points_against_main_lines(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<(Point, Point)> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current = wide_boundary_line_pair_for_endpoint(object, node_map, bond, shared_node_id, stroke_width)?;
    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if !has_joinable_main_line(other_bond) {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let Some(other) = main_line_boundary_lines_for_endpoint(
            object,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(candidate) = extended_boundary_line_join_points(current, other) else {
            continue;
        };
        if best.as_ref().is_none_or(|(_, best_score)| candidate.1 < *best_score) {
            best = Some(candidate);
        }
    }
    best.map(|(points, _)| (points[0], points[1]))
}

fn main_line_join_points_against_wide_bonds(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    current: [LineGeometry; 2],
    stroke_width: f64,
) -> Option<(Point, Point)> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if bond_stereo_kind(other_bond).is_none() {
            continue;
        }
        if is_hashed_wedge_bond(other_bond) {
            continue;
        }
        let other_stroke_width = stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE));
        let Some(other) = wide_boundary_line_pair_for_endpoint(
            object,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(candidate) = paired_boundary_line_join_points(current, other) else {
            continue;
        };
        if best.as_ref().is_none_or(|(_, best_score)| candidate.1 < *best_score) {
            best = Some(candidate);
        }
    }
    best.map(|(points, _)| (points[0], points[1]))
}

fn solid_joinable_main_line(bond: &Bond) -> bool {
    has_joinable_main_line(bond)
        && bond.line_weights.main == BondLineWeight::Normal
        && bond.line_styles.main == BondLinePattern::Solid
}

fn main_line_far_boundary_for_wide_bond(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    contact_sample: Point,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let center = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    let unit = center.direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let to_contact = Vector::new(contact_sample.x - center.shared.x, contact_sample.y - center.shared.y);
    let contact_side = (to_contact.x * normal.x + to_contact.y * normal.y).signum();
    let far_side = if contact_side.abs() <= EPSILON {
        -1.0
    } else {
        -contact_side
    };
    let half_width = stroke_width * 0.5;
    Some(LineGeometry {
        point: Point::new(
            center.shared.x + normal.x * half_width * far_side,
            center.shared.y + normal.y * half_width * far_side,
        ),
        direction: center.direction,
        shared: center.shared,
        length: center.length,
        offset_distance: half_width,
    })
}

fn bold_main_line_join_polygon(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5;
    let current = boundary_lines_from_endpoint(endpoint, forward, half_width)?;
    let base_plus = current[0].point;
    let base_minus = current[1].point;
    let mut best: Option<(Vec<Point>, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if !solid_joinable_main_line(other_bond) {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let Some(far_boundary) = main_line_far_boundary_for_wide_bond(
            object,
            node_map,
            other_bond,
            shared_node_id,
            Point::new(endpoint.x + forward.x, endpoint.y + forward.y),
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(plus_intersection) =
            line_intersection(base_plus, current[0].direction, far_boundary.point, far_boundary.direction)
        else {
            continue;
        };
        let Some(minus_intersection) =
            line_intersection(base_minus, current[1].direction, far_boundary.point, far_boundary.direction)
        else {
            continue;
        };
        let score = plus_intersection.distance(endpoint) + minus_intersection.distance(endpoint);
        let polygon = vec![base_plus, plus_intersection, minus_intersection, base_minus];
        if best.as_ref().is_none_or(|(_, best_score)| score < *best_score) {
            best = Some((polygon, score));
        }
    }
    best.map(|(polygon, _)| polygon)
}

#[allow(clippy::too_many_arguments)]
fn main_line_polygon_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    allow_start_join: bool,
    allow_end_join: bool,
    start_endpoint_profile: Option<Vec<Point>>,
    end_endpoint_profile: Option<Vec<Point>>,
) -> Option<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let half_width = stroke_width * 0.5;
    let mut start_lines = boundary_lines_from_endpoint(start, unit, half_width)?;
    let mut end_lines = boundary_lines_from_endpoint(end, Vector::new(-unit.x, -unit.y), half_width)?;

    if start_endpoint_profile.is_none() && allow_start_join {
        if let Some((join_plus, join_minus)) = main_line_join_points_against_wide_bonds(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            start_lines,
            stroke_width,
        ) {
            start_lines[0].point = join_plus;
            start_lines[1].point = join_minus;
        }
    }
    if end_endpoint_profile.is_none() && allow_end_join {
        if let Some((join_plus, join_minus)) = main_line_join_points_against_wide_bonds(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            end_lines,
            stroke_width,
        ) {
            end_lines[0].point = join_plus;
            end_lines[1].point = join_minus;
        }
    }

    let start_profile = endpoint_profile_global(
        start_endpoint_profile,
        false,
        vec![start_lines[0].point, start_lines[1].point],
    );
    let end_profile = endpoint_profile_global(
        end_endpoint_profile,
        true,
        vec![end_lines[1].point, end_lines[0].point],
    );

    Some(bond_polygon_from_endpoint_profiles(start_profile, end_profile))
}

fn is_centered_double_bond(bond: &Bond) -> bool {
    bond.order == 2 && side_double_placement(bond).is_none()
}

fn centered_double_line_weight_for_side(bond: &Bond, line_side: f64) -> BondLineWeight {
    if line_side > 0.0 {
        bond.line_weights.left
    } else {
        bond.line_weights.right
    }
}

fn centered_double_outer_line_boundary_pair_for_direction(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    reference_direction: Vector,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    if !is_centered_double_bond(bond) {
        return None;
    }
    let center = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    // Choose the centered-double child line by the bond's global axis rather than the
    // endpoint-local axis. At `bond.end`, the local axis points back into the bond, which
    // would flip the side test and make hash bonds / hashed wedges retreat to the wrong line.
    let axis = if shared_node_id == bond.begin {
        center.direction
    } else if shared_node_id == bond.end {
        Vector::new(-center.direction.x, -center.direction.y)
    } else {
        return None;
    };
    let line_side = main_contact_side(axis, reference_direction)?;
    centered_double_line_boundary_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        line_side,
        stroke_width,
        centered_double_line_weight_for_side(bond, line_side),
    )
    .map(|(lines, _)| lines)
}

fn endpoint_retreat_against_center_double_outer_line(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    half_width: f64,
    stroke_width: f64,
) -> Option<f64> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current = boundary_lines_from_endpoint(endpoint, forward, half_width)?;
    let mut best: Option<f64> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE));
        let Some(other) = centered_double_outer_line_boundary_pair_for_direction(
            object,
            node_map,
            other_bond,
            shared_node_id,
            forward,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some((points, _)) = extended_boundary_line_join_points(current, other) else {
            continue;
        };
        let retreat = points
            .into_iter()
            .zip(current.into_iter())
            .map(|(point, line)| {
                let delta = Vector::new(point.x - line.point.x, point.y - line.point.y);
                vector_dot(delta, line.direction).max(0.0)
            })
            .fold(0.0, f64::max);
        if retreat <= EPSILON {
            continue;
        }
        if best.is_none_or(|current_best| retreat < current_best) {
            best = Some(retreat);
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn main_bond_center_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    Some(LineGeometry {
        point: shared,
        direction,
        shared,
        length,
        offset_distance: 0.0,
    })
}

fn main_bond_cap_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let center_line = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    Some(LineGeometry {
        point: center_line.shared,
        direction: Vector::new(-center_line.direction.y, center_line.direction.x),
        shared: center_line.shared,
        length: center_line.length,
        offset_distance: 0.0,
    })
}

fn wide_boundary_lines_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Vec<LineGeometry> {
    let begin = match node_map.get(bond.begin.as_str()).copied() {
        Some(node) => world_point(object, node),
        None => return Vec::new(),
    };
    let end = match node_map.get(bond.end.as_str()).copied() {
        Some(node) => world_point(object, node),
        None => return Vec::new(),
    };
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let mut out = Vec::new();

    if bond.line_weights.main == BondLineWeight::Bold
        && bond.line_styles.main == BondLinePattern::Solid
        && bond.stereo.is_none()
    {
        let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5;
        let (shared, direction) = if shared_node_id == bond.begin {
            (begin, unit)
        } else if shared_node_id == bond.end {
            (end, Vector::new(-unit.x, -unit.y))
        } else {
            return Vec::new();
        };
        for side in [1.0, -1.0] {
            out.push(LineGeometry {
                point: Point::new(
                    shared.x + normal.x * half_width * side,
                    shared.y + normal.y * half_width * side,
                ),
                direction,
                shared,
                length,
                offset_distance: half_width,
            });
        }
        return out;
    }

    let Some(stereo_kind) = bond_stereo_kind(bond) else {
        return Vec::new();
    };
    let Some((tip_center, cap_center)) = (match stereo_kind {
        BondStereoKind::SolidWedgeEnd | BondStereoKind::HashedWedgeEnd if shared_node_id == bond.end => {
            Some((begin, end))
        }
        BondStereoKind::SolidWedgeBegin | BondStereoKind::HashedWedgeBegin if shared_node_id == bond.begin => {
            Some((end, begin))
        }
        _ => None,
    }) else {
        return Vec::new();
    };
    let cap_half_width = solid_wedge_half_width(stroke_width);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    for (cap_point, tip_point) in [
        (
            Point::new(
                cap_center.x + normal.x * cap_half_width,
                cap_center.y + normal.y * cap_half_width,
            ),
            Point::new(
                tip_center.x + normal.x * tip_half_width,
                tip_center.y + normal.y * tip_half_width,
            ),
        ),
        (
            Point::new(
                cap_center.x - normal.x * cap_half_width,
                cap_center.y - normal.y * cap_half_width,
            ),
            Point::new(
                tip_center.x - normal.x * tip_half_width,
                tip_center.y - normal.y * tip_half_width,
            ),
        ),
    ] {
        let direction = Vector::new(tip_point.x - cap_point.x, tip_point.y - cap_point.y);
        out.push(LineGeometry {
            point: cap_point,
            direction,
            shared: cap_center,
            length: direction.length(),
            offset_distance: cap_half_width,
        });
    }
    out
}

fn molecule_stroke(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "stroke"))
        .unwrap_or_else(|| CHEMCORE_INK.to_string())
}

fn bond_stroke_width(document: &ChemcoreDocument, object: &SceneObject, bond: &Bond) -> f64 {
    if bond.stroke_width > 0.0 {
        return bond.stroke_width;
    }
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_number(style, "strokeWidth").or_else(|| style_number(style, "stroke_width")))
        .unwrap_or(DEFAULT_BOND_STROKE)
}

fn style_string(style: &JsonValue, key: &str) -> Option<String> {
    style.get(key)?.as_str().map(ToString::to_string)
}

fn style_nullable_string(style: &JsonValue, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

fn style_number(style: &JsonValue, key: &str) -> Option<f64> {
    style.get(key)?.as_f64()
}

fn style_number_array(style: &JsonValue, key: &str) -> Option<Vec<f64>> {
    Some(
        style
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(JsonValue::as_f64)
            .collect(),
    )
}

fn payload_string(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload.extra.get(key)?.as_str().map(ToString::to_string)
}

fn payload_number(payload: &ObjectPayload, key: &str) -> Option<f64> {
    payload.extra.get(key)?.as_f64()
}

fn payload_bool(payload: &ObjectPayload, key: &str) -> Option<bool> {
    payload.extra.get(key)?.as_bool()
}

fn payload_points(payload: &ObjectPayload, key: &str) -> Vec<Point> {
    payload
        .extra
        .get(key)
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let coords = value.as_array()?;
            Some(Point::new(coords.first()?.as_f64()?, coords.get(1)?.as_f64()?))
        })
        .collect()
}

fn payload_box_width(payload: &ObjectPayload, key: &str) -> Option<f64> {
    let coords = payload.extra.get(key)?.as_array()?;
    coords.get(2)?.as_f64()
}

fn payload_runs(payload: &ObjectPayload, key: &str) -> Vec<LabelRun> {
    payload
        .extra
        .get(key)
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .unwrap_or_default()
}

fn payload_arrow_head(payload: &ObjectPayload, key: &str) -> Option<ArrowHeadGeometry> {
    let value = payload.extra.get(key)?;
    Some(ArrowHeadGeometry {
        length: value.get("length").and_then(JsonValue::as_f64).unwrap_or(8.0),
        width: value
            .get("width")
            .and_then(JsonValue::as_f64)
            .unwrap_or_else(|| value.get("length").and_then(JsonValue::as_f64).unwrap_or(8.0) * 0.55),
        head_full: value
            .get("head")
            .and_then(JsonValue::as_str)
            .is_some_and(|head| head.eq_ignore_ascii_case("full")),
    })
}

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label.font_size.unwrap_or(0.0).max(9.5);
    for run in &label.runs {
        size = size.max(run.font_size.unwrap_or(0.0));
    }
    size.max(9.5)
}

fn fragment_label_lines(label: &crate::NodeLabel) -> Vec<String> {
    if !label.lines.is_empty() {
        return label
            .lines
            .iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    if label.text.contains('\n') {
        return label
            .text
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if label.text.trim().is_empty() {
        Vec::new()
    } else {
        vec![label.text.clone()]
    }
}

fn fragment_label_runs_for_line(label: &crate::NodeLabel, index: usize, line: &str) -> Vec<LabelRun> {
    if let Some(line_runs) = label.line_runs.get(index) {
        return line_runs.clone();
    }
    if index == 0 && !label.runs.is_empty() && !label.text.contains('\n') && label.lines.is_empty() {
        return label.runs.clone();
    }
    vec![LabelRun {
        text: line.to_string(),
        font_family: label.font_family.clone(),
        font_size: label.font_size,
        fill: label.fill.clone(),
        font_weight: None,
        font_style: None,
        script: None,
        face: None,
    }]
}

fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

fn fragment_label_position_world(label: &crate::NodeLabel, object: &SceneObject) -> Point {
    let position = label.position.unwrap_or([0.0, 0.0]);
    Point::new(
        object.transform.translate[0] + position[0],
        object.transform.translate[1] + position[1],
    )
}

fn label_box_world(node: &Node, object: &SceneObject) -> Option<RectBox> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(RectBox {
        x1: bbox[0] + object.transform.translate[0],
        y1: bbox[1] + object.transform.translate[1],
        x2: bbox[2] + object.transform.translate[0],
        y2: bbox[3] + object.transform.translate[1],
    })
}

fn clip_point_out_of_box(
    start: Point,
    end: Point,
    rect: Option<RectBox>,
    margin: f64,
) -> Point {
    let Some(expanded) = rect.map(|box_value| box_value.expanded(margin)) else {
        return start;
    };
    if !expanded.contains(start) {
        return start;
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut candidates = Vec::new();
    if dx.abs() > EPSILON {
        for x in [expanded.x1, expanded.x2] {
            let t = (x - start.x) / dx;
            let y = start.y + dy * t;
            if (0.0..=1.0).contains(&t) && y >= expanded.y1 && y <= expanded.y2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    if dy.abs() > EPSILON {
        for y in [expanded.y1, expanded.y2] {
            let t = (y - start.y) / dy;
            let x = start.x + dx * t;
            if (0.0..=1.0).contains(&t) && x >= expanded.x1 && x <= expanded.x2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| point)
        .unwrap_or(start)
}

#[allow(clippy::too_many_arguments)]
fn render_fragment_line(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
) {
    render_fragment_line_with_profiles(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        start_box,
        end_box,
        allow_bold_contacts,
        stroke,
        stroke_width,
        dash_array,
        line_weight,
        object_id,
        true,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_fragment_line_with_profiles(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
    inherit_kernel_profiles: bool,
    start_endpoint_profile_override: Option<Vec<Point>>,
    end_endpoint_profile_override: Option<Vec<Point>>,
) {
    let clipped_start = clip_point_out_of_box(start, end, start_box, 0.8);
    let clipped_end = clip_point_out_of_box(end, clipped_start, end_box, 0.8);
    let mut start_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.begin);
    let mut end_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.end);
    if is_hash_bond(bond) && line_weight == BondLineWeight::Bold && !dash_array.is_empty() {
        let direction = Vector::new(clipped_end.x - clipped_start.x, clipped_end.y - clipped_start.y);
        if direction.length() > EPSILON {
            let unit = direction.normalized();
            let half_width = line_weight_stroke_width(stroke_width, line_weight) * 0.5;
            start_retreat = start_retreat.max(
                endpoint_retreat_against_center_double_outer_line(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    unit,
                    half_width,
                    stroke_width,
                )
                .unwrap_or(0.0),
            );
            end_retreat = end_retreat.max(
                endpoint_retreat_against_center_double_outer_line(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-unit.x, -unit.y),
                    half_width,
                    stroke_width,
                )
                .unwrap_or(0.0),
            );
        }
    }
    let (clipped_start, clipped_end) =
        apply_segment_endpoint_retreats(clipped_start, clipped_end, start_retreat, end_retreat);
    let mut start_endpoint_profile = start_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.begin)
        } else {
            None
        }
    });
    let mut end_endpoint_profile = end_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.end)
        } else {
            None
        }
    });
    if start_retreat > EPSILON {
        start_endpoint_profile = None;
    }
    if end_retreat > EPSILON {
        end_endpoint_profile = None;
    }
    let use_start_contact_kernel =
        contact_kernel.uses_endpoint(&bond.id, &bond.begin) || start_endpoint_profile.is_some();
    let use_end_contact_kernel =
        contact_kernel.uses_endpoint(&bond.id, &bond.end) || end_endpoint_profile.is_some();
    if line_weight == BondLineWeight::Normal && dash_array.is_empty() {
        let allow_main_line_join =
            is_joinable_main_line_render(bond, allow_bold_contacts, line_weight);
        if let Some(points) = main_line_polygon_points(
            object,
            bonds,
            node_map,
            bond,
            clipped_start,
            clipped_end,
            stroke_width,
            allow_main_line_join && !use_start_contact_kernel,
            allow_main_line_join && !use_end_contact_kernel,
            start_endpoint_profile.clone(),
            end_endpoint_profile.clone(),
        ) {
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
            return;
        }
    }
    if !dash_array.is_empty() {
        let polygon_points = if line_weight == BondLineWeight::Bold {
            Some(compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && !use_start_contact_kernel,
                allow_bold_contacts && !use_end_contact_kernel,
                start_endpoint_profile.clone(),
                end_endpoint_profile.clone(),
            ))
        } else {
            main_line_polygon_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                line_weight_stroke_width(stroke_width, line_weight),
                is_joinable_main_line_render(bond, allow_bold_contacts, line_weight) && !use_start_contact_kernel,
                is_joinable_main_line_render(bond, allow_bold_contacts, line_weight) && !use_end_contact_kernel,
                start_endpoint_profile.clone(),
                end_endpoint_profile.clone(),
            )
        };
        if let Some(points) = polygon_points {
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id.clone());
            let knockouts = if line_weight == BondLineWeight::Bold {
                hash_bond_knockout_polygons(
                    clipped_start,
                    clipped_end,
                    line_weight_stroke_width(stroke_width, line_weight),
                )
            } else {
                dashed_bond_knockout_polygons(
                    clipped_start,
                    clipped_end,
                    line_weight_stroke_width(stroke_width, line_weight),
                    &dash_array,
                )
            };
            for knockout in knockouts {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
            return;
        }
    }
    if line_weight == BondLineWeight::Bold && dash_array.is_empty() {
        let direction = Vector::new(clipped_end.x - clipped_start.x, clipped_end.y - clipped_start.y);
        if direction.length() > EPSILON {
            if !use_start_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    direction,
                    stroke_width,
                ) {
                    push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id.clone());
                }
            }
            if !use_end_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-direction.x, -direction.y),
                    stroke_width,
                ) {
                    push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id.clone());
                }
            }
        }
        push_bond_polygon(
            out,
            &bond.id,
            compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && !use_start_contact_kernel,
                allow_bold_contacts && !use_end_contact_kernel,
                start_endpoint_profile,
                end_endpoint_profile,
            ),
            stroke,
            stroke,
            0.0,
            object_id,
        );
        return;
    }
    push_bond_line(
        out,
        &bond.id,
        clipped_start,
        clipped_end,
        stroke,
        line_weight_stroke_width(stroke_width, line_weight),
        dash_array,
        object_id,
    );
}

fn push_line(
    out: &mut Vec<RenderPrimitive>,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Line {
        role,
        object_id,
        bond_id: None,
        from,
        to,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
    });
}

fn push_bond_line(
    out: &mut Vec<RenderPrimitive>,
    bond_id: &str,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Line {
        role: RenderRole::DocumentBond,
        object_id,
        bond_id: Some(bond_id.to_string()),
        from,
        to,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
    });
}

#[allow(clippy::too_many_arguments)]
fn push_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role,
        object_id,
        bond_id: None,
        points,
        fill: fill.to_string(),
        stroke: stroke.to_string(),
        stroke_width,
    });
}

fn push_bond_polygon(
    out: &mut Vec<RenderPrimitive>,
    bond_id: &str,
    points: Vec<Point>,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentBond,
        object_id,
        bond_id: Some(bond_id.to_string()),
        points,
        fill: fill.to_string(),
        stroke: stroke.to_string(),
        stroke_width,
    });
}

fn push_knockout_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    object_id: Option<String>,
) {
    let points = compact_polygon_points(points);
    if points.len() < 3 || polygon_area_signed(&points).abs() <= 1.0e-4 {
        return;
    }
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentKnockout,
        object_id,
        bond_id: None,
        points,
        fill: KNOCKOUT_FILL.to_string(),
        stroke: "none".to_string(),
        stroke_width: 0.0,
    });
}

fn push_polyline(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_cap: Option<String>,
    line_join: Option<String>,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polyline {
        role,
        object_id,
        bond_id: None,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
        line_cap,
        line_join,
    });
}

fn push_text(
    out: &mut Vec<RenderPrimitive>,
    x: f64,
    y: f64,
    text: String,
    font_size: f64,
    font_family: Option<String>,
    fill: Option<String>,
    text_anchor: Option<String>,
    runs: Vec<LabelRun>,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id,
        x,
        y,
        text,
        font_size,
        font_family,
        fill,
        text_anchor,
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs,
    });
}

fn arrow_shaft_end(from: Point, to: Point, arrow_head: ArrowHeadGeometry) -> Point {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let head_length = 5.4_f64.max(arrow_head.length * 0.6);
    let notch_length = 3.2_f64.max(head_length * 0.66);
    let center_length = notch_length.min(length * 0.8).max(0.0);
    Point::new(to.x - unit.x * center_length, to.y - unit.y * center_length)
}

fn arrow_head_points(from: Point, to: Point, arrow_head: ArrowHeadGeometry) -> Vec<Point> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let head_length = 5.4_f64.max(arrow_head.length * 0.6);
    let head_width = 4.8_f64.max(arrow_head.width * 1.16);
    let notch_length = 3.2_f64.max((head_length * 0.66).min(head_length - 0.8));
    let tip = to;
    let left = Point::new(
        to.x - unit.x * head_length + normal.x * (head_width / 2.0),
        to.y - unit.y * head_length + normal.y * (head_width / 2.0),
    );
    let right = Point::new(
        to.x - unit.x * head_length - normal.x * (head_width / 2.0),
        to.y - unit.y * head_length - normal.y * (head_width / 2.0),
    );
    if arrow_head.head_full && notch_length < head_length - 0.2 {
        let notch = Point::new(to.x - unit.x * notch_length, to.y - unit.y * notch_length);
        vec![tip, left, notch, right]
    } else {
        vec![tip, left, right]
    }
}

fn split_runs_by_line(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut out = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                out.last_mut().expect("line vector always exists").push(next_run);
            }
            if index + 1 < segments.len() {
                out.push(Vec::new());
            }
        }
    }
    out
}

fn split_preserved_text_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn wrap_text_lines(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let max_chars = (max_width / 6.0_f64.max(font_size * 0.6)).floor().max(8.0) as usize;
    let mut out = Vec::new();

    for raw_line in raw_lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() <= max_chars || !line.contains(' ') {
            out.push(line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let next = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if next.len() > max_chars && !current.is_empty() {
                out.push(current);
                current = word.to_string();
            } else {
                current = next;
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }

    out
}

fn unit_normal(start: Point, end: Point) -> (f64, f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy).max(1.0);
    (-dy / length, dx / length)
}

fn inset_bond_segment(start: Point, end: Point, inset_start: f64, inset_end: f64) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let clamped_start = inset_start.max(0.0).min(length * 0.45);
    let clamped_end = inset_end.max(0.0).min(length * 0.45);
    (
        Point::new(start.x + unit.x * clamped_start, start.y + unit.y * clamped_start),
        Point::new(end.x - unit.x * clamped_end, end.y - unit.y * clamped_end),
    )
}

fn line_weight_stroke_width(stroke_width: f64, line_weight: BondLineWeight) -> f64 {
    if line_weight == BondLineWeight::Bold {
        (stroke_width * BOLD_BOND_WIDTH_FACTOR).max(stroke_width + BOLD_BOND_MIN_EXTRA_WIDTH)
    } else {
        stroke_width
    }
}

fn bond_length_scale(start: Point, end: Point) -> f64 {
    (start.distance(end) / DEFAULT_BOND_LENGTH.max(EPSILON)).max(0.01)
}

fn double_bond_offset_distance(start: Point, end: Point, stroke_width: f64) -> f64 {
    DOUBLE_BOND_OFFSET * (stroke_width / VIEWER_BOND_STROKE) * bond_length_scale(start, end)
}

fn triple_bond_offset_distance(start: Point, end: Point, stroke_width: f64) -> f64 {
    TRIPLE_BOND_OFFSET * (stroke_width / VIEWER_BOND_STROKE) * bond_length_scale(start, end)
}

fn solid_wedge_half_width(stroke_width: f64) -> f64 {
    SOLID_WEDGE_HALF_WIDTH * (stroke_width / VIEWER_BOND_STROKE)
}

fn solid_wedge_tip_half_width(stroke_width: f64) -> f64 {
    stroke_width * 0.5
}

fn dash_gap_intervals(length: f64, dash_array: &[f64]) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let segments: Vec<f64> = dash_array.iter().copied().filter(|value| *value > EPSILON).collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let mut gap_intervals = Vec::new();
    let mut offset = 0.0;
    let mut gap_segment = false;
    let mut index = 0usize;
    while offset < length - EPSILON {
        let segment_length = segments[index % segments.len()];
        let next = (offset + segment_length).min(length);
        if gap_segment && next > offset + EPSILON {
            gap_intervals.push((offset, next));
        }
        offset += segment_length;
        gap_segment = !gap_segment;
        index += 1;
    }
    gap_intervals
}

fn equal_black_segment_gap_intervals(
    length: f64,
    start_offset: f64,
    end_inset: f64,
    stripe_length: f64,
    target_gap_length: f64,
) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let usable_start = start_offset.max(0.0);
    let usable_end = (length - end_inset).max(usable_start);
    let usable_length = usable_end - usable_start;
    let stripe_length = stripe_length.max(EPSILON);
    let target_gap_length = target_gap_length.max(EPSILON);
    if usable_length <= stripe_length + EPSILON {
        return Vec::new();
    }

    let mut stripe_count =
        ((usable_length + target_gap_length) / (stripe_length + target_gap_length)).round() as usize;
    stripe_count = stripe_count.max(2);
    while stripe_count > 1 && stripe_length * stripe_count as f64 > usable_length + EPSILON {
        stripe_count -= 1;
    }
    if stripe_count < 2 {
        return Vec::new();
    }

    let gap_count = stripe_count - 1;
    let total_gap_length = (usable_length - stripe_length * stripe_count as f64).max(0.0);
    let gap_length = total_gap_length / gap_count as f64;
    let mut intervals = Vec::with_capacity(gap_count);
    let mut cursor = usable_start + stripe_length;
    for index in 0..gap_count {
        let gap_start = cursor;
        let gap_end = if index + 1 == gap_count {
            usable_end - stripe_length
        } else {
            gap_start + gap_length
        };
        if gap_end > gap_start + EPSILON {
            intervals.push((gap_start, gap_end));
        }
        cursor = gap_end + stripe_length;
    }
    intervals
}

fn dashed_bond_knockout_polygons(
    start: Point,
    end: Point,
    stroke_width: f64,
    dash_array: &[f64],
) -> Vec<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = stroke_width * 0.5 + stroke_width.max(0.35) * 0.45;
    dash_gap_intervals(length, dash_array)
        .into_iter()
        .map(|(gap_start, gap_end)| {
            let segment_start = Point::new(start.x + unit.x * gap_start, start.y + unit.y * gap_start);
            let segment_end = Point::new(start.x + unit.x * gap_end, start.y + unit.y * gap_end);
            compact_polygon_points(vec![
                Point::new(
                    segment_start.x + normal.x * half_width,
                    segment_start.y + normal.y * half_width,
                ),
                Point::new(
                    segment_end.x + normal.x * half_width,
                    segment_end.y + normal.y * half_width,
                ),
                Point::new(
                    segment_end.x - normal.x * half_width,
                    segment_end.y - normal.y * half_width,
                ),
                Point::new(
                    segment_start.x - normal.x * half_width,
                    segment_start.y - normal.y * half_width,
                ),
            ])
        })
        .collect()
}

fn hash_bond_knockout_polygons(
    start: Point,
    end: Point,
    stroke_width: f64,
) -> Vec<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = stroke_width * 0.5 + stroke_width * 0.12;
    let scale = stroke_width / VIEWER_BOND_STROKE;
    equal_black_segment_gap_intervals(
        length,
        0.0,
        0.0,
        HASH_BLACK_SEGMENT_LENGTH * scale,
        HASH_TARGET_GAP_LENGTH * scale,
    )
    .into_iter()
    .map(|(gap_start, gap_end)| {
        let segment_start = Point::new(start.x + unit.x * gap_start, start.y + unit.y * gap_start);
        let segment_end = Point::new(start.x + unit.x * gap_end, start.y + unit.y * gap_end);
        compact_polygon_points(vec![
            Point::new(
                segment_start.x + normal.x * half_width,
                segment_start.y + normal.y * half_width,
            ),
            Point::new(
                segment_end.x + normal.x * half_width,
                segment_end.y + normal.y * half_width,
            ),
            Point::new(
                segment_end.x - normal.x * half_width,
                segment_end.y - normal.y * half_width,
            ),
            Point::new(
                segment_start.x - normal.x * half_width,
                segment_start.y - normal.y * half_width,
            ),
        ])
    })
    .collect()
}

fn hashed_wedge_gap_intervals(length: f64, stroke_width: f64) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let start_offset = (0.5 * scale).min(length * 0.06);
    let end_inset = (0.18 * scale).min(length * 0.03);
    equal_black_segment_gap_intervals(
        length,
        start_offset,
        end_inset,
        (HASH_BLACK_SEGMENT_LENGTH * scale).max(length * 0.014),
        (HASH_TARGET_GAP_LENGTH * scale).max(length * 0.018),
    )
}

fn side_double_placement(bond: &Bond) -> Option<DoubleBondPlacement> {
    match bond.double.as_ref().map(|double| double.placement) {
        Some(DoubleBondPlacement::Left) => Some(DoubleBondPlacement::Left),
        Some(DoubleBondPlacement::Right) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

fn line_pattern_dash_array(pattern: BondLinePattern) -> Vec<f64> {
    if pattern == BondLinePattern::Dashed {
        DASHED_BOND_PATTERN.to_vec()
    } else {
        Vec::new()
    }
}

fn outer_line_pattern(bond: &Bond, side: f64) -> BondLinePattern {
    if side > 0.0 {
        bond.line_styles.left
    } else {
        bond.line_styles.right
    }
}

fn outer_line_weight(bond: &Bond, side: f64) -> BondLineWeight {
    if side > 0.0 {
        bond.line_weights.left
    } else {
        bond.line_weights.right
    }
}

fn fragment_node_degree(bonds: &[Bond], node_id: &str) -> usize {
    bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .count()
}

fn fragment_outer_bond_offset_for_side(
    bond: &Bond,
    side: f64,
    stroke_width: f64,
    start: Point,
    end: Point,
) -> Option<f64> {
    if bond.order >= 3 {
        return Some(triple_bond_offset_distance(start, end, stroke_width));
    }
    let placement = side_double_placement(bond)?;
    if (placement == DoubleBondPlacement::Left && side > 0.0)
        || (placement == DoubleBondPlacement::Right && side < 0.0)
    {
        return Some(double_bond_offset_distance(start, end, stroke_width));
    }
    None
}

fn outer_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.order >= 3 {
        return vec![1.0, -1.0];
    }
    match side_double_placement(bond) {
        Some(DoubleBondPlacement::Left) => vec![1.0],
        Some(DoubleBondPlacement::Right) => vec![-1.0],
        _ => Vec::new(),
    }
}

fn outer_bond_half_width_for_side(bond: &Bond, side: f64, stroke_width: f64) -> f64 {
    line_weight_stroke_width(stroke_width, outer_line_weight(bond, side)) * 0.5
}

fn outer_bond_offset_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let offset_distance = fragment_outer_bond_offset_for_side(bond, side, stroke_width, begin, end)?;
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    Some(LineGeometry {
        point: Point::new(
            shared.x + normal.x * offset_distance * side,
            shared.y + normal.y * offset_distance * side,
        ),
        direction,
        shared,
        length,
        offset_distance,
    })
}

fn outer_bond_boundary_line_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    let center = outer_bond_offset_line_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        side,
        stroke_width,
    )?;
    let half_width = outer_bond_half_width_for_side(bond, side, stroke_width);
    let boundaries = boundary_lines_from_endpoint(center.point, center.direction, half_width)?;
    Some((boundaries, center))
}

fn centered_double_line_boundary_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    line_side: f64,
    stroke_width: f64,
    line_weight: BondLineWeight,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let offset_distance = double_bond_offset_distance(begin, end, stroke_width) * 0.5;
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else if shared_node_id == bond.end {
        (end, Vector::new(-unit.x, -unit.y))
    } else {
        return None;
    };
    let center = LineGeometry {
        point: Point::new(
            shared.x + normal.x * offset_distance * line_side,
            shared.y + normal.y * offset_distance * line_side,
        ),
        direction,
        shared,
        length,
        offset_distance,
    };
    let half_width = line_weight_stroke_width(stroke_width, line_weight) * 0.5;
    let boundaries = boundary_lines_from_endpoint(center.point, center.direction, half_width)?;
    Some((boundaries, center))
}

fn boundary_lines_with_profile_terminals(
    mut lines: [LineGeometry; 2],
    profile: &[Point],
) -> [LineGeometry; 2] {
    if profile.len() < 2 {
        return lines;
    }
    let terminals = [profile[0], *profile.last().unwrap_or(&profile[0])];
    let direct =
        terminals[0].distance(lines[0].point) + terminals[1].distance(lines[1].point);
    let swapped =
        terminals[0].distance(lines[1].point) + terminals[1].distance(lines[0].point);
    if direct <= swapped {
        lines[0].point = terminals[0];
        lines[1].point = terminals[1];
    } else {
        lines[0].point = terminals[1];
        lines[1].point = terminals[0];
    }
    lines
}

fn main_bond_drawn_boundary_pair_for_endpoint(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    if is_hash_contact_obstacle(bond) {
        return None;
    }
    let geometry = main_bond_endpoint_geometry(object, node_map, bond, shared_node_id, stroke_width)?;
    let mut lines = [
        LineGeometry {
            point: geometry.base_plus,
            direction: geometry.contour_plus.direction,
            shared: geometry.center,
            length: geometry.contour_plus.extent,
            offset_distance: geometry.contour_plus.half_width,
        },
        LineGeometry {
            point: geometry.base_minus,
            direction: geometry.contour_minus.direction,
            shared: geometry.center,
            length: geometry.contour_minus.extent,
            offset_distance: geometry.contour_minus.half_width,
        },
    ];
    if let Some(profile) = contact_kernel.endpoint_profile(&bond.id, shared_node_id) {
        lines = boundary_lines_with_profile_terminals(lines, &profile);
    }
    Some((
        lines,
        LineGeometry {
            point: geometry.center,
            direction: geometry.axis,
            shared: geometry.center,
            length: geometry
                .contour_plus
                .extent
                .max(geometry.contour_minus.extent)
                .max(1.0),
            offset_distance: 0.0,
        },
    ))
}

fn outer_bond_drawn_boundary_pairs_for_endpoint(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Vec<([LineGeometry; 2], LineGeometry)> {
    let mut out = Vec::new();
    for side in outer_bond_candidate_sides(bond) {
        let Some((mut lines, center)) = outer_bond_boundary_line_pair_for_endpoint(
            object,
            node_map,
            bond,
            shared_node_id,
            side,
            stroke_width,
        ) else {
            continue;
        };
        if let Some(profile) = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            shared_node_id,
            side,
            stroke_width,
        ) {
            lines = boundary_lines_with_profile_terminals(lines, &profile);
        }
        out.push((lines, center));
    }
    out
}

fn main_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.line_weights.main == BondLineWeight::Bold && bond.line_styles.main == BondLinePattern::Solid
    {
        vec![1.0, -1.0]
    } else {
        vec![0.0]
    }
}

fn main_bond_boundary_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    let offset_distance = if side == 0.0 {
        stroke_width * 0.5
    } else {
        line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5
    };
    let point = if side == 0.0 {
        far_side_contact_line_point(shared, direction, if shared_node_id == bond.begin { end } else { begin }, stroke_width)
    } else {
        Point::new(
            shared.x + normal.x * offset_distance * side,
            shared.y + normal.y * offset_distance * side,
        )
    };
    Some(LineGeometry {
        point,
        direction,
        shared,
        length,
        offset_distance,
    })
}

fn outer_bond_endpoint_profile_for_side(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    if outer_line_pattern(bond, side) != BondLinePattern::Solid {
        return None;
    }

    let current_stroke_width = stroke_width.max(bond.stroke_width.max(VIEWER_BOND_STROKE));
    let (current, current_center) = outer_bond_boundary_line_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        side,
        current_stroke_width,
    )?;
    let current_local_side = line_geometry_local_side(current_center)?;

    let mut best: Option<([Point; 2], f64)> = None;
    let mut straight_through_profile = false;
    let mut acute_retreat = false;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let mut same_side_outer_candidate = false;
        for other_side in outer_bond_candidate_sides(other_bond) {
            if outer_line_pattern(other_bond, other_side) != BondLinePattern::Solid {
                continue;
            }
            let Some((other, other_center)) = outer_bond_boundary_line_pair_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                other_stroke_width,
            ) else {
                continue;
            };
            if main_contact_is_straight_through(current_center.direction, other_center.direction) {
                if side_double_placement(bond).is_some() {
                    straight_through_profile = true;
                }
                continue;
            }
            if side_double_placement(bond).is_some()
                && !line_geometries_share_side(current_center, other_center)
            {
                continue;
            }
            same_side_outer_candidate = true;
            let Some(candidate) = extended_boundary_line_join_points(current, other) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, best_score)| candidate.1 < *best_score) {
                best = Some(candidate);
            }
        }
        if side_double_placement(bond).is_some()
            && !same_side_outer_candidate
            && side_double_outer_line_requires_acute_retreat(
                object,
                node_map,
                other_bond,
                shared_node_id,
                current_center,
                current_local_side,
            )
        {
            acute_retreat = true;
        }
    }

    if acute_retreat && best.is_none() {
        return None;
    }
    if let Some((points, _)) = best {
        return Some(compact_polygon_points(vec![points[0], points[1]]));
    }
    if straight_through_profile {
        return Some(compact_polygon_points(vec![current[0].point, current[1].point]));
    }
    None
}

fn line_geometry_local_side(line: LineGeometry) -> Option<f64> {
    let normal = Vector::new(-line.direction.y, line.direction.x);
    let offset = Vector::new(line.point.x - line.shared.x, line.point.y - line.shared.y);
    let local_side = vector_dot(offset, normal).signum();
    if local_side.abs() <= EPSILON {
        None
    } else {
        Some(local_side)
    }
}

fn line_geometries_share_side(first: LineGeometry, second: LineGeometry) -> bool {
    let first_offset = Vector::new(first.point.x - first.shared.x, first.point.y - first.shared.y);
    let second_offset = Vector::new(second.point.x - second.shared.x, second.point.y - second.shared.y);
    vector_dot(first_offset, second_offset) > EPSILON
}

fn side_double_outer_line_requires_acute_retreat(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    other_bond: &Bond,
    shared_node_id: &str,
    current_center: LineGeometry,
    current_local_side: f64,
) -> bool {
    let Some(other_axis) = bond_axis_line_for_endpoint(object, node_map, other_bond, shared_node_id) else {
        return false;
    };
    let Some(contact_side) = main_contact_side(current_center.direction, other_axis.direction) else {
        return false;
    };
    (contact_side - current_local_side).abs() <= 1.0e-6
        && bond_ray_is_acute(current_center.direction, other_axis.direction)
}

fn bond_axis_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else if shared_node_id == bond.end {
        (end, Vector::new(-unit.x, -unit.y))
    } else {
        return None;
    };
    Some(LineGeometry {
        point: shared,
        direction,
        shared,
        length,
        offset_distance: 0.0,
    })
}

fn center_double_endpoint_profile_for_line_side(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    line_side: f64,
    stroke_width: f64,
    line_weight: BondLineWeight,
) -> Option<Vec<Point>> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }

    let current_stroke_width = stroke_width.max(bond.stroke_width.max(VIEWER_BOND_STROKE));
    let (current, current_center) = centered_double_line_boundary_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        line_side,
        current_stroke_width,
        line_weight,
    )?;
    let current_local_side = line_geometry_local_side(current_center)?;

    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let mut candidates = Vec::new();
        if let Some(candidate) = main_bond_drawn_boundary_pair_for_endpoint(
            object,
            contact_kernel,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) {
            candidates.push(candidate);
        }
        candidates.extend(outer_bond_drawn_boundary_pairs_for_endpoint(
            object,
            bonds,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ));

        for (other, other_center) in candidates {
            if center_double_skips_extension(current_center.direction, other_center.direction) {
                continue;
            }
            let Some(contact_side) = main_contact_side(current_center.direction, other_center.direction) else {
                continue;
            };
            if (contact_side - current_local_side).abs() > 1.0e-6 {
                continue;
            }
            let Some(candidate) = extended_boundary_line_join_points(current, other) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, best_score)| candidate.1 < *best_score) {
                best = Some(candidate);
            }
        }
    }

    best.map(|(points, _)| compact_polygon_points(vec![points[0], points[1]]))
}

fn line_intersection(point: Point, direction: Vector, other_point: Point, other_direction: Vector) -> Option<Point> {
    line_intersection_with_parameters(point, direction, other_point, other_direction).map(|value| value.0)
}

fn line_intersection_with_parameters(
    point: Point,
    direction: Vector,
    other_point: Point,
    other_direction: Vector,
) -> Option<(Point, f64, f64)> {
    let cross = direction.x * other_direction.y - direction.y * other_direction.x;
    if cross.abs() < 1.0e-6 {
        return None;
    }
    let dx = other_point.x - point.x;
    let dy = other_point.y - point.y;
    let t = (dx * other_direction.y - dy * other_direction.x) / cross;
    let u = (dx * direction.y - dy * direction.x) / cross;
    Some((
        Point::new(point.x + direction.x * t, point.y + direction.y * t),
        t,
        u,
    ))
}

#[derive(Debug, Clone, Copy)]
struct ContactEntry {
    direction: Vector,
    side: f64,
    side_value: f64,
}

fn contact_entries(directions: &[Vector], normal: Vector) -> Vec<ContactEntry> {
    directions
        .iter()
        .filter_map(|direction| {
            let unit = direction.normalized();
            let side_value = normal.x * unit.x + normal.y * unit.y;
            let side = side_value.signum();
            if side.abs() <= EPSILON {
                None
            } else {
                Some(ContactEntry {
                    direction: unit,
                    side,
                    side_value,
                })
            }
        })
        .collect()
}

fn far_side_contact_line_point(
    contact_point: Point,
    contact_direction: Vector,
    interior_point: Point,
    stroke_width: f64,
) -> Point {
    let normal = Vector::new(-contact_direction.y, contact_direction.x);
    let to_interior = Vector::new(interior_point.x - contact_point.x, interior_point.y - contact_point.y);
    let interior_side = (to_interior.x * normal.x + to_interior.y * normal.y).signum();
    let offset = stroke_width * 0.55;
    Point::new(
        contact_point.x - normal.x * if interior_side == 0.0 { 1.0 } else { interior_side } * offset,
        contact_point.y - normal.y * if interior_side == 0.0 { 1.0 } else { interior_side } * offset,
    )
}

fn wide_contact_directions(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    wide_node_id: &str,
) -> Vec<Vector> {
    let Some(wide_node) = node_map.get(wide_node_id).copied() else {
        return Vec::new();
    };
    if wide_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return Vec::new();
    }
    let wide_point = world_point(object, wide_node);
    let mut out = Vec::new();
    for other_bond in bonds {
        if other_bond.id == bond.id || !is_wide_contact_candidate(other_bond) {
            continue;
        }
        if other_bond.begin != wide_node_id && other_bond.end != wide_node_id {
            continue;
        }
        let other_node_id = if other_bond.begin == wide_node_id {
            other_bond.end.as_str()
        } else {
            other_bond.begin.as_str()
        };
        let Some(other_node) = node_map.get(other_node_id).copied() else {
            continue;
        };
        let other_point = world_point(object, other_node);
        let vector = Vector::new(other_point.x - wide_point.x, other_point.y - wide_point.y);
        if vector.length() > 1.0e-6 {
            out.push(vector);
        }
    }
    out
}

fn is_wide_contact_candidate(bond: &Bond) -> bool {
    if bond_stereo_kind(bond).is_some() {
        return false;
    }
    if bond.order == 1 {
        return true;
    }
    matches!(
        side_double_placement(bond),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    )
}

#[derive(Debug, Clone, Copy)]
enum BondStereoKind {
    SolidWedgeBegin,
    SolidWedgeEnd,
    HashedWedgeBegin,
    HashedWedgeEnd,
}

fn bond_stereo_kind(bond: &Bond) -> Option<BondStereoKind> {
    if let Some(stereo) = bond.stereo.as_ref() {
        return match (stereo.kind.as_str(), stereo.wide_end.as_str()) {
            ("solid-wedge", "begin") => Some(BondStereoKind::SolidWedgeBegin),
            ("solid-wedge", "end") => Some(BondStereoKind::SolidWedgeEnd),
            ("hashed-wedge", "begin") => Some(BondStereoKind::HashedWedgeBegin),
            ("hashed-wedge", "end") => Some(BondStereoKind::HashedWedgeEnd),
            _ => None,
        };
    }
    let display = bond
        .meta
        .pointer("/import/cdxml/display")
        .and_then(JsonValue::as_str)?;
    match display {
        "WedgeBegin" => Some(BondStereoKind::SolidWedgeEnd),
        "WedgeEnd" => Some(BondStereoKind::SolidWedgeBegin),
        "WedgedHashBegin" => Some(BondStereoKind::HashedWedgeEnd),
        "WedgedHashEnd" => Some(BondStereoKind::HashedWedgeBegin),
        _ => None,
    }
}
