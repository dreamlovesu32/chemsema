use super::*;
use crate::Point;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

const CHEMDRAW_AUTO_BRACKET_LABEL_GAP_EM: f64 = 0.1875;
const MAX_EMBEDDED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_EMBEDDED_IMAGE_DIMENSION_PX: u32 = 32_768;
const MAX_EMBEDDED_IMAGE_PIXELS: u64 = 100_000_000;

pub(super) fn append_embedded_image_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    resources: &mut BTreeMap<String, Resource>,
) {
    let mut index = 1usize;
    for node in descendants(root) {
        if !node.is("embeddedobject") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(bounds) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let raster = [
            ("PNG", "image/png"),
            ("JPEG", "image/jpeg"),
            ("GIF", "image/gif"),
            ("BMP", "image/bmp"),
        ]
        .into_iter()
        .find_map(|(attribute, mime_type)| {
            let bytes = crate::decode_hex_bytes(node.attr(attribute)?)?;
            let (pixel_width, pixel_height) = raster_pixel_dimensions(mime_type, &bytes)?;
            if bytes.is_empty()
                || bytes.len() > MAX_EMBEDDED_IMAGE_BYTES
                || pixel_width == 0
                || pixel_height == 0
                || pixel_width > MAX_EMBEDDED_IMAGE_DIMENSION_PX
                || pixel_height > MAX_EMBEDDED_IMAGE_DIMENSION_PX
                || u64::from(pixel_width) * u64::from(pixel_height) > MAX_EMBEDDED_IMAGE_PIXELS
            {
                return None;
            }
            Some((attribute, mime_type, bytes, pixel_width, pixel_height))
        });
        let opaque = raster
            .is_none()
            .then(|| {
                [
                    "TIFF",
                    "EnhancedMetafile",
                    "CompressedEnhancedMetafile",
                    "WindowsMetafile",
                    "CompressedWindowsMetafile",
                    "OLEObject",
                    "CompressedOLEObject",
                    "PDF",
                    "MacPICT",
                ]
                .into_iter()
                .find_map(|attribute| {
                    crate::decode_hex_bytes(node.attr(attribute)?)
                        .filter(|bytes| !bytes.is_empty())
                        .map(|bytes| (attribute, bytes))
                })
            })
            .flatten();
        if raster.is_none() && opaque.is_none() {
            continue;
        }
        let width = (bounds[2] - bounds[0]).abs();
        let height = (bounds[3] - bounds[1]).abs();
        if width <= crate::EPSILON || height <= crate::EPSILON {
            continue;
        }
        let attribute = raster
            .as_ref()
            .map(|(attribute, _, _, _, _)| *attribute)
            .or_else(|| opaque.as_ref().map(|(attribute, _)| *attribute))
            .unwrap_or("EmbeddedObject");
        let resource_id = format!("image_cdxml_{index:03}");
        let object_id = format!("obj_image_{index:03}");
        let (resource_type, data) =
            if let Some((_, mime_type, bytes, pixel_width, pixel_height)) = raster {
                let image = crate::ImageResourceData {
                    mime_type: mime_type.to_string(),
                    data_base64: BASE64.encode(&bytes),
                    pixel_width,
                    pixel_height,
                    source_name: None,
                };
                let Ok(data) = serde_json::to_value(image) else {
                    continue;
                };
                ("image", data)
            } else {
                let (_, bytes) = opaque.expect("opaque embedded payload was checked above");
                (
                    "embedded-object",
                    json!({ "format": attribute, "dataBase64": BASE64.encode(bytes) }),
                )
            };
        resources.insert(
            resource_id.clone(),
            Resource {
                resource_type: resource_type.to_string(),
                encoding: "base64".to_string(),
                data: ResourceData::Json(data),
                meta: json!({
                    "import": { "cdxml": { "id": node.attr("id"), "attribute": attribute } }
                }),
            },
        );
        objects.push(SceneObject {
            id: object_id,
            object_type: "image".to_string(),
            name: if resource_type == "image" {
                "embedded image"
            } else {
                "embedded object"
            }
            .to_string(),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(0),
            transform: Transform {
                translate: [round2(bounds[0]), round2(bounds[1])],
                rotate: parse_f64(node.attr("RotationAngle")).unwrap_or(0.0),
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "kind": "image",
                "import": { "cdxml": { "id": node.attr("id"), "attribute": attribute } }
            }),
            payload: ObjectPayload {
                resource_ref: Some(resource_id),
                bbox: Some([0.0, 0.0, round2(width), round2(height)]),
                extra: BTreeMap::from([
                    ("fit".to_string(), json!("stretch")),
                    ("opacity".to_string(), json!(1.0)),
                ]),
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn raster_pixel_dimensions(mime_type: &str, bytes: &[u8]) -> Option<(u32, u32)> {
    match mime_type {
        "image/png" if bytes.get(..8) == Some(b"\x89PNG\r\n\x1a\n") => Some((
            u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),
            u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?),
        )),
        "image/gif"
            if bytes
                .get(..6)
                .is_some_and(|header| header == b"GIF87a" || header == b"GIF89a") =>
        {
            Some((
                u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as u32,
                u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?) as u32,
            ))
        }
        "image/bmp" if bytes.get(..2) == Some(b"BM") => Some((
            u32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?),
            i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?).unsigned_abs(),
        )),
        "image/jpeg" => jpeg_pixel_dimensions(bytes),
        _ => None,
    }
}

fn jpeg_pixel_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..2) != Some(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if matches!(marker, 0xd8 | 0xd9) || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        if length < 2 || offset + length > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            let height =
                u16::from_be_bytes(bytes.get(offset + 3..offset + 5)?.try_into().ok()?) as u32;
            let width =
                u16::from_be_bytes(bytes.get(offset + 5..offset + 7)?.try_into().ok()?) as u32;
            return (width > 0 && height > 0).then_some((width, height));
        }
        offset += length;
    }
    None
}

fn non_bond_dash_array(defaults: CdxmlDefaults) -> Value {
    json!([defaults
        .hash_spacing
        .max(crate::DEFAULT_HASH_SPACING_PT.value() * 0.25)])
}

