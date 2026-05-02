mod clipboard;
mod command;
mod delete;
mod select;
mod templates;
mod text_edit;

pub use self::command::{
    CommandAnchor, EditorCommand, FocusedDeleteSource, HistoryEntry, TextEditCommandTarget,
};
use self::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
pub use self::text_edit::{
    TextEditLayout, TextEditLayoutCaret, TextEditLayoutCaretOffset, TextEditLayoutLine,
    TextEditLayoutRect, TextEditSelection, TextEditSelectionState, TextEditSession, TextEditTarget,
};

use self::delete::FocusedDeleteMode;
use crate::{
    anchor_from_point, angle_between, bond_center_focus_length, can_draw_bond,
    can_focus_bond_center, can_focus_endpoint, default_angle_for_anchor_for_variant,
    direction_from_angle, endpoint_from_angle_for_document, hit_test_arrow_center,
    hit_test_bond_center, hit_test_endpoint, hit_test_endpoint_excluding, nearest_angle,
    render_document, snapped_angle_for_anchor, ArrowCurve, ArrowEndpointStyle, ArrowHeadSize,
    ArrowNoGo, ArrowVariant, Bond, BondAnchor, BondLinePattern, BondLineStyles, BondLineWeight,
    BondLineWeights, BondPreview, BondStereo, BondVariant, ChemcoreDocument, DoubleBond,
    DoubleBondPlacement, DragState, EditorOptions, EndpointHit, HoverTextBox, OverlayState, Point,
    PointerEvent, RenderPrimitive, RenderRole, SceneObject, SelectionState, ShapeKind, ShapeStyle,
    Tool, ToolState, WorldCm, BOND_CENTER_FOCUS_WIDTH, BOND_CENTER_HIT_RADIUS, DEFAULT_BOND_LENGTH,
    DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS, GLOBAL_SNAP_ANGLES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;

const HOVER_STROKE_WIDTH: f64 = crate::px_to_cm(1.1);
const HOVER_LABEL_STROKE_WIDTH: f64 = crate::px_to_cm(1.1);
const HOVER_ENDPOINT_STROKE_WIDTH: f64 = crate::px_to_cm(1.4);
const HOVER_BOND_CENTER_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const PREVIEW_END_RADIUS: f64 = crate::px_to_cm(5.0);
const PREVIEW_END_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const SHAPE_STROKE_WIDTH: f64 = 1.0;
const SHAPE_DASH_LENGTH: f64 = 2.7;
const ELLIPSE_MINOR_AXIS_RATIO: f64 = 0.4;
const ROUND_RECT_CORNER_RADIUS: f64 = 6.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub selection: SelectionState,
    pub overlay: OverlayState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutRequest {
    pub session: TextEditSession,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<TextEditSelection>,
}

pub struct Engine {
    state: EngineState,
    drag: Option<DragState>,
    arrow_drag: Option<ArrowDragState>,
    arrow_edit_drag: Option<ArrowEditDragState>,
    selection_drag: Option<select::SelectionMoveDrag>,
    selection_rotate_drag: Option<select::SelectionRotateDrag>,
    template_drag: Option<templates::TemplateDrag>,
    shape_drag: Option<ShapeDragState>,
    clipboard: Option<clipboard::ClipboardContent>,
    options: EditorOptions,
    next_id: u64,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    command_context: Vec<EditorCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArrowDragState {
    start: Point,
    end: Option<Point>,
    has_dragged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrowEditMode {
    Head,
    Tail,
    Curve,
}

#[derive(Debug, Clone)]
struct ArrowEditDragState {
    object_id: String,
    mode: ArrowEditMode,
    original_points: Vec<Point>,
    start_pointer: Point,
    has_dragged: bool,
    current_degrees: f64,
    undo_pushed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShapeDragState {
    start: Point,
    current: Point,
    has_dragged: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState {
                document: ChemcoreDocument::blank(),
                tool: ToolState::default(),
                selection: SelectionState::default(),
                overlay: OverlayState::default(),
            },
            drag: None,
            arrow_drag: None,
            arrow_edit_drag: None,
            selection_drag: None,
            selection_rotate_drag: None,
            template_drag: None,
            shape_drag: None,
            clipboard: None,
            options: EditorOptions::default(),
            next_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            command_context: Vec::new(),
        }
    }

    pub fn state(&self) -> &EngineState {
        &self.state
    }

    pub fn state_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state)
    }

    pub fn document_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state.document)
    }

    pub fn load_document_json(&mut self, json: &str) -> Result<(), String> {
        let document = crate::parse_document_json(json)?;
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
        self.next_id = self.infer_next_id();
        Ok(())
    }

    pub fn render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_document() {
            render_document(&preview_document)
        } else {
            render_document(&self.state.document)
        };
        out.extend(self.selection_render_list());
        if let Some(hover) = &self.state.overlay.hover_text_box {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::HoverTextBox,
                object_id: hover.object_id.clone(),
                x: hover.bounds[0],
                y: hover.bounds[1],
                width: (hover.bounds[2] - hover.bounds[0]).max(0.0),
                height: (hover.bounds[3] - hover.bounds[1]).max(0.0),
                fill: Some("rgba(47,111,237,0.12)".to_string()),
                stroke: Some("rgba(47,111,237,0.76)".to_string()),
                stroke_width: HOVER_STROKE_WIDTH,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
        if let Some(hover) = &self.state.overlay.hover_arrow {
            if !self
                .state
                .selection
                .arrow_objects
                .contains(&hover.object_id)
            {
                for handle in &hover.handles {
                    out.push(RenderPrimitive::Circle {
                        role: RenderRole::HoverArrowHandle,
                        object_id: Some(hover.object_id.clone()),
                        center: *handle,
                        radius: crate::px_to_cm(1.5),
                        fill: "#ffffff".to_string(),
                        stroke: "rgba(47,111,237,0.82)".to_string(),
                        stroke_width: HOVER_STROKE_WIDTH,
                    });
                }
            }
        }
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            if let Some(label_anchor) = &hover.label_anchor {
                out.push(RenderPrimitive::Rect {
                    role: RenderRole::HoverLabelGlyph,
                    object_id: None,
                    x: label_anchor.glyph_box[0],
                    y: label_anchor.glyph_box[1],
                    width: (label_anchor.glyph_box[2] - label_anchor.glyph_box[0]).max(0.0),
                    height: (label_anchor.glyph_box[3] - label_anchor.glyph_box[1]).max(0.0),
                    fill: Some("rgba(47,111,237,0.12)".to_string()),
                    stroke: Some("rgba(47,111,237,0.82)".to_string()),
                    stroke_width: HOVER_LABEL_STROKE_WIDTH,
                    rx: None,
                    ry: None,
                    dash_array: Vec::new(),
                    fill_gradient: None,
                });
            } else {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::HoverEndpoint,
                    object_id: None,
                    center: hover.point,
                    radius: ENDPOINT_FOCUS_RADIUS,
                    fill: "rgba(47,111,237,0.24)".to_string(),
                    stroke: "rgba(47,111,237,0.78)".to_string(),
                    stroke_width: HOVER_ENDPOINT_STROKE_WIDTH,
                });
            }
        }
        if let Some(hover) = &self.state.overlay.hover_bond_center {
            let focus_length = bond_center_focus_length(hover.begin, hover.end);
            if focus_length > crate::EPSILON {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::HoverBondCenter,
                    object_id: None,
                    bond_id: None,
                    points: centered_oriented_rect_points(
                        hover.begin,
                        hover.end,
                        focus_length,
                        BOND_CENTER_FOCUS_WIDTH,
                    ),
                    fill: "rgba(47,111,237,0.11)".to_string(),
                    stroke: "rgba(47,111,237,0.72)".to_string(),
                    stroke_width: HOVER_BOND_CENTER_STROKE_WIDTH,
                });
            }
        }
        if let Some(preview) = &self.state.overlay.preview {
            out.push(RenderPrimitive::Circle {
                role: RenderRole::PreviewEnd,
                object_id: None,
                center: preview.end,
                radius: PREVIEW_END_RADIUS,
                fill: "#ffffff".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: PREVIEW_END_STROKE_WIDTH,
            });
        }
        out
    }

    pub fn set_tool_state(&mut self, tool: ToolState) {
        self.state.tool = tool;
        self.clear_interaction();
    }

    pub fn pointer_move(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.state.tool.active_tool == Tool::Select {
            self.hover_select_target(point);
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_move_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_move_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_move_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            {
                self.state.overlay.hover_endpoint = Some(endpoint);
                return;
            }
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
            }
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: None,
                    node_id: Some(node_id),
                });
                return;
            }
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
            return;
        }
        if !can_focus_endpoint(&self.state.tool) {
            self.clear_interaction();
            return;
        }

        if let Some(mut drag) = self.drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.hover_endpoint = None;
                let target = self.drag_target_endpoint(&drag.anchor, point);
                let end = if let Some(target) = target {
                    self.state.overlay.hover_endpoint = Some(target.clone());
                    drag.target = Some(BondAnchor {
                        node_id: Some(target.node_id.clone()),
                        point: target.point,
                        label_anchor: target.label_anchor.clone(),
                    });
                    target.point
                } else if drag.free_length {
                    drag.target = None;
                    point
                } else {
                    drag.target = None;
                    let angle = snapped_angle_for_anchor(&self.state.document, &drag.anchor, point);
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length_world_cm().value(),
                    )
                };
                drag.preview_end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.anchor.point,
                    end,
                });
            }
            self.drag = Some(drag);
            return;
        }

        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_text_box = None;
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
        }
        if can_focus_bond_center(&self.state.tool) {
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
                return;
            }
        }
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
    }

    pub fn pointer_down(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Select {
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_down_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_down_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_down_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            let point = event.point();
            self.with_command(
                EditorCommand::DeleteFocusedAtPoint {
                    x: point.x,
                    y: point.y,
                    source: FocusedDeleteSource::DeleteTool,
                },
                |engine| engine.delete_focused_at_point(point, FocusedDeleteMode::DeleteToolClick),
            );
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, event.point(), ENDPOINT_HIT_RADIUS);
            return;
        }
        if !can_draw_bond(&self.state.tool) {
            if can_focus_bond_center(&self.state.tool) {
                if let Some(hit) = hit_test_bond_center(
                    &self.state.document,
                    event.point(),
                    BOND_CENTER_HIT_RADIUS,
                ) {
                    self.cycle_bond_center_style(&hit.bond_id);
                }
            }
            return;
        }
        let point = event.point();
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.drag = Some(DragState {
                anchor: BondAnchor {
                    node_id: Some(endpoint.node_id),
                    point: endpoint.point,
                    label_anchor: endpoint.label_anchor,
                },
                start: point,
                has_dragged: false,
                free_length: event.alt_key,
                preview_end: None,
                target: None,
            });
            return;
        }
        if let Some(hit) = hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            self.cycle_bond_center_style(&hit.bond_id);
            return;
        }
        let Some(anchor) = anchor_from_point(&self.state.document, point) else {
            return;
        };
        self.drag = Some(DragState {
            anchor,
            start: point,
            has_dragged: false,
            free_length: false,
            preview_end: None,
            target: None,
        });
    }

    pub fn pointer_up(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_up_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, event.point(), ENDPOINT_HIT_RADIUS);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_up_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_up_shape(event);
            return;
        }
        let Some(drag) = self.drag.take() else {
            return;
        };
        let end_anchor = if drag.has_dragged {
            if let Some(target) = drag.target {
                target
            } else if drag.free_length {
                BondAnchor {
                    node_id: None,
                    point: drag.preview_end.unwrap_or_else(|| event.point()),
                    label_anchor: None,
                }
            } else {
                let end = drag.preview_end.unwrap_or_else(|| {
                    let angle =
                        snapped_angle_for_anchor(&self.state.document, &drag.anchor, event.point());
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length_world_cm().value(),
                    )
                });
                BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                }
            }
        } else {
            let angle = default_angle_for_anchor_for_variant(
                &self.state.document,
                &drag.anchor,
                self.state.tool.bond_variant,
            );
            let end = endpoint_from_angle_for_document(
                &self.state.document,
                &drag.anchor,
                angle,
                self.options.bond_length_world_cm().value(),
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        let pointer_point = event.point();
        let added = self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
        if added {
            self.refresh_bond_mode_hover(pointer_point);
        }
    }

    fn pointer_move_arrow(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_arrow = None;
        if let Some(mut drag) = self.arrow_drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                let end = arrow_drag_end(drag.start, point, event.alt_key);
                drag.end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.start,
                    end,
                });
            }
            self.arrow_drag = Some(drag);
            return;
        }
        self.state.overlay.preview = None;
        self.state.overlay.hover_arrow =
            hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
    }

    fn pointer_down_arrow(&mut self, event: PointerEvent) {
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.arrow_drag = Some(ArrowDragState {
            start: event.point(),
            end: None,
            has_dragged: false,
        });
    }

    fn pointer_up_arrow(&mut self, event: PointerEvent) {
        let Some(drag) = self.arrow_drag.take() else {
            return;
        };
        self.state.overlay.preview = None;
        let end = if drag.has_dragged {
            drag.end
                .unwrap_or_else(|| arrow_drag_end(drag.start, event.point(), event.alt_key))
        } else {
            return;
        };
        if drag.start.distance(end) <= crate::EPSILON {
            return;
        }
        if self.add_arrow_between(drag.start, end).is_some() {
            self.state.overlay.hover_arrow = None;
        }
    }

    fn add_arrow_between(&mut self, start: Point, end: Point) -> Option<String> {
        let command = EditorCommand::AddArrow {
            begin: CommandAnchor::from(start),
            end: CommandAnchor::from(end),
            variant: self.state.tool.arrow_variant,
            head_size: self.state.tool.arrow_head_size,
            curve: self.state.tool.arrow_curve,
            head_style: self.state.tool.arrow_head_style,
            tail_style: self.state.tool.arrow_tail_style,
            head: arrow_endpoint_enabled(self.state.tool.arrow_head_style),
            tail: arrow_endpoint_enabled(self.state.tool.arrow_tail_style),
            bold: self.state.tool.arrow_bold,
            no_go: self.state.tool.arrow_no_go,
        };
        let mut object_id = None;
        let changed = self.with_command(command, |engine| {
            object_id = engine.add_arrow_between_untracked(start, end);
            object_id.is_some()
        });
        changed.then_some(object_id).flatten()
    }

    fn add_arrow_between_untracked(&mut self, start: Point, end: Point) -> Option<String> {
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        ensure_arrow_style(&mut self.state.document);
        let object_id = self.next_id("obj_line");
        let object = self.arrow_scene_object(start, end, object_id.clone());
        self.state.document.objects.push(object);
        Some(object_id)
    }

    fn arrow_scene_object(&self, start: Point, end: Point, object_id: String) -> SceneObject {
        let (length, center_length, width) = arrow_payload_dimensions(
            self.state.tool.arrow_variant,
            self.state.tool.arrow_head_size,
            self.state.tool.arrow_bold,
        );
        let head_style = self.state.tool.arrow_head_style;
        let tail_style = self.state.tool.arrow_tail_style;
        let head_enabled = arrow_endpoint_enabled(head_style);
        let tail_enabled = arrow_endpoint_enabled(tail_style);
        let mut extra = std::collections::BTreeMap::new();
        extra.insert("kind".to_string(), json!("line"));
        extra.insert(
            "points".to_string(),
            json!([
                [round_point(start).x, round_point(start).y],
                [round_point(end).x, round_point(end).y]
            ]),
        );
        extra.insert(
            "head".to_string(),
            JsonValue::String(if head_enabled { "end" } else { "none" }.to_string()),
        );
        extra.insert(
            "tail".to_string(),
            JsonValue::String(if tail_enabled { "start" } else { "none" }.to_string()),
        );
        extra.insert(
            "arrowHead".to_string(),
            json!({
                "kind": arrow_variant_name(self.state.tool.arrow_variant),
                "curve": arrow_curve_sweep_degrees(self.state.tool.arrow_variant, self.state.tool.arrow_curve),
                "head": arrow_endpoint_payload_name(head_style),
                "tail": arrow_endpoint_payload_name(tail_style),
                "length": length,
                "centerLength": center_length,
                "width": width,
                "bold": self.state.tool.arrow_bold,
                "noGo": arrow_no_go_payload_name(self.state.tool.arrow_no_go),
            }),
        );
        SceneObject {
            id: object_id,
            object_type: "line".to_string(),
            name: "arrow".to_string(),
            visible: true,
            locked: false,
            z_index: 20,
            transform: crate::Transform::identity(),
            style_ref: Some("style_arrow_default".to_string()),
            meta: json!({"source": "chemcore-editor"}),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
        }
    }

    pub fn apply_arrow_options_to_selection(
        &mut self,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
        no_go: ArrowNoGo,
    ) -> bool {
        let object_ids = self.state.selection.arrow_objects.clone();
        if object_ids.is_empty() {
            return false;
        }
        let command = EditorCommand::ApplyArrowStyle {
            object_ids,
            variant,
            head_size,
            curve,
            head_style,
            tail_style,
            head,
            tail,
            bold,
            no_go,
        };
        self.with_command(command, |engine| {
            engine.apply_arrow_options_to_selection_untracked(
                variant, head_size, curve, head_style, tail_style, bold, no_go,
            )
        })
    }

    fn apply_arrow_options_to_selection_untracked(
        &mut self,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        bold: bool,
        no_go: ArrowNoGo,
    ) -> bool {
        let selected: std::collections::BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let (length, center_length, width) = arrow_payload_dimensions(variant, head_size, bold);
        let mut updates = Vec::new();
        for (index, object) in self.state.document.objects.iter().enumerate() {
            if object.object_type != "line" || !selected.contains(&object.id) {
                continue;
            }
            let mut next_extra = object.payload.extra.clone();
            next_extra.insert(
                "head".to_string(),
                JsonValue::String(
                    if arrow_endpoint_enabled(head_style) {
                        "end"
                    } else {
                        "none"
                    }
                    .to_string(),
                ),
            );
            next_extra.insert(
                "tail".to_string(),
                JsonValue::String(
                    if arrow_endpoint_enabled(tail_style) {
                        "start"
                    } else {
                        "none"
                    }
                    .to_string(),
                ),
            );
            next_extra.insert(
                "arrowHead".to_string(),
                json!({
                    "kind": arrow_variant_name(variant),
                    "curve": arrow_curve_sweep_degrees(variant, curve),
                    "head": arrow_endpoint_payload_name(head_style),
                    "tail": arrow_endpoint_payload_name(tail_style),
                    "length": length,
                    "centerLength": center_length,
                    "width": width,
                    "bold": bold,
                    "noGo": arrow_no_go_payload_name(no_go),
                }),
            );
            if object.payload.extra != next_extra
                || object.style_ref.as_deref() != Some("style_arrow_default")
            {
                updates.push((index, next_extra));
            }
        }
        if updates.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        for (index, next_extra) in updates {
            if let Some(object) = self.state.document.objects.get_mut(index) {
                object.payload.extra = next_extra;
                object.style_ref = Some("style_arrow_default".to_string());
            }
        }
        self.state.overlay.hover_arrow = None;
        true
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.arrow_drag = None;
        self.arrow_edit_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        self.template_drag = None;
        self.shape_drag = None;
        self.state.overlay = OverlayState::default();
    }

    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        self.add_bond_between(
            anchor,
            BondAnchor {
                node_id: None,
                point: end,
                label_anchor: None,
            },
            1,
        );
    }

    pub fn add_single_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor) -> bool {
        self.add_bond_between(anchor, end, 1)
    }

    pub fn add_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor, order: u8) -> bool {
        let command = EditorCommand::AddBond {
            begin: CommandAnchor::from(&anchor),
            end: CommandAnchor::from(&end),
            order,
            variant: self.state.tool.bond_variant,
        };
        self.with_command(command, |engine| {
            engine.add_bond_between_untracked(anchor, end, order)
        })
    }

    fn add_bond_between_untracked(
        &mut self,
        anchor: BondAnchor,
        end: BondAnchor,
        order: u8,
    ) -> bool {
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists(begin_id, end_id) {
                return false;
            }
        }
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        let begin_id = match anchor.node_id {
            Some(node_id) => node_id,
            None => self.insert_carbon(anchor.point),
        };
        let end_id = match end.node_id {
            Some(node_id) => node_id,
            None => self.insert_carbon(end.point),
        };
        if begin_id == end_id || self.bond_exists(&begin_id, &end_id) {
            self.undo_stack.pop();
            return false;
        }
        let bond_id = self.next_id("b");
        let pending_double =
            self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1));
        let pending_line_styles = self.pending_line_styles();
        let pending_line_weights = self.pending_line_weights();
        let pending_stereo = self.pending_bond_stereo();
        let mut entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        entry.fragment.bonds.push(Bond {
            id: bond_id.clone(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: pending_double,
            stereo: pending_stereo,
            stroke_width: self.options.bond_stroke_world_cm().value(),
            line_styles: pending_line_styles,
            line_weights: pending_line_weights,
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            &bond_id,
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            &bond_id,
        );
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        true
    }

    fn preview_document(&self) -> Option<ChemcoreDocument> {
        if let Some(preview_document) = self.template_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_document() {
            return Some(preview_document);
        }
        if let Some(drag) = self.arrow_drag.as_ref().filter(|drag| drag.has_dragged) {
            let end = drag.end?;
            let mut document = self.state.document.clone();
            ensure_arrow_style(&mut document);
            document.objects.push(self.arrow_scene_object(
                drag.start,
                end,
                "__preview_arrow".to_string(),
            ));
            return Some(document);
        }
        let drag = self.drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let end_anchor = if let Some(target) = drag.target.clone() {
            target
        } else {
            BondAnchor {
                node_id: None,
                point: drag.preview_end?,
                label_anchor: None,
            }
        };
        self.document_with_preview_bond(&drag.anchor, &end_anchor, self.pending_bond_order())
    }

    fn pointer_down_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.shape_drag = Some(ShapeDragState {
            start: point,
            current: point,
            has_dragged: false,
        });
    }

    fn pointer_move_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay = OverlayState::default();
        if let Some(mut drag) = self.shape_drag.take() {
            drag.current = point;
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.preview = Some(BondPreview {
                    start: point,
                    end: point,
                });
            }
            self.shape_drag = Some(drag);
        }
    }

    fn pointer_up_shape(&mut self, event: PointerEvent) {
        let Some(mut drag) = self.shape_drag.take() else {
            return;
        };
        drag.current = event.point();
        if drag.start.distance(drag.current) < DRAG_START_THRESHOLD {
            self.state.overlay = OverlayState::default();
            return;
        }
        drag.has_dragged = true;
        let command = EditorCommand::AddShape {
            kind: self.state.tool.shape_kind,
            style: self.state.tool.shape_style,
            color: self.state.tool.shape_color.clone(),
            begin: CommandAnchor::from(drag.start),
            end: CommandAnchor::from(drag.current),
        };
        self.with_command(command, |engine| engine.insert_shape_from_drag(&drag));
        self.state.overlay = OverlayState::default();
    }

    fn shape_preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.shape_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.state.document.clone();
        let style_id = "__preview_shape_style".to_string();
        document
            .styles
            .insert(style_id.clone(), self.pending_shape_style());
        document.objects.push(self.shape_scene_object(
            drag.start,
            drag.current,
            "__preview_shape".to_string(),
            style_id,
        )?);
        Some(document)
    }

    fn insert_shape_from_drag(&mut self, drag: &ShapeDragState) -> bool {
        let object_id = self.next_id("obj_shape");
        let style_id = format!("style_{object_id}");
        let Some(object) = self.shape_scene_object(
            drag.start,
            drag.current,
            object_id.clone(),
            style_id.clone(),
        ) else {
            return false;
        };
        self.push_undo_snapshot();
        self.state
            .document
            .styles
            .insert(style_id, self.pending_shape_style());
        self.state.document.objects.push(object);
        true
    }

    fn shape_scene_object(
        &self,
        start: Point,
        current: Point,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        let (transform, bbox, extra) = match self.state.tool.shape_kind {
            ShapeKind::Circle => {
                let radius = start.distance(current);
                if radius <= crate::EPSILON {
                    return None;
                }
                let angle = angle_between(start, current);
                let major = current;
                let minor = start.translated(direction_from_angle(angle + 90.0).scaled(radius));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!("circle"));
                extra.insert("center".to_string(), json!([start.x, start.y]));
                extra.insert("majorAxisEnd".to_string(), json!([major.x, major.y]));
                extra.insert("minorAxisEnd".to_string(), json!([minor.x, minor.y]));
                (
                    crate::Transform::identity(),
                    [
                        start.x - radius,
                        start.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    ],
                    extra,
                )
            }
            ShapeKind::Ellipse => {
                let major_radius = start.distance(current);
                if major_radius <= crate::EPSILON {
                    return None;
                }
                let angle = nearest_angle(angle_between(start, current), GLOBAL_SNAP_ANGLES);
                let major = start.translated(direction_from_angle(angle).scaled(major_radius));
                let minor_radius = major_radius * ELLIPSE_MINOR_AXIS_RATIO;
                let minor =
                    start.translated(direction_from_angle(angle + 90.0).scaled(minor_radius));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!("ellipse"));
                extra.insert("center".to_string(), json!([start.x, start.y]));
                extra.insert("majorAxisEnd".to_string(), json!([major.x, major.y]));
                extra.insert("minorAxisEnd".to_string(), json!([minor.x, minor.y]));
                (
                    crate::Transform::identity(),
                    [
                        start.x - major_radius,
                        start.y - major_radius,
                        major_radius * 2.0,
                        major_radius * 2.0,
                    ],
                    extra,
                )
            }
            ShapeKind::RoundRect | ShapeKind::Rect => {
                let x1 = start.x.min(current.x);
                let y1 = start.y.min(current.y);
                let width = (current.x - start.x).abs();
                let height = (current.y - start.y).abs();
                if width <= crate::EPSILON || height <= crate::EPSILON {
                    return None;
                }
                let mut extra = BTreeMap::new();
                extra.insert(
                    "kind".to_string(),
                    json!(if self.state.tool.shape_kind == ShapeKind::RoundRect {
                        "roundRect"
                    } else {
                        "rect"
                    }),
                );
                if self.state.tool.shape_kind == ShapeKind::RoundRect {
                    extra.insert(
                        "cornerRadius".to_string(),
                        json!(ROUND_RECT_CORNER_RADIUS.min(width * 0.5).min(height * 0.5)),
                    );
                }
                (
                    crate::Transform {
                        translate: [x1, y1],
                        rotate: 0.0,
                        scale: [1.0, 1.0],
                    },
                    [0.0, 0.0, width, height],
                    extra,
                )
            }
        };
        Some(SceneObject {
            id: object_id,
            object_type: "shape".to_string(),
            name: "shape".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform,
            style_ref: Some(style_id),
            meta: json!({
                "source": "editor",
                "cdxml": self.pending_shape_cdxml_meta(),
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some(bbox),
                extra,
            },
        })
    }

    fn pending_shape_style(&self) -> JsonValue {
        let color = self.state.tool.shape_color.clone();
        match self.state.tool.shape_style {
            ShapeStyle::Solid => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": SHAPE_STROKE_WIDTH,
                "dashArray": [],
            }),
            ShapeStyle::Dashed => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": SHAPE_STROKE_WIDTH,
                "dashArray": [SHAPE_DASH_LENGTH],
            }),
            ShapeStyle::Shaded => json!({
                "kind": "shape",
                "fill": color,
                "stroke": color,
                "strokeWidth": SHAPE_STROKE_WIDTH,
                "dashArray": [],
                "shaded": true,
            }),
            ShapeStyle::Filled => json!({
                "kind": "shape",
                "fill": color,
                "stroke": null,
                "strokeWidth": 0.0,
                "dashArray": [],
            }),
            ShapeStyle::Shadowed => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": SHAPE_STROKE_WIDTH,
                "dashArray": [],
                "shadow": true,
            }),
        }
    }

    fn pending_shape_cdxml_meta(&self) -> JsonValue {
        json!({
            "graphicType": match self.state.tool.shape_kind {
                ShapeKind::Circle | ShapeKind::Ellipse => "Oval",
                ShapeKind::RoundRect | ShapeKind::Rect => "Rectangle",
            },
            "ovalType": match (self.state.tool.shape_kind, self.state.tool.shape_style) {
                (ShapeKind::Circle, ShapeStyle::Solid) => "Circle",
                (ShapeKind::Circle, ShapeStyle::Dashed) => "Circle Dashed",
                (ShapeKind::Circle, ShapeStyle::Shaded) => "Circle Shaded",
                (ShapeKind::Circle, ShapeStyle::Filled) => "Circle Filled",
                (ShapeKind::Circle, ShapeStyle::Shadowed) => "Circle Shadowed",
                (ShapeKind::Ellipse, ShapeStyle::Solid) => "",
                (ShapeKind::Ellipse, ShapeStyle::Dashed) => "Dashed",
                (ShapeKind::Ellipse, ShapeStyle::Shaded) => "Shaded",
                (ShapeKind::Ellipse, ShapeStyle::Filled) => "Filled",
                (ShapeKind::Ellipse, ShapeStyle::Shadowed) => "Shadowed",
                _ => "",
            },
            "rectangleType": match (self.state.tool.shape_kind, self.state.tool.shape_style) {
                (ShapeKind::RoundRect, ShapeStyle::Solid) => "RoundEdge",
                (ShapeKind::RoundRect, ShapeStyle::Dashed) => "RoundEdge Dashed",
                (ShapeKind::RoundRect, ShapeStyle::Shaded) => "RoundEdge Shaded",
                (ShapeKind::RoundRect, ShapeStyle::Filled) => "RoundEdge Filled",
                (ShapeKind::RoundRect, ShapeStyle::Shadowed) => "RoundEdge Shadow",
                (ShapeKind::Rect, ShapeStyle::Solid) => "Plain",
                (ShapeKind::Rect, ShapeStyle::Dashed) => "Dashed",
                (ShapeKind::Rect, ShapeStyle::Shaded) => "Shaded",
                (ShapeKind::Rect, ShapeStyle::Filled) => "Filled",
                (ShapeKind::Rect, ShapeStyle::Shadowed) => "Shadow",
                _ => "",
            },
        })
    }

    fn next_shape_z_index(&self) -> i32 {
        self.state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(10)
            + 1
    }

    fn document_with_preview_bond(
        &self,
        anchor: &BondAnchor,
        end: &BondAnchor,
        order: u8,
    ) -> Option<ChemcoreDocument> {
        let mut document = self.state.document.clone();
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_in_document(&document, begin_id, end_id) {
                return None;
            }
        }
        let mut entry = document.editable_fragment_mut()?;
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(anchor.point);
                let node_id = "__preview_node_begin".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(end.point);
                let node_id = "__preview_node_end".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        if begin_id == end_id || self.bond_exists_in_fragment(entry.fragment, &begin_id, &end_id) {
            return None;
        }
        entry.fragment.bonds.push(Bond {
            id: "__preview_bond".to_string(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1)),
            stereo: self.pending_bond_stereo(),
            stroke_width: self.options.bond_stroke_world_cm().value(),
            line_styles: self.pending_line_styles(),
            line_weights: self.pending_line_weights(),
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            "__preview_bond",
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            "__preview_bond",
        );
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        Some(document)
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
        self.with_command(
            EditorCommand::CycleBondStyle {
                bond_id: bond_id.to_string(),
                variant: self.state.tool.bond_variant,
            },
            |engine| engine.cycle_bond_center_style_untracked(bond_id),
        )
    }

    fn cycle_bond_center_style_untracked(&mut self, bond_id: &str) -> bool {
        let (current_order, was_double_before) = self
            .state
            .document
            .editable_fragment()
            .and_then(|entry| entry.fragment.bonds.iter().find(|bond| bond.id == bond_id))
            .map(|bond| (bond.order, bond.order == 2 && bond.double.is_some()))
            .unwrap_or((1, false));
        let default_side = self
            .preferred_double_bond_side(bond_id)
            .unwrap_or(DoubleBondPlacement::Right);
        let default_placement =
            if current_order == 1 && self.should_default_center_double_bond(bond_id) {
                DoubleBondPlacement::Center
            } else {
                default_side
            };
        let should_freeze_after_change = was_double_before;
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.id == bond_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let changed = match self.state.tool.bond_variant {
            BondVariant::Single => apply_single_tool_center_style(bond, default_placement),
            BondVariant::Double => apply_double_tool_center_style(bond, default_placement),
            BondVariant::Triple => replace_with_plain_triple_bond_style(bond),
            BondVariant::Dashed => cycle_dashed_bond_center_style(bond, default_placement),
            BondVariant::DashedDouble => {
                cycle_dashed_double_bond_tool_center_style(bond, default_placement)
            }
            BondVariant::Bold => cycle_bold_bond_center_style(bond, default_placement),
            BondVariant::BoldDashed => replace_with_bold_dashed_bond_style(bond),
            BondVariant::Wedge | BondVariant::HashedWedge => {
                replace_with_stereo_bond_style(bond, self.state.tool.bond_variant)
            }
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        if let Some(double) = bond.double.as_mut() {
            double.frozen = should_freeze_after_change;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(mut entry) = self.undo_stack.pop() else {
            return false;
        };
        let after = entry
            .after
            .clone()
            .unwrap_or_else(|| self.state.document.clone());
        self.restore_document(entry.before.clone());
        entry.after = Some(after);
        self.redo_stack.push(entry);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };
        let Some(after) = entry.after.clone() else {
            return false;
        };
        self.restore_document(after);
        self.undo_stack.push(entry);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn drag_target_endpoint(&self, anchor: &BondAnchor, point: Point) -> Option<EndpointHit> {
        hit_test_endpoint_excluding(
            &self.state.document,
            point,
            ENDPOINT_HIT_RADIUS,
            anchor.node_id.as_deref(),
        )
    }

    fn endpoint_anchor_near(&self, anchor: &BondAnchor, point: Point) -> Option<BondAnchor> {
        let target = self.drag_target_endpoint(anchor, point)?;
        Some(BondAnchor {
            node_id: Some(target.node_id),
            point: target.point,
            label_anchor: target.label_anchor,
        })
    }

    fn refresh_bond_mode_hover(&mut self, point: Point) {
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
        if self.state.overlay.hover_endpoint.is_none() && can_focus_bond_center(&self.state.tool) {
            self.state.overlay.hover_bond_center =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
        }
    }

    fn bond_exists(&self, begin_id: &str, end_id: &str) -> bool {
        self.bond_exists_in_document(&self.state.document, begin_id, end_id)
    }

    fn bond_exists_in_document(
        &self,
        document: &ChemcoreDocument,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        let Some(entry) = document.editable_fragment() else {
            return false;
        };
        self.bond_exists_in_fragment(entry.fragment, begin_id, end_id)
    }

    fn bond_exists_in_fragment(
        &self,
        fragment: &crate::MoleculeFragment,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        fragment.bonds.iter().any(|bond| {
            (bond.begin == begin_id && bond.end == end_id)
                || (bond.begin == end_id && bond.end == begin_id)
        })
    }

    fn insert_carbon(&mut self, point: Point) -> String {
        let node_id = self.next_id("n");
        let entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        entry
            .fragment
            .nodes
            .push(crate::Node::carbon(node_id.clone(), local));
        node_id
    }

    fn next_id(&mut self, prefix: &str) -> String {
        let value = self.next_id;
        self.next_id += 1;
        format!("{prefix}_{value}")
    }

    fn with_command<F>(&mut self, command: EditorCommand, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !self.command_context.is_empty() {
            return apply(self);
        }
        let undo_len = self.undo_stack.len();
        self.command_context.push(command.clone());
        let changed = apply(self);
        self.command_context.pop();
        if changed {
            self.finalize_command_history(undo_len, command);
        }
        changed
    }

    fn finalize_command_history(&mut self, undo_len: usize, command: EditorCommand) {
        if self.undo_stack.len() <= undo_len {
            if let Some(entry) = self.undo_stack.last_mut() {
                if entry.command == command {
                    entry.after = Some(self.state.document.clone());
                }
            }
            return;
        }
        let before = self.undo_stack[undo_len].before.clone();
        self.undo_stack.truncate(undo_len);
        self.undo_stack.push(HistoryEntry {
            command,
            before,
            after: Some(self.state.document.clone()),
        });
    }

    fn current_history_command(&self) -> EditorCommand {
        self.command_context
            .last()
            .cloned()
            .unwrap_or_else(|| EditorCommand::LegacyMutation {
                label: "unclassified-mutation".to_string(),
            })
    }

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(HistoryEntry::new(
            self.current_history_command(),
            self.state.document.clone(),
        ));
        self.redo_stack.clear();
    }

    fn restore_document(&mut self, document: ChemcoreDocument) {
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.next_id = self.infer_next_id();
    }

    fn infer_next_id(&self) -> u64 {
        let mut max_id = 0;
        for id in self
            .state
            .document
            .objects
            .iter()
            .map(|object| object.id.as_str())
        {
            if let Some((_, suffix)) = id.rsplit_once('_') {
                if let Ok(value) = suffix.parse::<u64>() {
                    max_id = max_id.max(value);
                }
            }
        }
        if let Some(entry) = self.state.document.editable_fragment() {
            for id in entry
                .fragment
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .chain(entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
            {
                if let Some((_, suffix)) = id.rsplit_once('_') {
                    if let Ok(value) = suffix.parse::<u64>() {
                        max_id = max_id.max(value);
                    }
                }
            }
        }
        max_id + 1
    }

    fn preferred_double_bond_side(&self, bond_id: &str) -> Option<DoubleBondPlacement> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))?;
        preferred_double_bond_side_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
    }

    fn should_default_center_double_bond(&self, bond_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))
        else {
            return false;
        };
        should_default_center_double_bond_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
    }

    fn pending_bond_order(&self) -> u8 {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble => 2,
            BondVariant::Triple => 3,
            _ => 1,
        }
    }

    fn pending_double_state_for_new_bond(
        &self,
        begin_id: &str,
        end_id: &str,
        order: u8,
    ) -> Option<DoubleBond> {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble if order >= 2 => {
                let placement = if self.should_default_center_for_new_bond(begin_id, end_id) {
                    DoubleBondPlacement::Center
                } else {
                    let entry = self.state.document.editable_fragment()?;
                    preferred_double_bond_side_for_segment(entry.fragment, begin_id, end_id, None)
                        .unwrap_or(DoubleBondPlacement::Right)
                };
                Some(DoubleBond {
                    placement,
                    center_exit_side: None,
                    frozen: false,
                })
            }
            _ => None,
        }
    }

    fn should_default_center_for_new_bond(&self, begin_id: &str, end_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        should_default_center_double_bond_for_segment(entry.fragment, begin_id, end_id, None)
    }

    fn pending_line_styles(&self) -> BondLineStyles {
        match self.state.tool.bond_variant {
            BondVariant::Dashed | BondVariant::BoldDashed => {
                return BondLineStyles {
                    main: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            BondVariant::DashedDouble => {
                return BondLineStyles {
                    right: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            _ => {}
        }
        BondLineStyles::default()
    }

    fn pending_bond_stereo(&self) -> Option<BondStereo> {
        match self.state.tool.bond_variant {
            BondVariant::Wedge => Some(BondStereo {
                kind: "solid-wedge".to_string(),
                wide_end: "end".to_string(),
            }),
            BondVariant::HashedWedge => Some(BondStereo {
                kind: "hashed-wedge".to_string(),
                wide_end: "end".to_string(),
            }),
            _ => None,
        }
    }

    fn pending_line_weights(&self) -> BondLineWeights {
        match self.state.tool.bond_variant {
            BondVariant::Bold | BondVariant::BoldDashed => {
                return BondLineWeights {
                    main: BondLineWeight::Bold,
                    ..BondLineWeights::default()
                };
            }
            _ => {}
        }
        BondLineWeights::default()
    }
}

fn update_terminal_double_bond_placement_after_new_attachment(
    fragment: &mut crate::MoleculeFragment,
    attached_node_id: &str,
    new_bond_id: &str,
) {
    let connected_bond_ids: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == attached_node_id || bond.end == attached_node_id)
        .map(|bond| bond.id.clone())
        .collect();
    for bond_id in connected_bond_ids {
        if bond_id != new_bond_id {
            update_unfrozen_double_bond_auto_placement(fragment, &bond_id, new_bond_id);
        }
    }
}

#[derive(Default)]
struct SegmentEndpointSideCounts {
    begin_left: usize,
    begin_right: usize,
    end_left: usize,
    end_right: usize,
}

fn connected_attachment_side_counts_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<SegmentEndpointSideCounts> {
    let begin = fragment.nodes.iter().find(|node| node.id == begin_id)?;
    let end = fragment.nodes.iter().find(|node| node.id == end_id)?;
    let begin_point = begin.point();
    let end_point = end.point();
    let axis_x = end_point.x - begin_point.x;
    let axis_y = end_point.y - begin_point.y;
    let axis_length = axis_x.hypot(axis_y);
    if axis_length <= crate::EPSILON {
        return None;
    }
    let normal_x = -axis_y / axis_length;
    let normal_y = axis_x / axis_length;

    let mut counts = SegmentEndpointSideCounts::default();
    for other in &fragment.bonds {
        if ignored_bond_id.is_some_and(|ignored| other.id == ignored) {
            continue;
        }
        let (shared_id, shared_is_begin) = if other.begin == begin_id || other.end == begin_id {
            (Some(begin_id), true)
        } else if other.begin == end_id || other.end == end_id {
            (Some(end_id), false)
        } else {
            (None, false)
        };
        let Some(shared_id) = shared_id else {
            continue;
        };
        let other_id = if other.begin == shared_id {
            other.end.as_str()
        } else {
            other.begin.as_str()
        };
        let Some(shared_node) = fragment.nodes.iter().find(|node| node.id == shared_id) else {
            continue;
        };
        let Some(other_node) = fragment.nodes.iter().find(|node| node.id == other_id) else {
            continue;
        };
        let side_score = (other_node.position[0] - shared_node.position[0]) * normal_x
            + (other_node.position[1] - shared_node.position[1]) * normal_y;
        if side_score < -crate::EPSILON {
            if shared_is_begin {
                counts.begin_left += 1;
            } else {
                counts.end_left += 1;
            }
        } else if side_score > crate::EPSILON {
            if shared_is_begin {
                counts.begin_right += 1;
            } else {
                counts.end_right += 1;
            }
        }
    }

    Some(counts)
}

fn should_default_center_double_bond_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> bool {
    let Some(counts) =
        connected_attachment_side_counts_for_segment(fragment, begin_id, end_id, ignored_bond_id)
    else {
        return false;
    };
    endpoint_should_default_center(
        counts.begin_left,
        counts.begin_right,
        counts.end_left + counts.end_right,
    ) || endpoint_should_default_center(
        counts.end_left,
        counts.end_right,
        counts.begin_left + counts.begin_right,
    )
}

fn endpoint_should_default_center(
    left_count: usize,
    right_count: usize,
    other_total: usize,
) -> bool {
    other_total == 0
        && ((left_count >= 2 && right_count == 0) || (right_count >= 2 && left_count == 0))
}

fn preferred_double_bond_side_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<DoubleBondPlacement> {
    let begin = fragment.nodes.iter().find(|node| node.id == begin_id)?;
    let end = fragment.nodes.iter().find(|node| node.id == end_id)?;
    let begin_point = begin.point();
    let end_point = end.point();
    let dx = end_point.x - begin_point.x;
    let dy = end_point.y - begin_point.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return Some(DoubleBondPlacement::Left);
    }
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let mut score = 0.0;
    let mut attachment_count = 0usize;
    for other in &fragment.bonds {
        if ignored_bond_id.is_some_and(|ignored| other.id == ignored) {
            continue;
        }
        if other.begin == begin_id || other.end == begin_id {
            let other_id = if other.begin == begin_id {
                &other.end
            } else {
                &other.begin
            };
            if let Some(neighbor) = fragment.nodes.iter().find(|node| &node.id == other_id) {
                let point = neighbor.point();
                attachment_count += 1;
                score +=
                    (point.x - begin_point.x) * normal_x + (point.y - begin_point.y) * normal_y;
            }
        } else if other.begin == end_id || other.end == end_id {
            let other_id = if other.begin == end_id {
                &other.end
            } else {
                &other.begin
            };
            if let Some(neighbor) = fragment.nodes.iter().find(|node| &node.id == other_id) {
                let point = neighbor.point();
                attachment_count += 1;
                score += (point.x - end_point.x) * normal_x + (point.y - end_point.y) * normal_y;
            }
        }
    }
    if attachment_count == 0 {
        return None;
    }
    if score <= 0.0 {
        Some(DoubleBondPlacement::Left)
    } else {
        Some(DoubleBondPlacement::Right)
    }
}

fn update_unfrozen_double_bond_auto_placement(
    fragment: &mut crate::MoleculeFragment,
    double_bond_id: &str,
    new_bond_id: &str,
) {
    let Some(double_index) = fragment
        .bonds
        .iter()
        .position(|bond| bond.id == double_bond_id && bond.order == 2)
    else {
        return;
    };
    let Some(double) = fragment.bonds[double_index].double.as_ref() else {
        return;
    };
    if double.frozen {
        return;
    }

    let bond = fragment.bonds[double_index].clone();
    let Some(begin) = fragment.nodes.iter().find(|node| node.id == bond.begin) else {
        return;
    };
    let Some(end) = fragment.nodes.iter().find(|node| node.id == bond.end) else {
        return;
    };
    let begin_point = begin.point();
    let end_point = end.point();
    let axis_x = end_point.x - begin_point.x;
    let axis_y = end_point.y - begin_point.y;
    let axis_length = axis_x.hypot(axis_y);
    if axis_length <= crate::EPSILON {
        return;
    }
    let normal_x = -axis_y / axis_length;
    let normal_y = axis_x / axis_length;

    let mut left_count = 0usize;
    let mut right_count = 0usize;
    let mut new_bond_side: Option<DoubleBondPlacement> = None;
    for other in &fragment.bonds {
        if other.id == bond.id {
            continue;
        }
        let shared_id = if other.begin == bond.begin || other.end == bond.begin {
            Some(bond.begin.as_str())
        } else if other.begin == bond.end || other.end == bond.end {
            Some(bond.end.as_str())
        } else {
            None
        };
        let Some(shared_id) = shared_id else {
            continue;
        };
        let other_id = if other.begin == shared_id {
            other.end.as_str()
        } else {
            other.begin.as_str()
        };
        let Some(shared_node) = fragment.nodes.iter().find(|node| node.id == shared_id) else {
            continue;
        };
        let Some(other_node) = fragment.nodes.iter().find(|node| node.id == other_id) else {
            continue;
        };
        let side_score = (other_node.position[0] - shared_node.position[0]) * normal_x
            + (other_node.position[1] - shared_node.position[1]) * normal_y;
        let side = if side_score < -crate::EPSILON {
            Some(DoubleBondPlacement::Left)
        } else if side_score > crate::EPSILON {
            Some(DoubleBondPlacement::Right)
        } else {
            None
        };
        match side {
            Some(DoubleBondPlacement::Left) => left_count += 1,
            Some(DoubleBondPlacement::Right) => right_count += 1,
            _ => {}
        }
        if other.id == new_bond_id {
            new_bond_side = side;
        }
    }

    let placement = if left_count > right_count {
        Some(DoubleBondPlacement::Left)
    } else if right_count > left_count {
        Some(DoubleBondPlacement::Right)
    } else {
        new_bond_side
    };
    let Some(placement) = placement else {
        return;
    };
    fragment.bonds[double_index].double = Some(crate::DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
}

fn opposite_double_bond_placement(placement: DoubleBondPlacement) -> DoubleBondPlacement {
    match placement {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => DoubleBondPlacement::Right,
    }
}

fn apply_single_tool_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if is_plain_single_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    replace_with_plain_single_bond_style(bond)
}

fn apply_double_tool_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if is_plain_single_bond(bond) || is_plain_triple_bond(bond) {
        return replace_with_plain_double_bond_style(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_bold_family_bond(bond) {
        return if bond.order == 2 {
            cycle_bold_double_bond_style(bond, Some(default_placement))
        } else {
            cycle_bold_single_bond_style(bond, Some(default_placement))
        };
    }
    replace_with_plain_double_bond_style(bond, default_placement)
}

fn cycle_dashed_bond_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return cycle_dashed_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_dashed_bond_style(bond)
}

fn cycle_dashed_double_bond_tool_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return advance_plain_dashed_double_cycle(bond, default_placement);
    }
    replace_with_plain_dashed_double_bond_style(bond, default_placement)
}

