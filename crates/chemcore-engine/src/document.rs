use crate::{
    round2, Point, DEFAULT_BOND_LENGTH_CM, DEFAULT_BOND_STROKE_CM,
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM, DEFAULT_PAGE_HEIGHT_CM, DEFAULT_PAGE_WIDTH_CM,
    DEFAULT_TEXT_BLOCK_PADDING_CM, PT_PER_CM,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const DEFAULT_PAGE_WIDTH: f64 = DEFAULT_PAGE_WIDTH_CM;
pub const DEFAULT_PAGE_HEIGHT: f64 = DEFAULT_PAGE_HEIGHT_CM;
pub const DEFAULT_BOND_LENGTH: f64 = DEFAULT_BOND_LENGTH_CM;
pub const DEFAULT_BOND_STROKE: f64 = DEFAULT_BOND_STROKE_CM;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemcoreDocument {
    pub format: FormatInfo,
    pub document: DocumentInfo,
    #[serde(default)]
    pub styles: BTreeMap<String, Value>,
    #[serde(default)]
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
}

impl ChemcoreDocument {
    pub fn blank() -> Self {
        let mut styles = BTreeMap::new();
        styles.insert(
            "style_molecule_default".to_string(),
            json!({
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM
            }),
        );
        styles.insert(
            "style_arrow_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "lineCap": "butt",
                "lineJoin": "miter"
            }),
        );

        let mut resources = BTreeMap::new();
        resources.insert(
            "mol_editor".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment::blank()),
                meta: Value::Null,
            },
        );

        Self {
            format: FormatInfo {
                name: "chemcore".to_string(),
                version: "0.1".to_string(),
                unit: "pt".to_string(),
            },
            document: DocumentInfo {
                id: "doc_editor_untitled".to_string(),
                title: "Untitled".to_string(),
                page: Page {
                    width: DEFAULT_PAGE_WIDTH,
                    height: DEFAULT_PAGE_HEIGHT,
                    background: "#ffffff".to_string(),
                },
                meta: Value::Null,
            },
            styles,
            objects: vec![SceneObject {
                id: "obj_editor_molecule".to_string(),
                object_type: "molecule".to_string(),
                name: "molecule".to_string(),
                visible: true,
                locked: false,
                z_index: 10,
                transform: Transform::identity(),
                style_ref: Some("style_molecule_default".to_string()),
                meta: Value::Null,
                payload: ObjectPayload {
                    resource_ref: Some("mol_editor".to_string()),
                    bbox: Some([0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT]),
                    extra: BTreeMap::new(),
                },
            }],
            resources,
        }
    }

    pub fn editable_fragment_mut(&mut self) -> Option<EditableFragmentMut<'_>> {
        let object_index = self
            .objects
            .iter()
            .position(|object| object.object_type == "molecule")?;
        let resource_ref = self.objects[object_index].payload.resource_ref.clone()?;
        let resource = self.resources.get_mut(&resource_ref)?;
        let fragment = resource.data.as_fragment_mut()?;
        Some(EditableFragmentMut {
            object: &mut self.objects[object_index],
            fragment,
        })
    }

    pub fn editable_fragment(&self) -> Option<EditableFragment<'_>> {
        let object = self
            .objects
            .iter()
            .find(|object| object.object_type == "molecule")?;
        let resource_ref = object.payload.resource_ref.as_ref()?;
        let resource = self.resources.get(resource_ref)?;
        let fragment = resource.data.as_fragment()?;
        Some(EditableFragment { object, fragment })
    }
}