pub(super) fn append_curve_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("curve") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(points) = parse_cdxml_curve_points(node.attr("CurvePoints")) else {
            continue;
        };
        let curve_type = parse_i32(node.attr("CurveType")).unwrap_or(0);
        let dashed = curve_type & 0x0002 != 0
            || node
                .attr("LineType")
                .is_some_and(|value| value.contains("Dashed"));
        let bold = curve_type & 0x0004 != 0
            || node
                .attr("LineType")
                .is_some_and(|value| value.contains("Bold"));
        let stroke = colors.resolve(node.attr("color"));
        let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
            defaults.bold_width
        } else {
            defaults.line_width
        });
        let style_id = format!("style_curve_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "stroke",
                "stroke": stroke,
                "strokeWidth": stroke_width,
                "lineCap": "butt",
                "lineJoin": "round",
                "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
            }),
        );

        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("bezier-curve"));
        extra.insert("curvePoints".to_string(), json!(points));
        extra.insert("curveType".to_string(), json!(curve_type));
        extra.insert("closed".to_string(), json!(curve_type & 0x0001 != 0));
        let explicit_head = node.attr("ArrowheadHead").map(canonical_arrow_endpoint);
        let explicit_tail = node.attr("ArrowheadTail").map(canonical_arrow_endpoint);
        extra.insert(
            "head".to_string(),
            json!(explicit_head.unwrap_or(if curve_type & 0x0008 != 0 {
                "full"
            } else if curve_type & 0x0020 != 0 {
                "half"
            } else {
                "none"
            })),
        );
        extra.insert(
            "tail".to_string(),
            json!(explicit_tail.unwrap_or(if curve_type & 0x0010 != 0 {
                "full"
            } else if curve_type & 0x0040 != 0 {
                "half"
            } else {
                "none"
            })),
        );
        extra.insert(
            "arrowheadType".to_string(),
            json!(node.attr("ArrowheadType").unwrap_or("Solid")),
        );
        extra.insert(
            "headLength".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("HeadSize")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO,
            )),
        );
        extra.insert(
            "headCenterLength".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("ArrowheadCenterSize")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875,
            )),
        );
        extra.insert(
            "headWidth".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("ArrowheadWidth")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25,
            )),
        );
        let (min_x, min_y, max_x, max_y) = points.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, min_y, max_x, max_y), point| {
                (
                    min_x.min(point[0]),
                    min_y.min(point[1]),
                    max_x.max(point[0]),
                    max_y.max(point[1]),
                )
            },
        );
        objects.push(SceneObject {
            id: format!("obj_curve_{index:03}"),
            object_type: "curve".to_string(),
            name: format!("curve {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(18),
            transform: Transform::identity(),
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "curveId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([min_x, min_y, max_x - min_x, max_y - min_y]),
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn parse_cdxml_curve_points(value: Option<&str>) -> Option<Vec<[f64; 2]>> {
    let values: Vec<_> = value?
        .split_whitespace()
        .filter_map(|value| value.parse::<f64>().ok())
        .collect();
    if values.len() < 12 || values.len() % 2 != 0 {
        return None;
    }
    let points: Vec<_> = values
        .chunks_exact(2)
        .map(|pair| [pair[0], pair[1]])
        .collect();
    // ChemDraw stores one endpoint guide on each side of the drawable
    // spline.  The body is points[1..len-1]: P0 followed by C1, C2, P for
    // every cubic segment.  The two outer guides determine endpoint tangent
    // and arrow geometry but are not part of the stroked path.
    ((points.len() - 3) % 3 == 0).then_some(points)
}

pub(super) fn append_line_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !(node.is("arrow")
            || (node.is("graphic") && matches!(node.attr("GraphicType"), Some("Line" | "Arc"))))
        {
            continue;
        }
        if node.attr("SupersededBy").is_some() {
            continue;
        }
        // CDXML permits plain line graphics (and legacy line-shaped arrows) to
        // store their two endpoints only in BoundingBox. For these objects the
        // first pair is the head and the second pair is the tail; unlike a
        // rectangular extent, their authored order must be preserved.
        let legacy_arc = cdxml_legacy_arc_geometry(node);
        let bbox_endpoints = cdxml_line_bbox_endpoints(node.attr("BoundingBox"));
        let head = parse_xyz2(node.attr("Head3D"))
            .or_else(|| legacy_arc.map(|geometry| geometry.head))
            .or_else(|| bbox_endpoints.map(|points| points.0));
        let tail = parse_xyz2(node.attr("Tail3D"))
            .or_else(|| legacy_arc.map(|geometry| geometry.tail))
            .or_else(|| bbox_endpoints.map(|points| points.1));
        let (Some(tail), Some(head)) = (tail, head) else {
            continue;
        };
        let is_arrow = node.is("arrow") || has_arrow_attrs(node);
        let line_type = node.attr("LineType").unwrap_or("");
        let bold = line_type.contains("Bold");
        let head_endpoint = cdxml_arrow_endpoint(node, true);
        let tail_endpoint = cdxml_arrow_endpoint(node, false);
        let head_enabled = head_endpoint != "none";
        let tail_enabled = tail_endpoint != "none";
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("line"));
        extra.insert(
            "points".to_string(),
            json!([
                [round2(tail[0]), round2(tail[1])],
                [round2(head[0]), round2(head[1])]
            ]),
        );
        if is_arrow {
            let mut arrow_head = BTreeMap::new();
            let cdxml_kind = cdxml_arrow_kind(node);
            arrow_head.insert("kind".to_string(), json!(cdxml_kind));
            arrow_head.insert("head".to_string(), json!(head_endpoint));
            arrow_head.insert("tail".to_string(), json!(tail_endpoint));
            if let Some(fill_type) = node.attr("FillType").map(canonical_arrow_fill_type) {
                arrow_head.insert("fillType".to_string(), json!(fill_type));
            }
            arrow_head.insert(
                "length".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("HeadSize")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO
                )),
            );
            arrow_head.insert(
                "centerLength".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("ArrowheadCenterSize")).or_else(|| {
                        (!matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium"))
                            .then(|| parse_scaled_100(node.attr("ArrowShaftSpacing")))
                            .flatten()
                    }),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875
                )),
            );
            arrow_head.insert(
                "width".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("ArrowheadWidth")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25
                )),
            );
            if !matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium") {
                if let Some(shaft_spacing) = parse_scaled_100(node.attr("ArrowShaftSpacing")) {
                    arrow_head.insert("shaftSpacing".to_string(), json!(shaft_spacing));
                }
            }
            if matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium") {
                arrow_head.insert(
                    "shaftSpacing".to_string(),
                    json!(cdxml_arrow_size_for_render_scale(
                        parse_scaled_100(node.attr("ArrowShaftSpacing")),
                        3.0
                    )),
                );
                if let Some(ratio) = parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
                    .filter(|value| *value > 1.0)
                {
                    arrow_head.insert("equilibriumRatio".to_string(), json!(ratio));
                }
            }
            if let Some(curve) =
                parse_f64(node.attr("AngularSize")).filter(|value| value.abs() > crate::EPSILON)
            {
                arrow_head.insert("curve".to_string(), json!(curve));
            }
            if let Some(no_go) = node
                .attr("NoGo")
                .map(str::trim)
                .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("None"))
            {
                arrow_head.insert("noGo".to_string(), json!(no_go.to_ascii_lowercase()));
            } else if legacy_arrow_type_has(node, "NoGo") {
                arrow_head.insert("noGo".to_string(), json!("cross"));
            }
            if node.attr("Dipole").is_some_and(cdxml_boolean_enabled)
                || legacy_arrow_type_has(node, "Dipole")
            {
                arrow_head.insert("dipole".to_string(), json!(true));
            }
            if let Some(curve_spacing) = parse_scaled_100(node.attr("CurveSpacing")) {
                arrow_head.insert("curveSpacing".to_string(), json!(curve_spacing));
            }
            if node.attr("Closed").is_some_and(cdxml_boolean_enabled) {
                arrow_head.insert("closed".to_string(), json!(true));
            }
            if let Some(source) = node.attr("ArrowSource") {
                arrow_head.insert("source".to_string(), json!(source));
            }
            if let Some(target) = node.attr("ArrowTarget") {
                arrow_head.insert("target".to_string(), json!(target));
            }
            if bold {
                arrow_head.insert("bold".to_string(), json!(true));
            }
            let mut arrow_geometry = BTreeMap::new();
            if legacy_arc.is_none() {
                if let Some(bbox) = parse_bbox(node.attr("BoundingBox")) {
                    arrow_geometry.insert(
                        "boundingBox".to_string(),
                        json!([
                            round2(bbox[0]),
                            round2(bbox[1]),
                            round2(bbox[2]),
                            round2(bbox[3])
                        ]),
                    );
                }
            }
            if let Some(center) = parse_xyz2(node.attr("Center3D")) {
                arrow_geometry.insert(
                    "center".to_string(),
                    json!([round2(center[0]), round2(center[1])]),
                );
            } else if let Some(geometry) = legacy_arc {
                arrow_geometry.insert(
                    "center".to_string(),
                    json!([round2(geometry.center[0]), round2(geometry.center[1])]),
                );
                arrow_geometry.insert(
                    "majorAxisEnd".to_string(),
                    json!([
                        round2(geometry.center[0] + geometry.radius),
                        round2(geometry.center[1])
                    ]),
                );
                arrow_geometry.insert(
                    "minorAxisEnd".to_string(),
                    json!([
                        round2(geometry.center[0]),
                        round2(geometry.center[1] + geometry.radius)
                    ]),
                );
            }
            if let Some(major) = parse_xyz2(node.attr("MajorAxisEnd3D")) {
                arrow_geometry.insert(
                    "majorAxisEnd".to_string(),
                    json!([round2(major[0]), round2(major[1])]),
                );
            }
            if let Some(minor) = parse_xyz2(node.attr("MinorAxisEnd3D")) {
                arrow_geometry.insert(
                    "minorAxisEnd".to_string(),
                    json!([round2(minor[0]), round2(minor[1])]),
                );
            }
            if !arrow_geometry.is_empty() {
                extra.insert("arrowGeometry".to_string(), json!(arrow_geometry));
            }
            extra.insert(
                "head".to_string(),
                json!(if head_enabled { "end" } else { "none" }),
            );
            extra.insert(
                "tail".to_string(),
                json!(if tail_enabled { "start" } else { "none" }),
            );
            extra.insert("arrowHead".to_string(), json!(arrow_head));
        }
        let style_ref = cdxml_line_style_ref(node, is_arrow, styles, defaults, colors);
        objects.push(SceneObject {
            id: format!("obj_line_{index:03}"),
            object_type: "line".to_string(),
            name: format!("line {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(if is_arrow { 20 } else { 18 }),
            transform: Transform::identity(),
            style_ref: Some(style_ref),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "import": {"cdxml": {"kind": if is_arrow { "arrow" } else { "line" }, "lineType": empty_as_null(node.attr("LineType"))}}}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn cdxml_line_bbox_endpoints(value: Option<&str>) -> Option<([f64; 2], [f64; 2])> {
    let mut parts = value?.split_whitespace();
    let head = [parts.next()?.parse().ok()?, parts.next()?.parse().ok()?];
    let tail = [parts.next()?.parse().ok()?, parts.next()?.parse().ok()?];
    Some((head, tail))
}

#[derive(Clone, Copy)]
struct LegacyArcGeometry {
    head: [f64; 2],
    tail: [f64; 2],
    center: [f64; 2],
    radius: f64,
}

fn cdxml_legacy_arc_geometry(node: &XmlNode) -> Option<LegacyArcGeometry> {
    if !node.is("graphic") || node.attr("GraphicType") != Some("Arc") {
        return None;
    }
    // For legacy Arc graphics ChemDraw stores the authored head followed by
    // the circle center in BoundingBox. AngularSize is the signed sweep from
    // the head to the tail (screen coordinates, positive clockwise).
    let (head, center) = cdxml_line_bbox_endpoints(node.attr("BoundingBox"))?;
    let sweep = parse_f64(node.attr("AngularSize"))?.to_radians();
    let delta_x = head[0] - center[0];
    let delta_y = head[1] - center[1];
    let radius = delta_x.hypot(delta_y);
    if radius <= crate::EPSILON {
        return None;
    }
    let tail = [
        center[0] + delta_x * sweep.cos() - delta_y * sweep.sin(),
        center[1] + delta_x * sweep.sin() + delta_y * sweep.cos(),
    ];
    Some(LegacyArcGeometry {
        head,
        tail,
        center,
        radius,
    })
}

fn cdxml_arrow_kind(node: &XmlNode) -> &'static str {
    if !has_modern_arrow_fields(node) {
        if legacy_arrow_type_has(node, "Equilibrium") {
            return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
                .is_some_and(|value| value > 1.0)
            {
                "unequal-equilibrium"
            } else {
                "equilibrium"
            };
        }
        if legacy_arrow_type_has(node, "Hollow") {
            return "hollow";
        }
        if legacy_arrow_type_has(node, "RetroSynthetic") {
            return "open";
        }
    }
    let explicit_kind = node
        .attr("ArrowheadType")
        .or_else(|| node.attr("ArrowType"))
        .unwrap_or("Solid")
        .to_ascii_lowercase();
    if explicit_kind == "equilibrium" {
        return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
            .is_some_and(|value| value > 1.0)
        {
            "unequal-equilibrium"
        } else {
            "equilibrium"
        };
    }
    let head = canonical_arrow_endpoint(node.attr("ArrowheadHead").unwrap_or("None"));
    let tail = canonical_arrow_endpoint(node.attr("ArrowheadTail").unwrap_or("None"));
    let has_equilibrium_spacing = node.attr("ArrowShaftSpacing").is_some();
    if explicit_kind == "solid"
        && has_equilibrium_spacing
        && head != "none"
        && head == tail
        && matches!(head, "half-left" | "half-right")
    {
        return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
            .is_some_and(|value| value > 1.0)
        {
            "unequal-equilibrium"
        } else {
            "equilibrium"
        };
    }
    match explicit_kind.as_str() {
        "hollow" => "hollow",
        "angle" | "open" | "retrosynthetic" => "open",
        _ => "solid",
    }
}

fn legacy_arrow_type_has(node: &XmlNode, expected: &str) -> bool {
    node.attr("ArrowType").is_some_and(|value| {
        value
            .split_whitespace()
            .any(|token| token.eq_ignore_ascii_case(expected))
    })
}

fn has_modern_arrow_fields(node: &XmlNode) -> bool {
    ["ArrowheadHead", "ArrowheadTail", "ArrowheadType"]
        .into_iter()
        .any(|name| node.attr(name).is_some())
}

fn cdxml_boolean_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

fn cdxml_arrow_endpoint(node: &XmlNode, at_head: bool) -> &'static str {
    let modern_name = if at_head {
        "ArrowheadHead"
    } else {
        "ArrowheadTail"
    };
    if let Some(value) = node.attr(modern_name) {
        return canonical_arrow_endpoint(value);
    }
    if has_modern_arrow_fields(node) {
        return "none";
    }

    if legacy_arrow_type_has(node, "Equilibrium") {
        return "half-left";
    }
    if legacy_arrow_type_has(node, "Resonance") {
        return "full";
    }
    if !at_head {
        return "none";
    }
    if legacy_arrow_type_has(node, "HalfHead") {
        return if parse_f64(node.attr("HeadSize")).is_some_and(|value| value < 0.0) {
            "half-right"
        } else {
            "half-left"
        };
    }
    if ["FullHead", "Hollow", "RetroSynthetic", "Dipole"]
        .into_iter()
        .any(|kind| legacy_arrow_type_has(node, kind))
    {
        return "full";
    }
    "none"
}