fn cycle_bold_bond_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        if is_bold_family_bond(bond) {
            return cycle_bold_double_bond_style(bond, Some(default_placement));
        }
        let placement = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .unwrap_or(default_placement);
        return init_bold_double_bond_style(bond, placement, default_placement);
    }
    if bond.order == 1 && !has_stereo_style(bond) && all_line_patterns_solid(bond) {
        return cycle_bold_single_bond_style(bond, Some(default_placement));
    }
    if is_bold_family_bond(bond) && bond.order == 2 {
        return cycle_bold_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_bold_bond_style(bond)
}

fn cycle_dashed_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            let side_pattern = outer_line_pattern_mut(&mut bond.line_styles, placement);
            if *side_pattern != BondLinePattern::Dashed {
                *side_pattern = BondLinePattern::Dashed;
            } else if bond.line_styles.main != BondLinePattern::Dashed {
                bond.line_styles.main = BondLinePattern::Dashed;
            } else {
                let exit_side = opposite_double_bond_placement(placement);
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(exit_side),
                    frozen: false,
                });
                bond.line_styles.main = BondLinePattern::Solid;
                bond.line_styles.left = BondLinePattern::Dashed;
                bond.line_styles.right = BondLinePattern::Dashed;
            }
            true
        }
        DoubleBondPlacement::Center => {
            let dashed_sides = centered_dashed_sides(&bond.line_styles);
            if dashed_sides.is_empty() {
                *outer_line_pattern_mut(&mut bond.line_styles, default_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: None,
                    frozen: false,
                });
                return true;
            }
            if dashed_sides.len() == 1 {
                let first_dashed = dashed_sides[0];
                let second_side = opposite_double_bond_placement(first_dashed);
                *outer_line_pattern_mut(&mut bond.line_styles, second_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(first_dashed)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or(default_side);
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_styles.main = BondLinePattern::Solid;
            bond.line_styles.left = BondLinePattern::Solid;
            bond.line_styles.right = BondLinePattern::Solid;
            *outer_line_pattern_mut(&mut bond.line_styles, exit_side) = BondLinePattern::Dashed;
            true
        }
    }
}