pub fn parse_document_json(json: &str) -> Result<ChemcoreDocument, String> {
    let mut value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if document_json_uses_legacy_cm(&value) {
        scale_document_json_value(&mut value, PT_PER_CM);
    }
    ensure_document_json_unit(&mut value);
    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn document_json_uses_legacy_cm(value: &Value) -> bool {
    let unit = value
        .get("format")
        .and_then(|format| format.get("unit"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if unit.eq_ignore_ascii_case("cm") {
        return true;
    }
    if !unit.is_empty() {
        return false;
    }

    let width = value
        .get("document")
        .and_then(|document| document.get("page"))
        .and_then(|page| page.get("width"))
        .and_then(Value::as_f64);
    let height = value
        .get("document")
        .and_then(|document| document.get("page"))
        .and_then(|page| page.get("height"))
        .and_then(Value::as_f64);
    matches!((width, height), (Some(width), Some(height)) if width <= 100.0 && height <= 100.0)
}

fn ensure_document_json_unit(value: &mut Value) {
    if !value.is_object() {
        return;
    }
    let Some(format) = value.get_mut("format").and_then(Value::as_object_mut) else {
        return;
    };
    format.insert("unit".to_string(), Value::String("pt".to_string()));
}

fn scale_document_json_value(value: &mut Value, factor: f64) {
    scale_json_value_by_key("", value, factor);
}

fn scale_json_value_by_key(key: &str, value: &mut Value, factor: f64) {
    if scale_key_as_length_scalar(key) {
        scale_all_numbers(value, factor);
        return;
    }
    match value {
        Value::Array(items) if scale_key_as_length_array(key) => {
            for item in items {
                scale_all_numbers(item, factor);
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_json_value_by_key("", item, factor);
            }
        }
        Value::Object(object) => {
            for (child_key, child_value) in object {
                scale_json_value_by_key(child_key, child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_all_numbers(value: &mut Value, factor: f64) {
    match value {
        Value::Number(number) => {
            if let Some(scaled) = number
                .as_f64()
                .and_then(|value| serde_json::Number::from_f64(value * factor))
            {
                *number = scaled;
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_all_numbers(item, factor);
            }
        }
        Value::Object(object) => {
            for child_value in object.values_mut() {
                scale_all_numbers(child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_key_as_length_scalar(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "x"
            | "y"
            | "strokeWidth"
            | "fontSize"
            | "lineHeight"
            | "wrapWidth"
            | "pad"
            | "padding"
    )
}

fn scale_key_as_length_array(key: &str) -> bool {
    matches!(
        key,
        "bbox"
            | "box"
            | "boxField"
            | "position"
            | "translate"
            | "points"
            | "anchorOffset"
            | "glyphPolygons"
    )
}

fn default_format_unit() -> String {
    "pt".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub name: String,
    pub version: String,
    #[serde(default = "default_format_unit")]
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub page: Page,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_ref: Option<String>,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub payload: ObjectPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub translate: [f64; 2],
    pub rotate: f64,
    pub scale: [f64; 2],
}

impl Transform {
    pub const fn identity() -> Self {
        Self {
            translate: [0.0, 0.0],
            rotate: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub encoding: String,
    pub data: ResourceData,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceData {
    Fragment(MoleculeFragment),
    Text(String),
    Json(Value),
}

impl ResourceData {
    pub fn as_fragment(&self) -> Option<&MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_fragment_mut(&mut self) -> Option<&mut MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFragment {
    pub schema: String,
    pub bbox: [f64; 4],
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub bonds: Vec<Bond>,
    #[serde(default)]
    pub meta: Value,
}

impl MoleculeFragment {
    pub fn blank() -> Self {
        Self {
            schema: "chemcore.molecule.fragment2d".to_string(),
            bbox: [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT],
            nodes: Vec::new(),
            bonds: Vec::new(),
            meta: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub element: String,
    pub atomic_number: u8,
    pub position: [f64; 2],
    pub charge: i32,
    pub num_hydrogens: u8,
    #[serde(default)]
    pub is_external_connection_point: bool,
    #[serde(default)]
    pub is_placeholder: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<NodeLabel>,
    #[serde(default)]
    pub meta: Value,
}

impl Node {
    pub fn carbon(id: String, point: Point) -> Self {
        Self {
            id,
            element: "C".to_string(),
            atomic_number: 6,
            position: [round2(point.x), round2(point.y)],
            charge: 0,
            num_hydrogens: 0,
            is_external_connection_point: false,
            is_placeholder: false,
            label: None,
            meta: Value::Null,
        }
    }

    pub fn point(&self) -> Point {
        Point::new(self.position[0], self.position[1])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLabel {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_field: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_runs: Vec<Vec<LabelRun>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyph_polygons: Vec<Vec<[f64; 2]>>,
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default)]
    pub meta: Value,
}

impl NodeLabel {
    pub fn bbox(&self) -> Option<[f64; 4]> {
        self.box_value.or(self.box_field)
    }

    pub fn has_visible_text(&self) -> bool {
        !self.text.trim().is_empty()
    }

    pub fn glyph_polygons(&self) -> Vec<Vec<Point>> {
        self.glyph_polygons
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelRun {
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bond {
    pub id: String,
    pub begin: String,
    pub end: String,
    pub order: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double: Option<DoubleBond>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereo: Option<BondStereo>,
    #[serde(default)]
    pub stroke_width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wedge_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_clip_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_spacing: Option<f64>,
    #[serde(default)]
    pub line_styles: BondLineStyles,
    #[serde(default)]
    pub line_weights: BondLineWeights,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleBond {
    pub placement: DoubleBondPlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_exit_side: Option<DoubleBondPlacement>,
    #[serde(default)]
    pub frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineStyles {
    #[serde(default)]
    pub main: BondLinePattern,
    #[serde(default)]
    pub left: BondLinePattern,
    #[serde(default)]
    pub right: BondLinePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineWeights {
    #[serde(default)]
    pub main: BondLineWeight,
    #[serde(default)]
    pub left: BondLineWeight,
    #[serde(default)]
    pub right: BondLineWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLinePattern {
    #[default]
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLineWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondStereo {
    pub kind: String,
    pub wide_end: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DoubleBondPlacement {
    Left,
    Right,
    Center,
}

pub struct EditableFragment<'a> {
    pub object: &'a SceneObject,
    pub fragment: &'a MoleculeFragment,
}

impl EditableFragment<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }
}

pub struct EditableFragmentMut<'a> {
    pub object: &'a mut SceneObject,
    pub fragment: &'a mut MoleculeFragment,
}

impl EditableFragmentMut<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }

    pub fn local_point(&self, point: Point) -> Point {
        Point::new(
            point.x - self.object.transform.translate[0],
            point.y - self.object.transform.translate[1],
        )
    }

    pub fn update_bounds(&mut self) {
        let mut max_x = DEFAULT_PAGE_WIDTH;
        let mut max_y = DEFAULT_PAGE_HEIGHT;
        for node in &self.fragment.nodes {
            max_x = max_x.max(node.position[0] + DEFAULT_TEXT_BLOCK_PADDING_CM);
            max_y = max_y.max(node.position[1] + DEFAULT_TEXT_BLOCK_PADDING_CM);
        }
        self.fragment.bbox = [0.0, 0.0, round2(max_x), round2(max_y)];
        self.object.payload.bbox = Some(self.fragment.bbox);
    }
}