fn canonical_arrow_endpoint(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "full" | "fullhead" => "full",
        "halfleft" | "half-left" | "left" | "top" => "half-left",
        "halfright" | "half-right" | "right" | "bottom" => "half-right",
        _ => "none",
    }
}

fn canonical_arrow_fill_type(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "none" => "none",
        "solid" => "solid",
        "shaded" => "shaded",
        _ => "unknown",
    }
}

fn cdxml_arrow_size_for_render_scale(value: Option<f64>, default_value: f64) -> f64 {
    value.unwrap_or(default_value)
}

fn cdxml_line_style_ref(
    node: &XmlNode,
    is_arrow: bool,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) -> String {
    let line_type = node.attr("LineType").unwrap_or("");
    let bold = line_type.contains("Bold");
    let dashed = line_type.contains("Dashed");
    let color = colors.resolve(node.attr("color"));
    let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
        defaults.bold_width
    } else {
        defaults.line_width
    });
    let default_stroke_width = if bold {
        defaults.bold_width
    } else {
        defaults.line_width
    };
    let base = if is_arrow { "arrow" } else { "line" };
    let default_width = (stroke_width - default_stroke_width).abs() <= crate::EPSILON;
    if !bold && !dashed && color == "#000000" && default_width {
        return format!("style_{base}_default");
    }
    let style_id = format!(
        "style_{base}_{}{}{}{}",
        if bold { "bold" } else { "regular" },
        if dashed { "_dashed" } else { "" },
        if default_width {
            String::new()
        } else {
            format!("_w{}", cdxml_style_number(stroke_width))
        },
        if color == "#000000" {
            String::new()
        } else {
            format!("_{}", color.trim_start_matches('#'))
        }
    );
    styles.entry(style_id.clone()).or_insert_with(|| {
        // ChemDraw emits CDXML GraphicType="Line" shafts as square-ended
        // geometry, just like arrow shafts.  This is independent of whether
        // LineType contains Dashed or Bold; round caps are a native editor
        // default and must not be introduced at the CDXML import boundary.
        let line_cap = "butt";
        let line_join = "miter";
        json!({
            "kind": "stroke",
            "stroke": color,
            "strokeWidth": stroke_width,
            "lineCap": line_cap,
            "lineJoin": line_join,
            "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
        })
    });
    style_id
}

fn cdxml_style_number(value: f64) -> String {
    let mut text = format!("{:.3}", value.abs());
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    let text = text.replace('.', "p");
    if value < 0.0 {
        format!("m{text}")
    } else {
        text
    }
}