fn advance_plain_dashed_double_cycle(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    let opposite_placement = opposite_double_bond_placement(default_placement);
    let dashed_side = current_dashed_double_side(bond);
    let next_placement = match bond.double.as_ref().map(|double| double.placement) {
        Some(current) if current == default_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(default_placement) => {
            opposite_placement
        }
        Some(current) if current == opposite_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(opposite_placement) => {
            default_placement
        }
        _ => default_placement,
    };

    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.double = Some(DoubleBond {
        placement: next_placement,
        center_exit_side: None,
        frozen: false,
    });

    let next_dashed_side = match next_placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => next_placement,
        DoubleBondPlacement::Center => dashed_side.unwrap_or(default_placement),
    };
    *outer_line_pattern_mut(&mut bond.line_styles, next_dashed_side) = BondLinePattern::Dashed;
    true
}

fn advance_plain_double_cycle(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    let opposite_placement = opposite_double_bond_placement(default_placement);
    let next_placement = if bond.order == 1 {
        default_placement
    } else {
        match bond.double.as_ref().map(|double| double.placement) {
            Some(current) if current == default_placement => DoubleBondPlacement::Center,
            Some(DoubleBondPlacement::Center) => opposite_placement,
            Some(current) if current == opposite_placement => default_placement,
            _ => default_placement,
        }
    };
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement: next_placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_single_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_double_bond_style(bond: &mut Bond, placement: DoubleBondPlacement) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_triple_bond_style(bond: &mut Bond) -> bool {
    bond.order = 3;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    *outer_line_pattern_mut(&mut bond.line_styles, placement) = BondLinePattern::Dashed;
    true
}

fn current_dashed_double_side(bond: &Bond) -> Option<DoubleBondPlacement> {
    let left_dashed = bond.line_styles.left == BondLinePattern::Dashed;
    let right_dashed = bond.line_styles.right == BondLinePattern::Dashed;
    match (left_dashed, right_dashed) {
        (true, false) => Some(DoubleBondPlacement::Left),
        (false, true) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

fn replace_with_plain_bold_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

fn replace_with_bold_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

fn replace_with_stereo_bond_style(bond: &mut Bond, variant: BondVariant) -> bool {
    let kind = match variant {
        BondVariant::Wedge => "solid-wedge",
        BondVariant::HashedWedge => "hashed-wedge",
        _ => return false,
    };
    let current_wide_end = bond
        .stereo
        .as_ref()
        .map(|stereo| stereo.wide_end.as_str())
        .unwrap_or("end");
    let next_wide_end = match bond.stereo.as_ref() {
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "end" => "begin",
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "begin" => "end",
        Some(_) => current_wide_end,
        None => "end",
    };
    bond.order = 1;
    bond.double = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.stereo = Some(BondStereo {
        kind: kind.to_string(),
        wide_end: next_wide_end.to_string(),
    });
    true
}

fn init_bold_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
    default_placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            bond.double = Some(DoubleBond {
                placement,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
        }
        DoubleBondPlacement::Center => {
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(opposite_double_bond_placement(default_placement)),
                frozen: false,
            });
            *outer_line_weight_mut(&mut bond.line_weights, default_placement) =
                BondLineWeight::Bold;
        }
    }
    true
}

