use crate::*;

pub(crate) fn write_emf_preview(
    path: &Path,
    render_list_json: &str,
    bounds_json: &str,
) -> Result<(), String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CloseEnhMetaFile, CreateEnhMetaFileW, DeleteEnhMetaFile, Ellipse, LineTo, MoveToEx,
        Polygon, Polyline, Rectangle, SetBkMode, SetTextColor, TextOutW, TRANSPARENT,
    };

    let primitives: Vec<serde_json::Value> =
        serde_json::from_str(render_list_json).map_err(|error| error.to_string())?;
    let bounds_value: serde_json::Value =
        serde_json::from_str(bounds_json).map_err(|error| error.to_string())?;
    let bounds = EmfBounds::from_json(&bounds_value).unwrap_or_else(|| {
        bounds_from_primitives(&primitives).unwrap_or(EmfBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        })
    });
    let layout = EmfLayout::new(bounds);
    let frame = RECT {
        left: 0,
        top: 0,
        right: layout.page_width,
        bottom: layout.page_height,
    };
    let path_wide = wide_null(&path.to_string_lossy());
    let desc = wide_null("ChemSema\0EMF Preview");
    let hdc = unsafe {
        CreateEnhMetaFileW(
            std::ptr::null_mut(),
            path_wide.as_ptr(),
            &frame,
            desc.as_ptr(),
        )
    };
    if hdc.is_null() {
        return Err("Failed to create EMF preview.".to_string());
    }

    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
    }

    for primitive in primitives
        .iter()
        .filter(|primitive| is_document_primitive(primitive))
    {
        let kind = primitive
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        match kind {
            "line" => {
                let Some(from) = point_from_json(primitive.get("from")) else {
                    continue;
                };
                let Some(to) = point_from_json(primitive.get("to")) else {
                    continue;
                };
                with_pen(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    || unsafe {
                        let from = layout.point(from);
                        let to = layout.point(to);
                        MoveToEx(hdc, from.x, from.y, std::ptr::null_mut());
                        LineTo(hdc, to.x, to.y);
                    },
                );
            }
            "polyline" | "path" => {
                let points = points_from_json(primitive.get("points"), &layout);
                if points.len() < 2 {
                    continue;
                }
                with_pen(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    || unsafe {
                        Polyline(hdc, points.as_ptr(), points.len() as i32);
                    },
                );
            }
            "polygon" | "filled-path" => {
                let points = points_from_json(primitive.get("points"), &layout);
                if points.len() < 3 {
                    continue;
                }
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Polygon(hdc, points.as_ptr(), points.len() as i32);
                    },
                );
            }
            "rect" => {
                let Some((left, top, right, bottom)) = rect_from_json(primitive, &layout) else {
                    continue;
                };
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").and_then(|value| value.as_str()) != Some("none"),
                    || unsafe {
                        Rectangle(hdc, left, top, right, bottom);
                    },
                );
            }
            "circle" => {
                let Some(center) = point_from_json(primitive.get("center")) else {
                    continue;
                };
                let radius = primitive
                    .get("radius")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let center = layout.point(center);
                let radius = (radius * layout.scale).round().max(1.0) as i32;
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Ellipse(
                            hdc,
                            center.x - radius,
                            center.y - radius,
                            center.x + radius,
                            center.y + radius,
                        );
                    },
                );
            }
            "ellipse" => {
                let Some(center) = point_from_json(primitive.get("center")) else {
                    continue;
                };
                let rx = primitive
                    .get("rx")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let ry = primitive
                    .get("ry")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let center = layout.point(center);
                let rx = (rx * layout.scale).round().max(1.0) as i32;
                let ry = (ry * layout.scale).round().max(1.0) as i32;
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Ellipse(
                            hdc,
                            center.x - rx,
                            center.y - ry,
                            center.x + rx,
                            center.y + ry,
                        );
                    },
                );
            }
            "text" => {
                let Some(text) = primitive.get("text").and_then(|value| value.as_str()) else {
                    continue;
                };
                let x = primitive
                    .get("x")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let y = primitive
                    .get("y")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let point = layout.point((x, y));
                let wide = wide_null(text);
                unsafe {
                    SetTextColor(hdc, color_from_json(primitive.get("fill"), 0x000000));
                    TextOutW(
                        hdc,
                        point.x,
                        point.y,
                        wide.as_ptr(),
                        text.encode_utf16().count() as i32,
                    );
                }
            }
            _ => {}
        }
    }

    let metafile = unsafe { CloseEnhMetaFile(hdc) };
    if metafile.is_null() {
        return Err("Failed to finalize EMF preview.".to_string());
    }
    unsafe {
        DeleteEnhMetaFile(metafile);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn write_emf_preview(
    _path: &Path,
    _render_list_json: &str,
    _bounds_json: &str,
) -> Result<(), String> {
    Err("EMF export is only implemented on Windows.".to_string())
}

#[derive(Debug, Clone, Copy)]
struct EmfBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl EmfBounds {
    fn from_json(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            min_x: value.get("minX")?.as_f64()?,
            min_y: value.get("minY")?.as_f64()?,
            max_x: value.get("maxX")?.as_f64()?,
            max_y: value.get("maxY")?.as_f64()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct EmfLayout {
    bounds: EmfBounds,
    scale: f64,
    page_width: i32,
    page_height: i32,
    margin: i32,
}

impl EmfLayout {
    fn new(bounds: EmfBounds) -> Self {
        let width = (bounds.max_x - bounds.min_x).abs().max(1.0);
        let height = (bounds.max_y - bounds.min_y).abs().max(1.0);
        let margin = 300;
        let max_side = 9000.0;
        let scale = (max_side / width.max(height)).max(1.0);
        Self {
            bounds,
            scale,
            page_width: (width * scale).round() as i32 + margin * 2,
            page_height: (height * scale).round() as i32 + margin * 2,
            margin,
        }
    }

    fn point(&self, point: (f64, f64)) -> windows_sys::Win32::Foundation::POINT {
        windows_sys::Win32::Foundation::POINT {
            x: self.margin + ((point.0 - self.bounds.min_x) * self.scale).round() as i32,
            y: self.margin + ((point.1 - self.bounds.min_y) * self.scale).round() as i32,
        }
    }
}

fn is_document_primitive(primitive: &serde_json::Value) -> bool {
    primitive
        .get("role")
        .and_then(|value| value.as_str())
        .map(|role| role.starts_with("document-"))
        .unwrap_or(false)
}

fn point_from_json(value: Option<&serde_json::Value>) -> Option<(f64, f64)> {
    let value = value?;
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

fn points_from_json(
    value: Option<&serde_json::Value>,
    layout: &EmfLayout,
) -> Vec<windows_sys::Win32::Foundation::POINT> {
    value
        .and_then(|value| value.as_array())
        .map(|points| {
            points
                .iter()
                .filter_map(|point| point_from_json(Some(point)))
                .map(|point| layout.point(point))
                .collect()
        })
        .unwrap_or_default()
}

fn rect_from_json(
    primitive: &serde_json::Value,
    layout: &EmfLayout,
) -> Option<(i32, i32, i32, i32)> {
    let x = primitive.get("x")?.as_f64()?;
    let y = primitive.get("y")?.as_f64()?;
    let width = primitive.get("width")?.as_f64()?;
    let height = primitive.get("height")?.as_f64()?;
    let top_left = layout.point((x, y));
    let bottom_right = layout.point((x + width, y + height));
    Some((top_left.x, top_left.y, bottom_right.x, bottom_right.y))
}

fn bounds_from_primitives(primitives: &[serde_json::Value]) -> Option<EmfBounds> {
    let mut bounds: Option<EmfBounds> = None;
    for point in primitives
        .iter()
        .flat_map(|primitive| primitive_points(primitive).into_iter())
    {
        bounds = Some(match bounds {
            Some(bounds) => EmfBounds {
                min_x: bounds.min_x.min(point.0),
                min_y: bounds.min_y.min(point.1),
                max_x: bounds.max_x.max(point.0),
                max_y: bounds.max_y.max(point.1),
            },
            None => EmfBounds {
                min_x: point.0,
                min_y: point.1,
                max_x: point.0,
                max_y: point.1,
            },
        });
    }
    bounds
}

fn primitive_points(primitive: &serde_json::Value) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for key in ["from", "to", "center"] {
        if let Some(point) = point_from_json(primitive.get(key)) {
            points.push(point);
        }
    }
    if let Some(array) = primitive.get("points").and_then(|value| value.as_array()) {
        points.extend(
            array
                .iter()
                .filter_map(|point| point_from_json(Some(point))),
        );
    }
    if let (Some(x), Some(y), Some(width), Some(height)) = (
        primitive.get("x").and_then(|value| value.as_f64()),
        primitive.get("y").and_then(|value| value.as_f64()),
        primitive.get("width").and_then(|value| value.as_f64()),
        primitive.get("height").and_then(|value| value.as_f64()),
    ) {
        points.push((x, y));
        points.push((x + width, y + height));
    }
    points
}

fn pen_width_from_json(primitive: &serde_json::Value, scale: f64) -> i32 {
    let width = primitive
        .get("strokeWidth")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.02);
    (width * scale).round().clamp(1.0, 80.0) as i32
}

fn color_from_json(value: Option<&serde_json::Value>, fallback_rgb: u32) -> u32 {
    let Some(raw) = value.and_then(|value| value.as_str()) else {
        return rgb_to_colorref(fallback_rgb);
    };
    let raw = raw.trim();
    if raw == "none" {
        return rgb_to_colorref(fallback_rgb);
    }
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() < 6 {
        return rgb_to_colorref(fallback_rgb);
    }
    u32::from_str_radix(&hex[..6], 16)
        .map(rgb_to_colorref)
        .unwrap_or_else(|_| rgb_to_colorref(fallback_rgb))
}

fn rgb_to_colorref(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xff;
    let g = (rgb >> 8) & 0xff;
    let b = rgb & 0xff;
    r | (g << 8) | (b << 16)
}

#[cfg(target_os = "windows")]
fn with_pen<F: FnOnce()>(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    color: u32,
    width: i32,
    draw: F,
) {
    use windows_sys::Win32::Graphics::Gdi::{CreatePen, DeleteObject, SelectObject, PS_SOLID};
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        let previous = SelectObject(hdc, pen);
        draw();
        SelectObject(hdc, previous);
        DeleteObject(pen);
    }
}

#[cfg(target_os = "windows")]
fn with_pen_and_brush<F: FnOnce()>(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    stroke: u32,
    width: i32,
    fill: u32,
    fill_enabled: bool,
    draw: F,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, GetStockObject, SelectObject, NULL_BRUSH,
        PS_SOLID,
    };
    unsafe {
        let pen = CreatePen(PS_SOLID, width, stroke);
        let brush = if fill_enabled {
            CreateSolidBrush(fill)
        } else {
            GetStockObject(NULL_BRUSH)
        };
        let previous_pen = SelectObject(hdc, pen);
        let previous_brush = SelectObject(hdc, brush);
        draw();
        SelectObject(hdc, previous_brush);
        SelectObject(hdc, previous_pen);
        if fill_enabled {
            DeleteObject(brush);
        }
        DeleteObject(pen);
    }
}