pub(super) fn append_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let graphic_type = node.attr("GraphicType").unwrap_or("");
        if !matches!(graphic_type, "Rectangle" | "Oval") {
            continue;
        }
        let Some(raw_bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let bbox = [
            raw_bbox[0].min(raw_bbox[2]),
            raw_bbox[1].min(raw_bbox[3]),
            raw_bbox[0].max(raw_bbox[2]),
            raw_bbox[1].max(raw_bbox[3]),
        ];
        let type_value = node
            .attr(if graphic_type == "Rectangle" {
                "RectangleType"
            } else {
                "OvalType"
            })
            .unwrap_or("");
        let color = colors.resolve(node.attr("color"));
        let filled = type_value.contains("Filled");
        let shaded = type_value.contains("Shaded");
        let shadow = type_value.contains("Shadow");
        let line_type = node.attr("LineType").unwrap_or("");
        let dashed = type_value.contains("Dashed") || line_type.contains("Dashed");
        let bold = type_value.contains("Bold") || line_type.contains("Bold");
        let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
            defaults.bold_width
        } else {
            defaults.line_width
        });
        let shadow_size = parse_scaled_100(node.attr("ShadowSize")).unwrap_or(4.0);
        let style_id = format!("style_shape_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": if filled || shaded { json!(color) } else { Value::Null },
                "stroke": if filled { Value::Null } else { json!(color) },
                "strokeWidth": if filled { 0.0 } else { stroke_width },
                "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
                "shaded": if shaded { json!(true) } else { Value::Null },
                "shadow": if shadow { json!(true) } else { Value::Null },
                "shadowSize": if shadow { json!(shadow_size) } else { Value::Null },
            }),
        );
        let (transform, payload) = if graphic_type == "Oval" {
            let axes = match (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) {
                (Some(center), Some(major), Some(minor)) => Some((center, major, minor)),
                // Older CDX circle graphics use the two ordered BoundingBox
                // points as a radial endpoint followed by the center.  The
                // official BoundingBox documentation explicitly says Graphic
                // objects overload the rectangle as a pair of defining points.
                // Preserve that representation when the later 3D-axis fields
                // are absent.
                _ if type_value.contains("Circle") => {
                    let center = [raw_bbox[2], raw_bbox[3]];
                    let major = [raw_bbox[0], raw_bbox[1]];
                    let dx = major[0] - center[0];
                    let dy = major[1] - center[1];
                    (dx.hypot(dy) > crate::EPSILON).then_some((
                        center,
                        major,
                        [center[0] - dy, center[1] + dx],
                    ))
                }
                _ => None,
            };
            let Some((center, major, minor)) = axes else {
                continue;
            };
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("Circle") {
                    "circle"
                } else {
                    "ellipse"
                }),
            );
            extra.insert(
                "center".to_string(),
                json!([round2(center[0]), round2(center[1])]),
            );
            extra.insert(
                "majorAxisEnd".to_string(),
                json!([round2(major[0]), round2(major[1])]),
            );
            extra.insert(
                "minorAxisEnd".to_string(),
                json!([round2(minor[0]), round2(minor[1])]),
            );
            (
                Transform::identity(),
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        round2(bbox[0]),
                        round2(bbox[1]),
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        } else {
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("RoundEdge") {
                    "roundRect"
                } else {
                    "rect"
                }),
            );
            extra.insert(
                "cornerRadius".to_string(),
                json!(parse_scaled_100(node.attr("CornerRadius")).unwrap_or(0.0)),
            );
            (
                Transform {
                    translate: [round2(bbox[0]), round2(bbox[1])],
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        0.0,
                        0.0,
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        };
        objects.push(SceneObject {
            id: format!("obj_shape_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("shape {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform,
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
            payload,
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn append_orbital_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("graphic")
            || node.attr("SupersededBy").is_some()
            || node.attr("GraphicType") != Some("Orbital")
        {
            continue;
        }
        let Some(orbital_type) = node.attr("OrbitalType") else {
            continue;
        };
        let Some((template, style, phase)) = cdxml_orbital_family(orbital_type) else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_orbital_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": if style == "hollow" { Value::Null } else { json!(color.clone()) },
                "stroke": if style == "filled" { Value::Null } else { json!(color.clone()) },
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
                "shaded": if style == "shaded" { json!(true) } else { Value::Null },
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("orbital"));
        extra.insert("orbitalTemplate".to_string(), json!(template));
        extra.insert("orbitalStyle".to_string(), json!(style));
        extra.insert("orbitalPhase".to_string(), json!(phase));
        extra.insert("orbitalColor".to_string(), json!(color.clone()));

        let (transform, payload_bbox) = if matches!(template, "s" | "oval") {
            let (Some(center), Some(major), Some(minor)) = (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) else {
                continue;
            };
            extra.insert(
                "center".to_string(),
                json!([round2(center[0]), round2(center[1])]),
            );
            extra.insert(
                "majorAxisEnd".to_string(),
                json!([round2(major[0]), round2(major[1])]),
            );
            extra.insert(
                "minorAxisEnd".to_string(),
                json!([round2(minor[0]), round2(minor[1])]),
            );
            let rx = Point::new(center[0], center[1]).distance(Point::new(major[0], major[1]));
            let ry = Point::new(center[0], center[1]).distance(Point::new(minor[0], minor[1]));
            let bbox = [
                center[0] - rx,
                center[1] - ry,
                center[0] + rx,
                center[1] + ry,
            ];
            (
                Transform::identity(),
                Some([
                    round2(bbox[0].min(bbox[2])),
                    round2(bbox[1].min(bbox[3])),
                    round2((bbox[2] - bbox[0]).abs()),
                    round2((bbox[3] - bbox[1]).abs()),
                ]),
            )
        } else {
            let Some((anchor, tip)) = parse_orbital_axis_points(node.attr("BoundingBox")) else {
                continue;
            };
            extra.insert(
                "axisStart".to_string(),
                json!([round2(anchor[0]), round2(anchor[1])]),
            );
            extra.insert(
                "axisEnd".to_string(),
                json!([round2(tip[0]), round2(tip[1])]),
            );
            let padding = ((Point::new(anchor[0], anchor[1]).distance(Point::new(tip[0], tip[1]))
                * 0.75)
                .max(defaults.bond_length * 0.25))
            .max(6.0);
            let min_x = anchor[0].min(tip[0]) - padding;
            let min_y = anchor[1].min(tip[1]) - padding;
            let max_x = anchor[0].max(tip[0]) + padding;
            let max_y = anchor[1].max(tip[1]) + padding;
            (
                Transform::identity(),
                Some([
                    round2(min_x),
                    round2(min_y),
                    round2(max_x - min_x),
                    round2(max_y - min_y),
                ]),
            )
        };

        objects.push(SceneObject {
            id: format!("obj_shape_orbital_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("orbital {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform,
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "orbitalType": orbital_type}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: payload_bbox,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn parse_orbital_axis_points(value: Option<&str>) -> Option<([f64; 2], [f64; 2])> {
    let nums: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    if nums.len() != 4 {
        return None;
    }
    Some(([nums[2], nums[3]], [nums[0], nums[1]]))
}

fn cdxml_orbital_family(value: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match value {
        "s" => Some(("s", "hollow", "plus")),
        "sShaded" => Some(("s", "shaded", "plus")),
        "sFilled" => Some(("s", "filled", "plus")),
        "p" => Some(("p", "shaded", "plus")),
        "pFilled" => Some(("p", "filled", "plus")),
        "dxy" => Some(("dxy", "shaded", "plus")),
        "dxyFilled" => Some(("dxy", "filled", "plus")),
        "oval" => Some(("oval", "hollow", "plus")),
        "ovalShaded" => Some(("oval", "shaded", "plus")),
        "ovalFilled" => Some(("oval", "filled", "plus")),
        "hybridMinus" => Some(("hybrid", "shaded", "minus")),
        "hybridMinusFilled" => Some(("hybrid", "filled", "minus")),
        "hybridPlus" => Some(("hybrid", "shaded", "plus")),
        "hybridPlusFilled" => Some(("hybrid", "filled", "plus")),
        "dz2Minus" => Some(("dz2", "shaded", "minus")),
        "dz2MinusFilled" => Some(("dz2", "filled", "minus")),
        "dz2Plus" => Some(("dz2", "shaded", "plus")),
        "dz2PlusFilled" => Some(("dz2", "filled", "plus")),
        "lobe" => Some(("lobe", "hollow", "plus")),
        "lobeShaded" => Some(("lobe", "shaded", "plus")),
        "lobeFilled" => Some(("lobe", "filled", "plus")),
        _ => None,
    }
}

pub(super) fn append_table_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("table") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_table_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": Value::Null,
                "stroke": color,
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("crossTable"));
        objects.push(SceneObject {
            id: format!("obj_shape_table_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("table shape {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "tableId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn append_tlc_plate_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("tlcplate") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let corners = [
            parse_xyz2(node.attr("TopLeft")),
            parse_xyz2(node.attr("TopRight")),
            parse_xyz2(node.attr("BottomRight")),
            parse_xyz2(node.attr("BottomLeft")),
        ];
        let plate_bbox = corners
            .iter()
            .flatten()
            .fold(None, |acc: Option<[f64; 4]>, point| {
                Some(match acc {
                    Some([left, top, right, bottom]) => [
                        left.min(point[0]),
                        top.min(point[1]),
                        right.max(point[0]),
                        bottom.max(point[1]),
                    ],
                    None => [point[0], point[1], point[0], point[1]],
                })
            })
            .or_else(|| parse_bbox(node.attr("BoundingBox")));
        let Some(bbox) = plate_bbox else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_tlc_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": "#ffffff",
                "stroke": color,
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
            }),
        );
        let lanes_xml: Vec<_> = node
            .children
            .iter()
            .filter(|child| child.is("tlclane"))
            .collect();
        let lane_count = lanes_xml.len().max(1);
        let lanes: Vec<_> = lanes_xml
            .iter()
            .enumerate()
            .map(|(lane_index, lane)| {
                let spots: Vec<_> = lane
                    .children
                    .iter()
                    .filter(|child| child.is("tlcspot"))
                    .map(|spot| {
                        let mut json_spot = serde_json::Map::new();
                        json_spot.insert(
                            "rf".to_string(),
                            json!(round2(parse_f64(spot.attr("Rf")).unwrap_or(0.15))),
                        );
                        if let Some(width) = parse_f64(spot.attr("Width")) {
                            json_spot.insert(
                                "width".to_string(),
                                json!(round2(normalize_tlc_spot_extent(width))),
                            );
                        }
                        if let Some(height) = parse_f64(spot.attr("Height")) {
                            json_spot.insert(
                                "height".to_string(),
                                json!(round2(normalize_tlc_spot_extent(height))),
                            );
                        }
                        if let Some(curve_type) = parse_i32(spot.attr("CurveType")) {
                            json_spot.insert("curveType".to_string(), json!(curve_type));
                        }
                        if let Some(tail) = parse_f64(spot.attr("Tail")) {
                            json_spot.insert("tail".to_string(), json!(tail));
                        }
                        Value::Object(json_spot)
                    })
                    .collect();
                json!({
                    "offset": round2((lane_index as f64 + 1.0) / (lane_count as f64 + 1.0)),
                    "spots": spots,
                })
            })
            .collect();
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("tlcPlate"));
        extra.insert(
            "originFraction".to_string(),
            json!(round2(
                parse_f64(node.attr("OriginFraction")).unwrap_or(0.1)
            )),
        );
        extra.insert(
            "solventFrontFraction".to_string(),
            json!(round2(
                parse_f64(node.attr("SolventFrontFraction")).unwrap_or(0.1)
            )),
        );
        extra.insert(
            "showOrigin".to_string(),
            json!(node
                .attr("ShowOrigin")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showSolventFront".to_string(),
            json!(node
                .attr("ShowSolventFront")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showBorders".to_string(),
            json!(node
                .attr("ShowBorders")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showSideTicks".to_string(),
            json!(node
                .attr("ShowSideTicks")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "dashSpacing".to_string(),
            json!(round2(defaults.hash_spacing)),
        );
        extra.insert("lanes".to_string(), json!(lanes));
        objects.push(SceneObject {
            id: format!("obj_shape_tlc_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("tlc plate {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "tlcPlateId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn normalize_tlc_spot_extent(raw: f64) -> f64 {
    if raw.abs() > 1024.0 {
        raw / 65536.0
    } else {
        raw
    }
}

#[derive(Clone)]
struct PendingCdxmlBracket {
    kind: String,
    bbox: [f64; 4],
    z_index: i32,
    graphic_id: Option<String>,
    repeat_count: Option<u32>,
    stroke: String,
    stroke_width: f64,
    lip_size: i16,
}

pub(super) fn append_bracket_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut brackets = Vec::new();
    let mut symbol_index = 1;
    let repeat_counts = bracket_repeat_counts_by_graphic_id(root);
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        match node.attr("GraphicType").unwrap_or("") {
            "Bracket" => {
                let Some(bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                let graphic_id = node.attr("id").map(ToString::to_string);
                let repeat_count = graphic_id
                    .as_deref()
                    .and_then(|id| repeat_counts.get(id).copied())
                    .or_else(|| bracket_usage_count(node));
                brackets.push(PendingCdxmlBracket {
                    kind: match node.attr("BracketType").unwrap_or("") {
                        "Square" => "square",
                        "Curly" => "curly",
                        _ => "round",
                    }
                    .to_string(),
                    bbox,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    graphic_id,
                    repeat_count,
                    stroke: colors.resolve(node.attr("color")),
                    stroke_width: parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width),
                    lip_size: parse_i32(node.attr("LipSize"))
                        .and_then(|value| i16::try_from(value).ok())
                        .unwrap_or(60),
                });
            }
            "Symbol" => {
                let symbol_type = node.attr("SymbolType").unwrap_or("");
                let Some(kind) = cdxml_symbol_kind(symbol_type) else {
                    continue;
                };
                let Some(raw_bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                let style = crate::cdxml_symbol_style_from_line_width(defaults.line_width);
                let metrics =
                    crate::cdxml_symbol_metrics_from_bbox(kind, raw_bbox, defaults.line_width);
                let (width, height) = (metrics.width, metrics.height);
                let [cx, cy] = cdxml_symbol_center(kind, raw_bbox);
                let fill = colors.resolve(node.attr("color"));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!(kind));
                extra.insert("fill".to_string(), json!(fill));
                extra.insert(
                    "symbolStyle".to_string(),
                    json!(crate::cdxml_symbol_style_name(style)),
                );
                extra.insert(
                    "symbolAnchorWidth".to_string(),
                    json!(metrics.cdxml_anchor_width),
                );
                extra.insert(
                    "symbolAnchorHeight".to_string(),
                    json!(metrics.cdxml_anchor_height),
                );
                extra.insert("symbolLineWidth".to_string(), json!(metrics.line_width));
                extra.insert("cdxmlBoundingBox".to_string(), json!(raw_bbox));
                if let Some(stroke_width) = metrics.stroke_width {
                    extra.insert("strokeWidth".to_string(), json!(stroke_width));
                }
                if let Some(attribute) = node
                    .direct_children("represent")
                    .find_map(|represent| represent.attr("attribute"))
                {
                    extra.insert("representAttribute".to_string(), json!(attribute));
                }
                objects.push(SceneObject {
                    id: format!("obj_symbol_{symbol_index:03}"),
                    object_type: "symbol".to_string(),
                    name: format!("symbol {symbol_index}"),
                    visible: true,
                    locked: false,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    transform: Transform {
                        translate: [round2(cx - width * 0.5), round2(cy - height * 0.5)],
                        rotate: 0.0,
                        scale: [1.0, 1.0],
                    },
                    style_ref: None,
                    meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
                    payload: ObjectPayload {
                        resource_ref: None,
                        bbox: Some([0.0, 0.0, width, height]),
                        extra,
                    },
                    children: Vec::new(),
                });
                symbol_index += 1;
            }
            _ => {}
        }
    }

    let mut used = vec![false; brackets.len()];
    let mut pairs = Vec::new();
    let bracket_indices_by_graphic_id: BTreeMap<_, _> = brackets
        .iter()
        .enumerate()
        .filter_map(|(index, bracket)| {
            bracket
                .graphic_id
                .as_ref()
                .map(|graphic_id| (graphic_id.as_str(), index))
        })
        .collect();
    for group in descendants(root)
        .into_iter()
        .filter(|node| node.is("bracketedgroup"))
    {
        let attachment_indices: Vec<_> = group
            .direct_children("bracketattachment")
            .filter_map(|attachment| attachment.attr("GraphicID"))
            .filter_map(|graphic_id| bracket_indices_by_graphic_id.get(graphic_id).copied())
            .collect();
        let [left_index, right_index] = attachment_indices.as_slice() else {
            continue;
        };
        if left_index == right_index || used[*left_index] || used[*right_index] {
            continue;
        }
        used[*left_index] = true;
        used[*right_index] = true;
        pairs.push((*left_index, *right_index));
    }

    for left_index in 0..brackets.len() {
        if used[left_index] {
            continue;
        }
        let left = &brackets[left_index];
        let left_bounds = normalized_bbox(left.bbox);
        let mut best_index = None;
        let mut best_dx = f64::INFINITY;
        for right_index in 0..brackets.len() {
            if left_index == right_index || used[right_index] {
                continue;
            }
            let right = &brackets[right_index];
            if right.kind != left.kind {
                continue;
            }
            let right_bounds = normalized_bbox(right.bbox);
            if (center_y(left_bounds) - center_y(right_bounds)).abs() > 2.0
                || (height_of(left_bounds) - height_of(right_bounds)).abs() > 2.0
            {
                continue;
            }
            let dx = (center_x(right_bounds) - center_x(left_bounds)).abs();
            if dx > crate::EPSILON && dx < best_dx {
                best_dx = dx;
                best_index = Some(right_index);
            }
        }
        let Some(right_index) = best_index else {
            continue;
        };
        used[left_index] = true;
        used[right_index] = true;
        pairs.push((left_index, right_index));
    }

    let mut object_index = 1;
    for (left_index, right_index) in pairs {
        let left = &brackets[left_index];
        let right = &brackets[right_index];
        let lb = normalized_bbox(left.bbox);
        let rb = normalized_bbox(right.bbox);
        let (left_bracket, right_bracket, left_bounds, right_bounds) =
            if center_x(lb) <= center_x(rb) {
                (left, right, lb, rb)
            } else {
                (right, left, rb, lb)
            };
        let min_x = lb[0].min(rb[0]);
        let min_y = lb[1].min(rb[1]);
        let max_x = lb[2].max(rb[2]);
        let max_y = lb[3].max(rb[3]);
        let pair_width = round2(max_x - min_x);
        let pair_height = round2(max_y - min_y);
        let group_id = format!("obj_bracket_{object_index:03}");
        let mut meta = json!({
            "source": "cdxml",
            "kind": "bracket-group",
            "graphicIds": [left_bracket.graphic_id.clone(), right_bracket.graphic_id.clone()],
        });
        if let Some(repeat_count) = left.repeat_count.or(right.repeat_count) {
            meta["repeatCount"] = json!(repeat_count);
        }
        let left_child = cdxml_bracket_side_scene_object(
            format!("{group_id}_left"),
            "left",
            left_bracket,
            left_bounds,
            min_x,
            pair_width,
        );
        let right_child = cdxml_bracket_side_scene_object(
            format!("{group_id}_right"),
            "right",
            right_bracket,
            right_bounds,
            min_x,
            pair_width,
        );
        objects.push(SceneObject {
            id: group_id,
            object_type: "group".to_string(),
            name: "bracket-group".to_string(),
            visible: true,
            locked: false,
            z_index: left.z_index.min(right.z_index),
            transform: Transform::identity(),
            style_ref: None,
            meta,
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([round2(min_x), round2(min_y), pair_width, pair_height]),
                extra: BTreeMap::new(),
            },
            children: vec![left_child, right_child],
        });
        object_index += 1;
    }

    // A single CDXML bracket is a complete authored graphic, not necessarily
    // one side of a polymer pair.  Horizontal curly braces used as scheme
    // annotations are commonly stored as two ordered points with identical
    // y coordinates; retain that orientation and render one rotated side.
    for (index, bracket) in brackets.iter().enumerate() {
        if used[index] {
            continue;
        }
        let dx = bracket.bbox[2] - bracket.bbox[0];
        let dy = bracket.bbox[3] - bracket.bbox[1];
        if dx.abs() <= dy.abs() || dx.abs() <= crate::EPSILON {
            continue;
        }
        let length = dx.abs();
        let depth =
            cdxml_bracket_side_width(&bracket.kind, length, length).max(bracket.stroke_width);
        let min_x = bracket.bbox[0].min(bracket.bbox[2]);
        let y = bracket.bbox[1];
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!(bracket.kind.clone()));
        extra.insert("side".to_string(), json!("left"));
        extra.insert("stroke".to_string(), json!(bracket.stroke.clone()));
        extra.insert("strokeWidth".to_string(), json!(bracket.stroke_width));
        extra.insert("lipSize".to_string(), json!(bracket.lip_size));
        extra.insert("orientation".to_string(), json!("horizontal"));
        objects.push(SceneObject {
            id: format!("obj_bracket_{object_index:03}"),
            object_type: "bracket".to_string(),
            name: "standalone horizontal bracket".to_string(),
            visible: true,
            locked: false,
            z_index: bracket.z_index,
            transform: Transform {
                translate: [
                    round2(min_x + (length - depth) * 0.5),
                    round2(y - (length - depth) * 0.5),
                ],
                rotate: -90.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "source": "cdxml",
                "graphicId": bracket.graphic_id.clone(),
                "standalone": true,
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(depth), round2(length)]),
                extra,
            },
            children: Vec::new(),
        });
        object_index += 1;
    }
}

fn cdxml_bracket_side_scene_object(
    object_id: String,
    side: &str,
    bracket: &PendingCdxmlBracket,
    bounds: [f64; 4],
    pair_x: f64,
    pair_width: f64,
) -> SceneObject {
    let stroke_width = bracket.stroke_width;
    let side_height = height_of(bounds);
    let side_width = cdxml_bracket_side_width(&bracket.kind, pair_width, side_height)
        .max(stroke_width)
        .max(bounds[2] - bounds[0]);
    let translate_x = match side {
        "right" if bracket.kind == "round" => pair_x + pair_width,
        "right" => pair_x + pair_width - side_width,
        "left" if bracket.kind == "round" => pair_x - side_width,
        _ => pair_x,
    };
    let mut extra = BTreeMap::new();
    extra.insert("kind".to_string(), json!(bracket.kind.clone()));
    extra.insert("side".to_string(), json!(side));
    extra.insert("stroke".to_string(), json!(bracket.stroke.clone()));
    extra.insert("strokeWidth".to_string(), json!(stroke_width));
    extra.insert("lipSize".to_string(), json!(bracket.lip_size));
    SceneObject {
        id: object_id,
        object_type: "bracket".to_string(),
        name: format!("bracket-{side}"),
        visible: true,
        locked: false,
        z_index: bracket.z_index,
        transform: Transform {
            translate: [round2(translate_x), round2(bounds[1])],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: json!({
            "source": "cdxml",
            "graphicId": bracket.graphic_id.clone(),
            "bracketSide": side,
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, round2(side_width), round2(side_height)]),
            extra,
        },
        children: Vec::new(),
    }
}

fn cdxml_bracket_side_width(kind: &str, pair_width: f64, height: f64) -> f64 {
    match kind {
        "square" => (height * 0.07248).min(pair_width * 0.22).max(0.0),
        "curly" => (height * 0.14423).min(pair_width * 0.24).max(0.0),
        _ => (height * (1.0 - 3.0_f64.sqrt() * 0.5))
            .min(pair_width * 0.22)
            .max(0.0),
    }
}

fn bracket_repeat_counts_by_graphic_id(root: &XmlNode) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for node in descendants(root) {
        if !node.is("bracketedgroup") {
            continue;
        }
        let Some(count) = parse_u32(node.attr("RepeatCount")).filter(|value| *value >= 2) else {
            continue;
        };
        for attachment in node.direct_children("bracketattachment") {
            if let Some(graphic_id) = attachment.attr("GraphicID") {
                counts.insert(graphic_id.to_string(), count);
            }
        }
    }
    counts
}

fn bracket_usage_count(node: &XmlNode) -> Option<u32> {
    node.direct_children("objecttag")
        .filter(|tag| tag.attr("Name") == Some("bracketusage"))
        .flat_map(descendants)
        .find_map(|tag_node| {
            tag_node
                .is("t")
                .then(|| tag_node.full_text().trim().parse::<u32>().ok())
                .flatten()
        })
        .filter(|value| *value >= 2)
}

fn cdxml_symbol_center(_kind: &str, bbox: [f64; 4]) -> [f64; 2] {
    // For CDXML Symbol graphics, the first BoundingBox point is the symbol
    // center; the second point stores the ChemDraw anchor extent/direction.
    [bbox[0], bbox[1]]
}

fn parse_ordered_bbox(value: Option<&str>) -> Option<[f64; 4]> {
    let mut parts = value?.split_whitespace();
    Some([
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ])
}

fn normalized_bbox(bbox: [f64; 4]) -> [f64; 4] {
    [
        bbox[0].min(bbox[2]),
        bbox[1].min(bbox[3]),
        bbox[0].max(bbox[2]),
        bbox[1].max(bbox[3]),
    ]
}

fn center_x(bbox: [f64; 4]) -> f64 {
    (bbox[0] + bbox[2]) * 0.5
}

fn center_y(bbox: [f64; 4]) -> f64 {
    (bbox[1] + bbox[3]) * 0.5
}

fn height_of(bbox: [f64; 4]) -> f64 {
    bbox[3] - bbox[1]
}

fn cdxml_symbol_kind(symbol_type: &str) -> Option<&'static str> {
    Some(match symbol_type {
        "DoubleDagger" => "double-dagger",
        "Dagger" => "dagger",
        "CirclePlus" => "circle-plus",
        "Plus" => "plus",
        "RadicalCation" => "radical-cation",
        "LonePair" => "lone-pair",
        "CircleMinus" => "circle-minus",
        "Minus" => "minus",
        "RadicalAnion" => "radical-anion",
        "Electron" => "electron",
        _ => return None,
    })
}

pub(super) fn append_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let mut index = 1;
    let node_positions: BTreeMap<String, [f64; 2]> = descendants(root)
        .into_iter()
        .filter(|node| node.is("n"))
        .filter_map(|node| Some((node.attr("id")?.to_string(), parse_xy(node.attr("p"))?)))
        .collect();
    let auto_position_enhanced_stereo = !root
        .attr("CreationProgram")
        .is_some_and(|program| program.starts_with("ChemDraw JS"));
    append_text_objects_recursive(
        root,
        false,
        true,
        false,
        false,
        None,
        0,
        None,
        CdxmlTextObjectRole::FreeText,
        None,
        None,
        false,
        auto_position_enhanced_stereo,
        &node_positions,
        None,
        &mut index,
        objects,
        styles,
        defaults,
        colors,
        fonts,
        display_fragment_ids,
        bonded_node_ids,
    );
}

pub(super) fn append_synthesized_enhanced_stereo_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) {
    let node_positions: BTreeMap<&str, [f64; 2]> = descendants(root)
        .into_iter()
        .filter(|node| node.is("n"))
        .filter_map(|node| Some((node.attr("id")?, parse_xy(node.attr("p"))?)))
        .collect();
    let bonds = descendants(root)
        .into_iter()
        .filter(|node| node.is("b"))
        .collect::<Vec<_>>();
    let font_size = defaults.label_size * 0.75;
    let font_id = defaults.label_font.to_string();
    let font_family = fonts
        .get(&font_id)
        .cloned()
        .unwrap_or_else(|| "Arial".to_string());
    let fill = colors.resolve(Some(&defaults.color.to_string()));
    let mut index = objects
        .iter()
        .filter(|object| object.object_type == "text")
        .count()
        + 1;

    for node in descendants(root).into_iter().filter(|node| node.is("n")) {
        let Some(stereo_type) = node.attr("EnhancedStereoType") else {
            continue;
        };
        if node.direct_children("objecttag").any(|tag| {
            tag.attr("Name") == Some("enhancedstereo")
                && !tag
                    .attr("Visible")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no"))
        }) {
            continue;
        }
        let Some(node_id) = node.attr("id") else {
            continue;
        };
        let Some(position) = node_positions.get(node_id).copied() else {
            continue;
        };
        let text = match stereo_type.to_ascii_lowercase().as_str() {
            "absolute" | "abs" => "abs".to_string(),
            "or" => format!("or{}", node.attr("EnhancedStereoGroupNum").unwrap_or("1")),
            "and" => format!("&{}", node.attr("EnhancedStereoGroupNum").unwrap_or("1")),
            _ => continue,
        };
        let direction = enhanced_stereo_label_direction(node_id, position, &bonds, &node_positions)
            .unwrap_or(Point::new(1.0, -1.0));
        let width = estimated_annotation_text_width(&text, font_size);
        let height = font_size * 0.86;
        let left = if direction.x >= 0.0 {
            position[0] + font_size * 0.48
        } else {
            position[0] - width - font_size * 0.5
        };
        let top = if direction.y < 0.0 {
            position[1] - font_size * 1.06
        } else {
            position[1] + font_size * 0.09
        };
        let style_id = format!("style_text_auto_enhanced_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "text",
                "fontFamily": font_family,
                "fontSize": font_size,
                "fontWeight": 400,
                "fill": fill,
                "stroke": null,
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("text".to_string(), json!(text));
        extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
        extra.insert("align".to_string(), json!("left"));
        extra.insert("valign".to_string(), json!("top"));
        extra.insert("lineHeight".to_string(), json!(font_size * 1.15));
        extra.insert("fontSize".to_string(), json!(font_size));
        extra.insert("anchorOffsetX".to_string(), json!(0.0));
        extra.insert("baselineOffset".to_string(), json!(font_size * 0.84));
        extra.insert("preserveLines".to_string(), json!(true));
        extra.insert(
            "runs".to_string(),
            json!([LabelRun {
                text: text.clone(),
                font_family: Some(font_family.clone()),
                font_size: Some(font_size),
                fill: Some(fill.clone()),
                font_weight: Some(400),
                font_style: Some("normal".to_string()),
                underline: Some(false),
                outline: Some(false),
                shadow: Some(false),
                script: Some("normal".to_string()),
            }]),
        );
        objects.push(SceneObject {
            id: format!("obj_text_auto_enhanced_{index:03}"),
            object_type: "text".to_string(),
            name: format!("enhanced stereo label {node_id}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(30),
            transform: Transform {
                translate: [round2(left), round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({
                "source": "cdxml",
                "role": "enhanced_stereo",
                "synthetic": true,
                "nodeId": node_id,
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn append_synthesized_bond_query_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) {
    let node_positions: BTreeMap<&str, [f64; 2]> = descendants(root)
        .into_iter()
        .filter(|node| node.is("n"))
        .filter_map(|node| Some((node.attr("id")?, parse_xy(node.attr("p"))?)))
        .collect();
    let font_size = defaults.label_size * 0.75;
    let font_id = defaults.label_font.to_string();
    let font_family = fonts
        .get(&font_id)
        .cloned()
        .unwrap_or_else(|| "Arial".to_string());
    let fill = colors.resolve(Some(&defaults.color.to_string()));
    let mut index = objects
        .iter()
        .filter(|object| object.object_type == "text")
        .count()
        + 1;

    for bond in descendants(root).into_iter().filter(|node| node.is("b")) {
        let Some(label) = synthesized_bond_query_label(bond.attr("Order")) else {
            continue;
        };
        if bond.direct_children("objecttag").any(|tag| {
            tag.attr("Name") == Some("query")
                && !tag
                    .attr("Visible")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no"))
        }) {
            continue;
        }
        let Some((begin, end)) = bond.attr("B").zip(bond.attr("E")).and_then(|(begin, end)| {
            Some((*node_positions.get(begin)?, *node_positions.get(end)?))
        }) else {
            continue;
        };
        let midpoint = [(begin[0] + end[0]) * 0.5, (begin[1] + end[1]) * 0.5];
        let bond_length = (end[0] - begin[0]).hypot(end[1] - begin[1]);
        let vertical_fraction = if bond_length > crate::EPSILON {
            ((end[1] - begin[1]) / bond_length).abs()
        } else {
            0.0
        };
        let width = estimated_annotation_text_width(&label, font_size);
        let height = font_size * 0.86;
        // ChemDraw anchors the synthetic query mnemonic just left of the bond
        // midpoint and above the shaft. The offset follows the bond slope so
        // horizontal and conventional 30-degree bonds retain the same gap.
        let left = midpoint[0] + font_size * (-0.475 + 0.424 * vertical_fraction);
        let top = midpoint[1] + font_size * (-1.169 + 0.372 * vertical_fraction);
        let style_id = format!("style_text_auto_query_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "text",
                "fontFamily": font_family,
                "fontSize": font_size,
                "fontWeight": 400,
                "fill": fill,
                "stroke": null,
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("text".to_string(), json!(label));
        extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
        extra.insert("align".to_string(), json!("left"));
        extra.insert("valign".to_string(), json!("top"));
        extra.insert("lineHeight".to_string(), json!(font_size * 1.15));
        extra.insert("fontSize".to_string(), json!(font_size));
        extra.insert("anchorOffsetX".to_string(), json!(0.0));
        extra.insert("baselineOffset".to_string(), json!(font_size * 0.82));
        extra.insert("preserveLines".to_string(), json!(true));
        extra.insert(
            "runs".to_string(),
            json!([LabelRun {
                text: label.clone(),
                font_family: Some(font_family.clone()),
                font_size: Some(font_size),
                fill: Some(fill.clone()),
                font_weight: Some(400),
                font_style: Some("normal".to_string()),
                underline: Some(false),
                outline: Some(false),
                shadow: Some(false),
                script: Some("normal".to_string()),
            }]),
        );
        objects.push(SceneObject {
            id: format!("obj_text_auto_query_{index:03}"),
            object_type: "text".to_string(),
            name: format!("bond query label {}", bond.attr("id").unwrap_or("")),
            visible: true,
            locked: false,
            z_index: parse_i32(bond.attr("Z")).unwrap_or(30),
            transform: Transform {
                translate: [round2(left), round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({
                "source": "cdxml",
                "role": "query",
                "synthetic": true,
                "bondId": bond.attr("id"),
                "order": bond.attr("Order"),
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn synthesized_bond_query_label(order: Option<&str>) -> Option<String> {
    let tokens = order?.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return None;
    }
    tokens
        .into_iter()
        .map(|token| match token {
            "1" => Some("S"),
            "1.5" => Some("A"),
            "2" => Some("D"),
            "3" => Some("T"),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

fn enhanced_stereo_label_direction(
    node_id: &str,
    position: [f64; 2],
    bonds: &[&XmlNode],
    node_positions: &BTreeMap<&str, [f64; 2]>,
) -> Option<Point> {
    let bond = bonds.iter().copied().find(|bond| {
        (bond.attr("B") == Some(node_id) || bond.attr("E") == Some(node_id))
            && bond
                .attr("Display")
                .is_some_and(|display| display.to_ascii_lowercase().contains("wedge"))
    })?;
    let other_id = if bond.attr("B") == Some(node_id) {
        bond.attr("E")?
    } else {
        bond.attr("B")?
    };
    let other = node_positions.get(other_id)?;
    let dx = position[0] - other[0];
    let dy = position[1] - other[1];
    let length = (dx * dx + dy * dy).sqrt();
    (length > crate::EPSILON).then(|| Point::new(dx / length, dy / length))
}

fn estimated_annotation_text_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .map(|character| match character {
            '&' => 0.68,
            '0'..='9' => 0.5,
            'a'..='z' => 0.52,
            _ => 0.55,
        })
        .sum::<f64>()
        * font_size
}

pub(super) fn append_text_objects_recursive(
    node: &XmlNode,
    skip_text: bool,
    text_visible: bool,
    force_text_visible: bool,
    prefer_parameterized_bracket_label: bool,
    auto_bracket_label_right_x: Option<f64>,
    placeholder_depth: usize,
    inherited_z: Option<i32>,
    text_role: CdxmlTextObjectRole,
    containing_node_position: Option<[f64; 2]>,
    containing_node_id: Option<String>,
    automatic_object_tag: bool,
    auto_position_enhanced_stereo: bool,
    node_positions: &BTreeMap<String, [f64; 2]>,
    containing_bond_points: Option<([f64; 2], [f64; 2])>,
    index: &mut usize,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let object_tag_role = node
        .is("objecttag")
        .then(|| CdxmlTextObjectRole::from_object_tag_name(node.attr("Name")))
        .flatten();
    let use_parameterized_bracket_label = object_tag_role
        == Some(CdxmlTextObjectRole::ParameterizedBracketLabel)
        && prefer_parameterized_bracket_label;
    let suppress_bracket_usage = object_tag_role == Some(CdxmlTextObjectRole::BracketUsage)
        && prefer_parameterized_bracket_label;
    let next_force_text_visible = if object_tag_role.is_some() {
        use_parameterized_bracket_label
    } else {
        force_text_visible
    };
    let next_text_visible = if use_parameterized_bracket_label {
        true
    } else if suppress_bracket_usage {
        false
    } else if object_tag_role.is_some() {
        !node
            .attr("Visible")
            .is_some_and(|value| value.eq_ignore_ascii_case("no"))
    } else if node.is("objecttag") {
        node.attr("Visible")
            .is_some_and(|value| value.eq_ignore_ascii_case("yes"))
    } else {
        text_visible
    };
    let next_skip_text = if object_tag_role.is_some() {
        false
    } else {
        skip_text
            || (node.is("fragment")
                && node
                    .attr("id")
                    .is_some_and(|id| display_fragment_ids.contains(id)))
            || (node.is("n")
                && node.attr("Element").is_some()
                && node
                    .attr("id")
                    .map_or(true, |id| bonded_node_ids.contains(id)))
    };
    let next_placeholder_depth = if node.is("n")
        && matches!(
            node.attr("NodeType").unwrap_or(""),
            "Fragment" | "Nickname" | "Unspecified"
        ) {
        1
    } else if placeholder_depth > 0 {
        placeholder_depth + 1
    } else {
        0
    };
    let next_text_role = object_tag_role.unwrap_or(text_role);
    let next_containing_node_position = if node.is("n") {
        parse_xy(node.attr("p")).or(containing_node_position)
    } else {
        containing_node_position
    };
    let next_containing_node_id = if node.is("n") {
        node.attr("id")
            .map(ToString::to_string)
            .or(containing_node_id)
    } else {
        containing_node_id
    };
    let next_automatic_object_tag = if object_tag_role.is_some() {
        uses_automatic_object_tag_positioning(node)
    } else {
        automatic_object_tag
    };
    let next_containing_bond_points = if node.is("b") {
        node.attr("B")
            .zip(node.attr("E"))
            .and_then(|(begin, end)| Some((*node_positions.get(begin)?, *node_positions.get(end)?)))
            .or(containing_bond_points)
    } else {
        containing_bond_points
    };
    let next_auto_bracket_label_right_x =
        if node.is("graphic") && node.attr("GraphicType") == Some("Bracket") {
            parse_bbox(node.attr("BoundingBox")).map(|bbox| bbox[0].max(bbox[2]))
        } else if object_tag_role.is_some() && !uses_automatic_object_tag_positioning(node) {
            None
        } else {
            auto_bracket_label_right_x
        };
    let current_z = parse_i32(node.attr("Z")).or(inherited_z);
    if node.is("t") && !skip_text && placeholder_depth <= 1 {
        let visible = text_visible
            && (force_text_visible
                || !node
                    .attr("Visible")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no")));
        if let Some(object) = text_object(
            node,
            *index,
            current_z.unwrap_or(30),
            next_text_role,
            next_containing_node_id.as_deref(),
            visible,
            auto_bracket_label_right_x,
            (next_text_role == CdxmlTextObjectRole::EnhancedStereo
                && next_automatic_object_tag
                && auto_position_enhanced_stereo)
                .then_some(next_containing_node_position)
                .flatten(),
            (next_text_role == CdxmlTextObjectRole::Query && next_automatic_object_tag)
                .then_some(next_containing_bond_points)
                .flatten(),
            styles,
            defaults,
            colors,
            fonts,
        ) {
            objects.push(object);
            *index += 1;
        }
    }
    for child in &node.children {
        append_text_objects_recursive(
            child,
            next_skip_text,
            next_text_visible,
            next_force_text_visible,
            if node.is("graphic") {
                node.direct_children("objecttag")
                    .any(|tag| tag.attr("Name") == Some("parameterizedBracketLabel"))
            } else {
                prefer_parameterized_bracket_label
            },
            next_auto_bracket_label_right_x,
            next_placeholder_depth,
            current_z,
            next_text_role,
            next_containing_node_position,
            next_containing_node_id.clone(),
            next_automatic_object_tag,
            auto_position_enhanced_stereo,
            node_positions,
            next_containing_bond_points,
            index,
            objects,
            styles,
            defaults,
            colors,
            fonts,
            display_fragment_ids,
            bonded_node_ids,
        );
    }
}

fn uses_automatic_object_tag_positioning(node: &XmlNode) -> bool {
    node.attr("PositioningType")
        .is_none_or(|value| value.eq_ignore_ascii_case("auto"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CdxmlTextObjectRole {
    FreeText,
    BracketUsage,
    ParameterizedBracketLabel,
    AtomNumber,
    Query,
    Stereo,
    EnhancedStereo,
}

impl CdxmlTextObjectRole {
    fn from_object_tag_name(name: Option<&str>) -> Option<Self> {
        Some(match name? {
            "bracketusage" => Self::BracketUsage,
            "parameterizedBracketLabel" => Self::ParameterizedBracketLabel,
            "number" => Self::AtomNumber,
            "query" => Self::Query,
            "stereo" => Self::Stereo,
            "enhancedstereo" => Self::EnhancedStereo,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FreeText => "free_text",
            Self::BracketUsage => "bracket_usage",
            Self::ParameterizedBracketLabel => "parameterized_bracket_label",
            Self::AtomNumber => "atom_number",
            Self::Query => "query",
            Self::Stereo => "stereo",
            Self::EnhancedStereo => "enhanced_stereo",
        }
    }

    fn is_bracket_label(self) -> bool {
        matches!(self, Self::BracketUsage | Self::ParameterizedBracketLabel)
    }
}

fn text_object(
    node: &XmlNode,
    index: usize,
    z_index: i32,
    role: CdxmlTextObjectRole,
    containing_node_id: Option<&str>,
    visible: bool,
    auto_bracket_label_right_x: Option<f64>,
    auto_enhanced_stereo_anchor: Option<[f64; 2]>,
    auto_query_bond_points: Option<([f64; 2], [f64; 2])>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Option<SceneObject> {
    let text = node
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| node.full_text())
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let bbox = parse_bbox(node.attr("BoundingBox"));
    let point = parse_xy(node.attr("p")).or_else(|| bbox.map(|bbox| [bbox[0], bbox[1]]))?;
    let align = node
        .attr("CaptionJustification")
        .or_else(|| node.attr("Justification"))
        .unwrap_or(defaults.caption_justification.as_cdxml())
        .to_ascii_lowercase();
    let default_caption_font = defaults.caption_font.to_string();
    let font_id = node
        .attr("font")
        .or_else(|| node.direct_children("s").find_map(|run| run.attr("font")))
        .unwrap_or(default_caption_font.as_str());
    let face = parse_u32(node.attr("face")).unwrap_or(defaults.caption_face);
    let color_id = node
        .attr("color")
        .or_else(|| node.direct_children("s").find_map(|run| run.attr("color")))
        .unwrap_or("0");
    let font_size = parse_f64(node.attr("size")).unwrap_or_else(|| {
        node.direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(defaults.caption_size)
    });
    let style_id = format!("style_text_{index:03}");
    styles.entry(style_id.clone()).or_insert_with(|| {
        json!({
            "kind": "text",
            "fontFamily": fonts.get(font_id).cloned().unwrap_or_else(|| "Arial".to_string()),
            "fontSize": font_size,
            "fontWeight": 400,
            "fill": colors.resolve(Some(color_id)),
            "stroke": null,
        })
    });
    let runs: Vec<LabelRun> = node
        .direct_children("s")
        .flat_map(|run| {
            let run_text = run.full_text();
            if run_text.is_empty() {
                Vec::new()
            } else {
                label_display_runs(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(face),
                    run.attr("font").unwrap_or(font_id),
                    run.attr("color").unwrap_or(color_id),
                    parse_f64(run.attr("size")).unwrap_or(font_size),
                    colors,
                    fonts,
                )
            }
        })
        .collect();
    let (text, runs) = if node.attr("WordWrapWidth").is_some() || node.attr("LineStarts").is_some()
    {
        apply_cdxml_line_starts(&text, runs, node.attr("LineStarts"))
    } else {
        (text, runs)
    };
    let width = bbox
        .map(|bbox| (bbox[2] - bbox[0]).abs())
        .filter(|width| *width > crate::EPSILON)
        .unwrap_or_else(|| (text.chars().count() as f64 * font_size * 0.55).max(font_size));
    let height = bbox
        .map(|bbox| (bbox[3] - bbox[1]).abs())
        .filter(|height| *height > crate::EPSILON)
        .unwrap_or(font_size * 1.4);
    let auto_enhanced_stereo_placement = bbox.and_then(|bbox| {
        auto_enhanced_stereo_anchor
            .and_then(|anchor| automatic_enhanced_stereo_text_placement(anchor, bbox, font_size))
    });
    let auto_query_placement = bbox.and_then(|bbox| {
        auto_query_bond_points
            .and_then(|points| automatic_query_bond_text_placement(points, bbox, font_size))
    });
    let automatic_placement = auto_enhanced_stereo_placement.or(auto_query_placement);
    let translate = if let Some((translate, _)) = automatic_placement {
        translate
    } else if let Some(bbox) = bbox {
        let x = match align.as_str() {
            _ if role.is_bracket_label() => auto_bracket_label_right_x
                .map(|right_x| right_x + font_size * CHEMDRAW_AUTO_BRACKET_LABEL_GAP_EM)
                .unwrap_or(point[0]),
            "center" => (bbox[0] + bbox[2]) * 0.5,
            "right" => bbox[2],
            _ => bbox[0],
        };
        [round2(x), round2(bbox[1])]
    } else {
        [round2(point[0]), round2(point[1])]
    };
    let mut extra = BTreeMap::new();
    extra.insert("text".to_string(), json!(text));
    let box_x = bbox.map_or_else(
        || match align.as_str() {
            "center" => -width * 0.5,
            "right" => -width,
            _ => 0.0,
        },
        |bbox| bbox[0] - translate[0],
    );
    extra.insert(
        "box".to_string(),
        json!([round2(box_x), 0.0, round2(width), round2(height)]),
    );
    extra.insert("align".to_string(), json!(align));
    extra.insert("valign".to_string(), json!("top"));
    let line_spacing = cdxml_text_line_spacing(node, defaults, font_size, &runs);
    extra.insert(
        "lineHeight".to_string(),
        json!(round2(line_spacing.line_height)),
    );
    extra.insert("lineHeightMode".to_string(), json!(line_spacing.mode));
    if !line_spacing.line_advances.is_empty() {
        extra.insert(
            "lineAdvances".to_string(),
            json!(line_spacing
                .line_advances
                .iter()
                .copied()
                .map(round2)
                .collect::<Vec<_>>()),
        );
    }
    extra.insert("fontSize".to_string(), json!(round2(font_size)));
    if let Some((_, baseline_offset)) = automatic_placement {
        extra.insert("anchorOffsetX".to_string(), json!(0.0));
        extra.insert("baselineOffset".to_string(), json!(round2(baseline_offset)));
    } else if let Some(point) = parse_xy(node.attr("p")) {
        extra.insert(
            "anchorOffsetX".to_string(),
            json!(round2(point[0] - translate[0])),
        );
        extra.insert(
            "baselineOffset".to_string(),
            json!(round2(point[1] - translate[1])),
        );
    }
    extra.insert("preserveLines".to_string(), json!(true));
    if !runs.is_empty() {
        extra.insert("runs".to_string(), serde_json::to_value(runs).ok()?);
    }
    Some(SceneObject {
        id: format!("obj_text_{index:03}"),
        object_type: "text".to_string(),
        name: format!("text {index}"),
        visible,
        locked: false,
        z_index,
        transform: Transform {
            translate,
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: Some(style_id),
        meta: json!({
            "source": "cdxml",
            "role": role.as_str(),
            "attachedNodeId": containing_node_id,
            "textId": node.attr("id"),
            "import": {
                "cdxml": {
                    "captionJustification": node.attr("CaptionJustification"),
                    "justification": node.attr("Justification"),
                    "lineHeight": node.attr("LineHeight"),
                    "captionLineHeight": node.attr("CaptionLineHeight"),
                    "wordWrapWidth": node.attr("WordWrapWidth"),
                    "lineStarts": node.attr("LineStarts"),
                }
            }
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: None,
            extra,
        },
        children: Vec::new(),
    })
}

fn automatic_enhanced_stereo_text_placement(
    anchor: [f64; 2],
    bbox: [f64; 4],
    font_size: f64,
) -> Option<([f64; 2], f64)> {
    let width = (bbox[2] - bbox[0]).abs();
    let height = (bbox[3] - bbox[1]).abs();
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let cached_center = [(bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5];
    let dx = cached_center[0] - anchor[0];
    let dy = cached_center[1] - anchor[1];
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    let unit = [dx / length, dy / length];
    let nearly_vertical = unit[0].abs() < 0.1;
    let radius = font_size * if nearly_vertical { 0.65 } else { 0.75 };
    let horizontal_bias = if nearly_vertical {
        -font_size * 0.115
    } else {
        0.0
    };
    let center = [
        anchor[0] + unit[0] * radius + horizontal_bias,
        anchor[1] + unit[1] * radius,
    ];
    let translate = [
        round2(center[0] - width * 0.5),
        round2(center[1] - height * 0.5),
    ];
    let baseline_offset = height + font_size * 0.08;
    Some((translate, baseline_offset))
}

fn automatic_query_bond_text_placement(
    points: ([f64; 2], [f64; 2]),
    bbox: [f64; 4],
    font_size: f64,
) -> Option<([f64; 2], f64)> {
    let width = (bbox[2] - bbox[0]).abs();
    let height = (bbox[3] - bbox[1]).abs();
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let midpoint = [
        (points.0[0] + points.1[0]) * 0.5,
        (points.0[1] + points.1[1]) * 0.5,
    ];
    let cached_center = [(bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5];
    let dx = cached_center[0] - midpoint[0];
    let dy = cached_center[1] - midpoint[1];
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    let unit = [dx / length, dy / length];
    let translate = if unit[0] < -0.4 && unit[1] < -0.4 {
        [
            midpoint[0] - width - font_size * 0.18,
            midpoint[1] - height - font_size * 0.07,
        ]
    } else if unit[0] < -0.7 {
        [
            midpoint[0] - width - font_size * 0.47,
            midpoint[1] + font_size * 0.065,
        ]
    } else if unit[0] > 0.7 {
        [midpoint[0] + font_size * 0.317, midpoint[1] - height * 0.45]
    } else {
        return None;
    };
    Some(([round2(translate[0]), round2(translate[1])], height))
}

fn cdxml_text_line_spacing(
    node: &XmlNode,
    defaults: CdxmlDefaults,
    font_size: f64,
    runs: &[LabelRun],
) -> super::ResolvedCdxmlLineSpacing {
    let value = parse_cdxml_line_height(node.attr("CaptionLineHeight"))
        .or_else(|| parse_cdxml_line_height(node.attr("LineHeight")))
        .or(defaults.caption_line_height)
        .or(defaults.line_height)
        .unwrap_or(CdxmlLineHeight::Auto);
    match value {
        CdxmlLineHeight::Fixed(value) if value > 1.0 => super::ResolvedCdxmlLineSpacing {
            line_height: value,
            line_advances: Vec::new(),
            mode: "fixed",
        },
        CdxmlLineHeight::Variable => {
            let line_runs = super::split_label_runs_by_line(runs);
            let line_advances = crate::variable_text_line_advances(&line_runs, font_size);
            super::ResolvedCdxmlLineSpacing {
                line_height: line_advances
                    .first()
                    .copied()
                    .unwrap_or_else(|| crate::molecule_label_line_advance(font_size)),
                line_advances,
                mode: "variable",
            }
        }
        _ => super::ResolvedCdxmlLineSpacing {
            line_height: super::chemdraw_auto_text_line_height(font_size, runs),
            line_advances: Vec::new(),
            mode: "auto",
        },
    }
}