fn cycle_bold_single_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    if bond.line_weights.main != BondLineWeight::Bold {
        bond.line_weights.main = BondLineWeight::Bold;
        return true;
    }

    let side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement: side,
        center_exit_side: None,
        frozen: false,
    });
    bond.line_weights.main = BondLineWeight::Bold;
    bond.line_weights.left = BondLineWeight::Normal;
    bond.line_weights.right = BondLineWeight::Normal;
    true
}

fn cycle_bold_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            if bond.line_weights.main != BondLineWeight::Bold {
                bond.line_weights.main = BondLineWeight::Bold;
                return true;
            }

            let exit_side = opposite_double_bond_placement(placement);
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(exit_side),
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Normal;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            *outer_line_weight_mut(&mut bond.line_weights, placement) = BondLineWeight::Bold;
            true
        }
        DoubleBondPlacement::Center => {
            let bold_sides = centered_bold_sides(&bond.line_weights);
            if bold_sides.is_empty() {
                *outer_line_weight_mut(&mut bond.line_weights, default_side) = BondLineWeight::Bold;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(default_side)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or_else(|| opposite_double_bond_placement(bold_sides[0]));
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            true
        }
    }
}

fn is_plain_single_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_double_bond(bond: &Bond) -> bool {
    bond.order == 2
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_triple_bond(bond: &Bond) -> bool {
    bond.order == 3
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_bold_family_bond(bond: &Bond) -> bool {
    bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && (bond.line_weights.main == BondLineWeight::Bold
            || bond.line_weights.left == BondLineWeight::Bold
            || bond.line_weights.right == BondLineWeight::Bold)
}

fn has_stereo_style(bond: &Bond) -> bool {
    bond.stereo.is_some()
}

fn arrow_drag_end(start: Point, point: Point, alt_key: bool) -> Point {
    if alt_key {
        return point;
    }
    let dx = point.x - start.x;
    let dy = point.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= crate::EPSILON {
        return point;
    }
    let angle = nearest_angle(crate::angle_between(start, point), GLOBAL_SNAP_ANGLES);
    let direction = crate::direction_from_angle(angle);
    start.translated(direction.scaled(length))
}

fn arrow_head_dimensions(size: ArrowHeadSize, bold: bool) -> (f64, f64, f64) {
    let (length, center_length, width) = match size {
        ArrowHeadSize::Large => (22.5, 19.69, 5.63),
        ArrowHeadSize::Medium => (15.0, 13.13, 3.75),
        ArrowHeadSize::Small => (10.0, 8.75, 2.5),
    };
    if bold {
        (length * 2.0, center_length * 2.0, width * 2.0)
    } else {
        (length, center_length, width)
    }
}

fn arrow_payload_dimensions(
    variant: ArrowVariant,
    size: ArrowHeadSize,
    bold: bool,
) -> (f64, f64, f64) {
    match variant {
        ArrowVariant::Solid => arrow_head_dimensions(size, bold),
        ArrowVariant::Curved | ArrowVariant::CurvedMirror => arrow_head_dimensions(size, bold),
        ArrowVariant::Hollow | ArrowVariant::Open => (12.0, 12.0, 3.0),
    }
}

fn arrow_curve_degrees(curve: ArrowCurve) -> f64 {
    match curve {
        ArrowCurve::Arc270 => 270.0,
        ArrowCurve::Arc180 => 180.0,
        ArrowCurve::Arc120 => 120.0,
        ArrowCurve::Arc90 => 90.0,
    }
}

fn arrow_curve_sweep_degrees(variant: ArrowVariant, curve: ArrowCurve) -> f64 {
    match variant {
        ArrowVariant::Curved => -arrow_curve_degrees(curve),
        ArrowVariant::CurvedMirror => arrow_curve_degrees(curve),
        ArrowVariant::Solid | ArrowVariant::Hollow | ArrowVariant::Open => 0.0,
    }
}

fn arrow_endpoint_enabled(style: ArrowEndpointStyle) -> bool {
    !matches!(style, ArrowEndpointStyle::None)
}

fn arrow_endpoint_payload_name(style: ArrowEndpointStyle) -> &'static str {
    match style {
        ArrowEndpointStyle::None => "none",
        ArrowEndpointStyle::Full => "full",
        ArrowEndpointStyle::Left => "half-left",
        ArrowEndpointStyle::Right => "half-right",
    }
}

fn arrow_no_go_payload_name(no_go: ArrowNoGo) -> &'static str {
    match no_go {
        ArrowNoGo::None => "none",
        ArrowNoGo::Cross => "cross",
        ArrowNoGo::Hash => "hash",
    }
}

fn arrow_variant_name(variant: ArrowVariant) -> &'static str {
    match variant {
        ArrowVariant::Solid => "solid",
        ArrowVariant::Curved => "curved",
        ArrowVariant::CurvedMirror => "curved-mirror",
        ArrowVariant::Hollow => "hollow",
        ArrowVariant::Open => "open",
    }
}

fn round_point(point: Point) -> Point {
    Point::new(crate::round2(point.x), crate::round2(point.y))
}

fn ensure_arrow_style(document: &mut ChemcoreDocument) {
    document
        .styles
        .entry("style_arrow_default".to_string())
        .or_insert_with(|| {
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": crate::DEFAULT_BOND_STROKE,
                "lineCap": "butt",
                "lineJoin": "miter"
            })
        });
}

fn all_line_patterns_solid(bond: &Bond) -> bool {
    bond.line_styles.main == BondLinePattern::Solid
        && bond.line_styles.left == BondLinePattern::Solid
        && bond.line_styles.right == BondLinePattern::Solid
}

fn all_line_weights_normal(bond: &Bond) -> bool {
    bond.line_weights.main == BondLineWeight::Normal
        && bond.line_weights.left == BondLineWeight::Normal
        && bond.line_weights.right == BondLineWeight::Normal
}

fn centered_dashed_sides(line_styles: &BondLineStyles) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_styles.left == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Left);
    }
    if line_styles.right == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn centered_bold_sides(line_weights: &BondLineWeights) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_weights.left == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Left);
    }
    if line_weights.right == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn outer_line_pattern_mut(
    line_styles: &mut BondLineStyles,
    placement: DoubleBondPlacement,
) -> &mut BondLinePattern {
    match placement {
        DoubleBondPlacement::Left => &mut line_styles.left,
        DoubleBondPlacement::Right => &mut line_styles.right,
        DoubleBondPlacement::Center => &mut line_styles.right,
    }
}

fn outer_line_weight_mut(
    line_weights: &mut BondLineWeights,
    placement: DoubleBondPlacement,
) -> &mut BondLineWeight {
    match placement {
        DoubleBondPlacement::Left => &mut line_weights.left,
        DoubleBondPlacement::Right => &mut line_weights.right,
        DoubleBondPlacement::Center => &mut line_weights.right,
    }
}

fn centered_oriented_rect_points(
    start: Point,
    end: Point,
    length_along_bond: f64,
    width_across_bond: f64,
) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let bond_length = dx.hypot(dy);
    let center = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    if bond_length <= crate::EPSILON {
        let half = width_across_bond / 2.0;
        return vec![
            Point::new(center.x - half, center.y - half),
            Point::new(center.x + half, center.y - half),
            Point::new(center.x + half, center.y + half),
            Point::new(center.x - half, center.y + half),
        ];
    }
    let ux = dx / bond_length;
    let uy = dy / bond_length;
    let tx = ux * length_along_bond / 2.0;
    let ty = uy * length_along_bond / 2.0;
    let nx = -uy * width_across_bond / 2.0;
    let ny = ux * width_across_bond / 2.0;
    vec![
        Point::new(center.x - tx + nx, center.y - ty + ny),
        Point::new(center.x + tx + nx, center.y + ty + ny),
        Point::new(center.x + tx - nx, center.y + ty - ny),
        Point::new(center.x - tx - nx, center.y - ty - ny),
    ]
}

impl Engine {
    pub fn options(&self) -> &EditorOptions {
        &self.options
    }

    pub fn set_bond_length_world_cm(&mut self, length: WorldCm) {
        self.options.bond_length = if length.value() > 0.0 {
            length.value()
        } else {
            DEFAULT_BOND_LENGTH
        };
    }

    pub fn set_bond_length(&mut self, length: f64) {
        self.set_bond_length_world_cm(WorldCm(length));
    }
}
