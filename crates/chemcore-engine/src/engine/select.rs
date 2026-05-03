use super::text_edit::{
    endpoint_label_world_bounds, refresh_attached_node_label_geometry_for_all_nodes,
    text_object_world_bounds,
};
use super::{ArrowEditDragState, ArrowEditMode, EditorCommand, Engine};
use crate::{
    angle_between, arrow_object_handle_points, direction_from_angle, fragment_bond_visual_bounds,
    hit_test_arrow_center, hit_test_bond_center, hit_test_endpoint, line_object_points,
    nearest_angle, round2, HoverTextBox, Point, RenderPrimitive, RenderRole, SelectionState,
    BOND_CENTER_HIT_RADIUS, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS,
    ENDPOINT_HIT_RADIUS, GLOBAL_SNAP_ANGLES,
};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeSet, VecDeque};

const SELECTION_NODE_BOX_SIZE: f64 = ENDPOINT_FOCUS_RADIUS * 2.0;
const SELECTION_BOX_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const SELECTION_BOND_DOT_RADIUS: f64 = crate::px_to_cm(3.0);

#[derive(Clone)]
enum SelectHit {
    TextObject { object_id: String },
    ArrowObject { object_id: String },
    Label { node_id: String },
    Node { node_id: String },
    Bond { bond_id: String },
}

#[derive(Clone, Copy)]
struct AxisBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl AxisBounds {
    fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    fn around_point(point: Point, half_size: f64) -> Self {
        Self::new(
            point.x - half_size,
            point.y - half_size,
            point.x + half_size,
            point.y + half_size,
        )
    }

    fn from_array(bounds: [f64; 4]) -> Self {
        Self::new(bounds[0], bounds[1], bounds[2], bounds[3])
    }

    fn include_point(&mut self, point: Point) {
        self.min_x = self.min_x.min(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_x = self.max_x.max(point.x);
        self.max_y = self.max_y.max(point.y);
    }

    fn include_bounds(&mut self, bounds: AxisBounds) {
        self.min_x = self.min_x.min(bounds.min_x);
        self.min_y = self.min_y.min(bounds.min_y);
        self.max_x = self.max_x.max(bounds.max_x);
        self.max_y = self.max_y.max(bounds.max_y);
    }

    fn expanded(self, amount: f64) -> Self {
        Self {
            min_x: self.min_x - amount,
            min_y: self.min_y - amount,
            max_x: self.max_x + amount,
            max_y: self.max_y + amount,
        }
    }
}

struct ComponentSelection {
    node_ids: Vec<String>,
    label_node_ids: Vec<String>,
    bond_ids: Vec<String>,
}

#[derive(Clone, Copy)]
enum FragmentItemKind {
    Node,
    Label,
    Bond,
}

#[derive(Clone, Copy)]
struct FragmentSelectionItem {
    kind: FragmentItemKind,
    bounds: AxisBounds,
    center: Point,
}

#[derive(Clone)]
struct SelectionArrangeItem {
    original_bounds: AxisBounds,
    bounds: AxisBounds,
    node_ids: Vec<String>,
    text_object_ids: Vec<String>,
    mirror_x: Option<f64>,
    mirror_y: Option<f64>,
}

#[derive(Clone)]
struct NodeMoveOriginal {
    node_id: String,
    position: [f64; 2],
}

#[derive(Clone)]
struct TextMoveOriginal {
    object_id: String,
    translate: [f64; 2],
}

#[derive(Clone)]
enum SelectionMoveMode {
    Translate,
    TerminalNode {
        node_id: String,
        pivot: Point,
        length: f64,
    },
}

pub(super) struct SelectionMoveDrag {
    start: Point,
    node_originals: Vec<NodeMoveOriginal>,
    text_originals: Vec<TextMoveOriginal>,
    mode: SelectionMoveMode,
    preserve_selection_after_drag: bool,
    undo_pushed: bool,
    changed: bool,
}

pub(super) struct SelectionRotateDrag {
    center: Point,
    start_angle: f64,
    node_originals: Vec<NodeMoveOriginal>,
    text_originals: Vec<TextMoveOriginal>,
    undo_pushed: bool,
    changed: bool,
}

impl Engine {
    pub fn selection_contains_point(&self, point: Point) -> bool {
        self.selection_hit_bounds()
            .into_iter()
            .any(|bounds| point_in_bounds(point, bounds))
    }

    pub fn hover_arrow_action_at_point(&self, point: Point) -> &'static str {
        match self.hover_arrow_edit_mode_at_point(point) {
            Some(ArrowEditMode::Head) => "head",
            Some(ArrowEditMode::Tail) => "tail",
            Some(ArrowEditMode::Curve) => "curve",
            None => "",
        }
    }

    pub fn begin_hover_arrow_edit(&mut self, point: Point) -> &'static str {
        let Some((object_id, mode, points, curve)) = self.hover_arrow_edit_target_at_point(point)
        else {
            return "";
        };
        self.arrow_edit_drag = Some(ArrowEditDragState {
            object_id,
            mode,
            original_points: points,
            start_pointer: point,
            has_dragged: false,
            current_degrees: curve.abs().round(),
            undo_pushed: false,
        });
        clear_select_hover_overlay(self);
        self.state.overlay.preview = None;
        self.drag = None;
        self.arrow_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        match mode {
            ArrowEditMode::Head => "head",
            ArrowEditMode::Tail => "tail",
            ArrowEditMode::Curve => "curve",
        }
    }

    pub fn update_hover_arrow_edit(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-arrow".to_string(),
            },
            |engine| engine.update_hover_arrow_edit_untracked(point, alt_key),
        )
    }

    fn update_hover_arrow_edit_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        let Some(mut drag) = self.arrow_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) >= DRAG_START_THRESHOLD {
            drag.has_dragged = true;
        }
        if drag.has_dragged {
            self.apply_arrow_edit_drag(&mut drag, point, alt_key);
        }
        self.arrow_edit_drag = Some(drag);
        true
    }

    pub fn finish_hover_arrow_edit(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-arrow".to_string(),
            },
            |engine| engine.finish_hover_arrow_edit_untracked(point, alt_key),
        )
    }

    fn finish_hover_arrow_edit_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        let Some(mut drag) = self.arrow_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) >= DRAG_START_THRESHOLD {
            drag.has_dragged = true;
        }
        if !drag.has_dragged {
            self.hover_select_target(point);
            return false;
        }
        self.apply_arrow_edit_drag(&mut drag, point, alt_key);
        self.hover_select_target(point);
        true
    }

    pub fn active_arrow_edit_degrees(&self) -> f64 {
        self.arrow_edit_drag
            .as_ref()
            .map(|drag| drag.current_degrees)
            .unwrap_or(0.0)
    }

    fn hover_arrow_edit_mode_at_point(&self, point: Point) -> Option<ArrowEditMode> {
        self.hover_arrow_edit_target_at_point(point)
            .map(|(_, mode, _, _)| mode)
    }

    fn hover_arrow_edit_target_at_point(
        &self,
        point: Point,
    ) -> Option<(String, ArrowEditMode, Vec<Point>, f64)> {
        let hover = hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)?;
        if self
            .state
            .selection
            .arrow_objects
            .contains(&hover.object_id)
        {
            return None;
        }
        let mut candidates = Vec::new();
        if let Some(tail) = hover.handles.first() {
            candidates.push((tail.distance(point), ArrowEditMode::Tail));
        }
        if let Some(head) = hover.handles.get(2) {
            candidates.push((head.distance(point), ArrowEditMode::Head));
        }
        if let Some(center) = hover.handles.get(1) {
            candidates.push((center.distance(point), ArrowEditMode::Curve));
        }
        let (_, mode) = candidates
            .into_iter()
            .filter(|(distance, _)| *distance <= ENDPOINT_HIT_RADIUS)
            .min_by(|left, right| left.0.total_cmp(&right.0))?;
        let object = self
            .state
            .document
            .objects
            .iter()
            .find(|object| object.id == hover.object_id)?;
        Some((
            hover.object_id,
            mode,
            line_object_points(object),
            object_arrow_curve(object),
        ))
    }

    fn apply_arrow_edit_drag(
        &mut self,
        drag: &mut ArrowEditDragState,
        point: Point,
        alt_key: bool,
    ) -> bool {
        if drag.original_points.len() < 2 {
            return false;
        }
        if !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        let start = drag.original_points[0];
        let end = *drag.original_points.last().unwrap_or(&start);
        match drag.mode {
            ArrowEditMode::Head => {
                let next_end = snapped_arrow_endpoint(start, point, alt_key);
                update_arrow_object_points(self, &drag.object_id, start, next_end)
            }
            ArrowEditMode::Tail => {
                let next_start = snapped_arrow_endpoint(end, point, alt_key);
                update_arrow_object_points(self, &drag.object_id, next_start, end)
            }
            ArrowEditMode::Curve => {
                let curve = snapped_arrow_curve_from_point(start, end, point, alt_key);
                drag.current_degrees = curve.abs().round();
                update_arrow_object_curve(self, &drag.object_id, curve)
            }
        }
    }

    pub fn begin_selection_move_at_point(
        &mut self,
        point: Point,
        additive: bool,
        _alt_key: bool,
    ) -> bool {
        let mut preserve_selection_after_drag = true;
        if self.state.selection.is_empty() || !self.selection_contains_point(point) {
            let Some(hit) = self.select_hit_at_point(point) else {
                return false;
            };
            preserve_selection_after_drag = selection_contains_hit(&self.state.selection, &hit);
            let mut selection = if additive {
                self.state.selection.clone()
            } else {
                SelectionState::default()
            };
            if !additive {
                selection.region = false;
            }
            add_hit_to_selection(&mut selection, hit);
            self.state.selection = selection;
        }
        let Some(drag) = self.build_selection_move_drag(point, preserve_selection_after_drag)
        else {
            return false;
        };
        self.drag = None;
        clear_select_hover_overlay(self);
        self.selection_drag = Some(drag);
        true
    }

    pub fn begin_selection_rotate(&mut self, point: Point) -> bool {
        let Some(drag) = self.build_selection_rotate_drag(point) else {
            return false;
        };
        self.drag = None;
        self.selection_drag = None;
        clear_select_hover_overlay(self);
        self.selection_rotate_drag = Some(drag);
        true
    }

    pub fn update_selection_rotate(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::RotateSelection, |engine| {
            engine.update_selection_rotate_untracked(point, alt_key)
        })
    }

    fn update_selection_rotate_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_rotate_drag.is_none() {
            return false;
        }
        clear_select_hover_overlay(self);
        self.apply_selection_rotate_drag(point, alt_key);
        true
    }

    pub fn finish_selection_rotate(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::RotateSelection, |engine| {
            engine.finish_selection_rotate_untracked(point, alt_key)
        })
    }

    fn finish_selection_rotate_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_rotate_drag.is_none() {
            return false;
        }
        self.apply_selection_rotate_drag(point, alt_key);
        let changed = self
            .selection_rotate_drag
            .as_ref()
            .is_some_and(|drag| drag.changed);
        self.selection_rotate_drag = None;
        self.hover_select_target(point);
        changed
    }

    pub fn update_selection_move(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::MoveSelection, |engine| {
            engine.update_selection_move_untracked(point, alt_key)
        })
    }

    fn update_selection_move_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_drag.is_none() {
            return false;
        }
        clear_select_hover_overlay(self);
        self.apply_selection_move_drag(point, alt_key);
        true
    }

    pub fn finish_selection_move(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::MoveSelection, |engine| {
            engine.finish_selection_move_untracked(point, alt_key)
        })
    }

    fn finish_selection_move_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_drag.is_none() {
            return false;
        }
        self.apply_selection_move_drag(point, alt_key);
        let changed = self
            .selection_drag
            .as_ref()
            .is_some_and(|drag| drag.changed);
        let should_clear_selection = changed
            && self
                .selection_drag
                .as_ref()
                .is_some_and(|drag| !drag.preserve_selection_after_drag);
        self.selection_drag = None;
        if should_clear_selection {
            self.state.selection = SelectionState::default();
        }
        self.hover_select_target(point);
        changed
    }

    pub fn apply_selection_arrange_command(&mut self, command: &str) -> bool {
        self.with_command(
            EditorCommand::ApplySelectionArrange {
                command: command.to_string(),
            },
            |engine| engine.apply_selection_arrange_command_untracked(command),
        )
    }

    fn apply_selection_arrange_command_untracked(&mut self, command: &str) -> bool {
        let mut items = self.selection_arrange_items();
        if items.is_empty() {
            return false;
        }
        let changed = match command {
            "align-left" => align_items(&mut items, AlignAxis::XMin),
            "align-right" => align_items(&mut items, AlignAxis::XMax),
            "align-top" => align_items(&mut items, AlignAxis::YMin),
            "align-bottom" => align_items(&mut items, AlignAxis::YMax),
            "align-h-center" => align_items(&mut items, AlignAxis::XCenter),
            "align-v-center" => align_items(&mut items, AlignAxis::YCenter),
            "distribute-h" => distribute_items(&mut items, DistributeAxis::Horizontal),
            "distribute-v" => distribute_items(&mut items, DistributeAxis::Vertical),
            "flip-h" => flip_items(&mut items, FlipAxis::Horizontal),
            "flip-v" => flip_items(&mut items, FlipAxis::Vertical),
            _ => false,
        };
        if !changed {
            return false;
        }
        self.push_undo_snapshot();
        apply_arrange_items_to_document(self, &items);
        self.clear_interaction();
        true
    }

    pub fn select_at_point(&mut self, point: Point, additive: bool) {
        let hit = self.select_hit_at_point(point);
        self.state.selection = if let Some(hit) = hit {
            let mut selection = if additive {
                self.state.selection.clone()
            } else {
                SelectionState::default()
            };
            if !additive {
                selection.region = false;
            }
            add_hit_to_selection(&mut selection, hit);
            selection
        } else if additive {
            self.state.selection.clone()
        } else {
            SelectionState::default()
        };
        self.state.overlay.preview = None;
        self.hover_select_target(point);
    }

    pub fn select_component_at_point(&mut self, point: Point, additive: bool) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let hit_node = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            .map(|hit| hit.node_id);
        let hit_bond = if hit_node.is_none() {
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
                .map(|hit| hit.bond_id)
        } else {
            None
        };
        let seed_node_id = if let Some(node_id) = hit_node {
            node_id
        } else if let Some(bond_id) = hit_bond {
            let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == bond_id) else {
                return false;
            };
            bond.begin.clone()
        } else {
            return false;
        };
        let component_node_ids = connected_component_node_ids(entry.fragment, &seed_node_id);
        let component_bond_ids: Vec<String> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                component_node_ids.contains(&bond.begin) && component_node_ids.contains(&bond.end)
            })
            .map(|bond| bond.id.clone())
            .collect();
        let label_node_ids: Vec<String> = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| component_node_ids.contains(&node.id) && node.label.is_some())
            .map(|node| node.id.clone())
            .collect();
        let bracket_ids = bracket_object_ids_containing_component(
            &self.state.document,
            &entry,
            &component_node_ids,
        );
        let mut selection = if additive {
            self.state.selection.clone()
        } else {
            SelectionState::default()
        };
        selection.region = false;
        for node_id in component_node_ids {
            push_unique(&mut selection.nodes, node_id);
        }
        for bond_id in component_bond_ids {
            push_unique(&mut selection.bonds, bond_id);
        }
        for node_id in label_node_ids {
            push_unique(&mut selection.label_nodes, node_id);
        }
        for object_id in bracket_ids {
            push_unique(&mut selection.arrow_objects, object_id);
        }
        self.state.selection = selection;
        self.clear_interaction();
        true
    }

    pub fn select_in_rect(&mut self, start: Point, end: Point, additive: bool) {
        let bounds = AxisBounds::new(start.x, start.y, end.x, end.y);
        let selection = self.collect_region_selection(
            |point| point_in_bounds(point, bounds),
            |segment_start, segment_end| {
                segment_intersects_bounds(segment_start, segment_end, bounds)
            },
            |candidate_bounds| bounds_intersect(bounds, candidate_bounds),
        );
        self.state.selection = merge_selection(self.state.selection.clone(), selection, additive);
        self.clear_interaction();
    }

    pub fn select_in_polygon(&mut self, points: Vec<Point>, additive: bool) {
        if points.len() < 3 {
            return;
        }
        let polygon_bounds = polygon_bounds(&points);
        let selection = self.collect_region_selection(
            |point| point_in_polygon(point, &points),
            |segment_start, segment_end| {
                segment_intersects_polygon(segment_start, segment_end, &points, polygon_bounds)
            },
            |candidate_bounds| {
                bounds_intersect(polygon_bounds, candidate_bounds)
                    && rect_intersects_polygon(candidate_bounds, &points, polygon_bounds)
            },
        );
        self.state.selection = merge_selection(self.state.selection.clone(), selection, additive);
        self.clear_interaction();
    }

    pub(super) fn hover_select_target(&mut self, point: Point) {
        self.drag = None;
        if self.selection_drag.is_some() {
            return;
        }
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.preview = None;
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
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
        }
        if let Some(center) =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            self.state.overlay.hover_bond_center = Some(center);
            return;
        }
        if let Some(arrow) =
            hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            if !self
                .state
                .selection
                .arrow_objects
                .contains(&arrow.object_id)
            {
                self.state.overlay.hover_arrow = Some(arrow);
            }
        }
    }

    fn select_hit_at_point(&self, point: Point) -> Option<SelectHit> {
        if let Some((node_id, _)) = self.hit_test_endpoint_label_box(point) {
            return Some(SelectHit::Label { node_id });
        }
        if let Some((object_id, _)) = self.hit_test_text_object(point) {
            return Some(SelectHit::TextObject { object_id });
        }
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            return Some(SelectHit::Node {
                node_id: endpoint.node_id,
            });
        }
        if let Some(center) =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            return Some(SelectHit::Bond {
                bond_id: center.bond_id,
            });
        }
        for object in self.state.document.objects.iter().rev() {
            if !matches!(object.object_type.as_str(), "bracket" | "symbol") || !object.visible {
                continue;
            }
            let Some(bounds) = scene_object_selection_bounds(object) else {
                continue;
            };
            if point_in_bounds(point, bounds.expanded(crate::px_to_cm(3.0))) {
                return Some(SelectHit::ArrowObject {
                    object_id: object.id.clone(),
                });
            }
        }
        hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS).map(|arrow| {
            SelectHit::ArrowObject {
                object_id: arrow.object_id,
            }
        })
    }

    fn collect_region_selection<FP, FS, FB>(
        &self,
        mut point_inside: FP,
        mut segment_selected: FS,
        mut bounds_selected: FB,
    ) -> SelectionState
    where
        FP: FnMut(Point) -> bool,
        FS: FnMut(Point, Point) -> bool,
        FB: FnMut(AxisBounds) -> bool,
    {
        let mut selection = SelectionState::default();
        selection.region = true;
        for object in &self.state.document.objects {
            if object.object_type != "text" || !object.visible {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            if bounds_selected(AxisBounds::from_array(bounds)) {
                selection.text_objects.push(object.id.clone());
            }
        }
        for object in &self.state.document.objects {
            if object.object_type != "line" || !object.visible {
                continue;
            }
            let points = line_object_points(object);
            if points.len() < 2 {
                continue;
            }
            let start = points[0];
            let end = *points.last().unwrap_or(&start);
            if segment_selected(start, end) {
                selection.arrow_objects.push(object.id.clone());
            }
        }
        for object in &self.state.document.objects {
            if !matches!(object.object_type.as_str(), "bracket" | "symbol") || !object.visible {
                continue;
            }
            let Some(bounds) = scene_object_selection_bounds(object) else {
                continue;
            };
            if bounds_selected(bounds) {
                selection.arrow_objects.push(object.id.clone());
            }
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return selection;
        };

        for node in &entry.fragment.nodes {
            if let Some(bounds) =
                endpoint_label_world_bounds(node, entry.object.transform.translate)
            {
                if bounds_selected(AxisBounds::from_array(bounds)) {
                    selection.label_nodes.push(node.id.clone());
                }
            }
            let node_point = entry.world_point_for_node(node);
            if point_inside(node_point) {
                selection.nodes.push(node.id.clone());
            }
        }

        for bond in &entry.fragment.bonds {
            let Some(begin) = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
            else {
                continue;
            };
            let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
                continue;
            };
            let begin_point = entry.world_point_for_node(begin);
            let end_point = entry.world_point_for_node(end);
            if segment_selected(begin_point, end_point) {
                selection.bonds.push(bond.id.clone());
            }
        }
        selection
    }

    fn selection_hit_bounds(&self) -> Vec<AxisBounds> {
        let mut bounds = Vec::new();
        for object in &self.state.document.objects {
            if !self
                .state
                .selection
                .text_objects
                .iter()
                .any(|object_id| object_id == &object.id)
            {
                continue;
            }
            if let Some(text_bounds) = text_object_world_bounds(object) {
                bounds.push(AxisBounds::from_array(text_bounds));
            }
        }
        for object in &self.state.document.objects {
            if !self.state.selection.arrow_objects.contains(&object.id) {
                continue;
            }
            if let Some(arrow_bounds) = scene_object_selection_bounds(object) {
                bounds.push(arrow_bounds);
            }
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return bounds;
        };
        for component in selected_component_summaries(self) {
            let items = component_selection_items(&self.state.document, &entry, &component);
            if items.is_empty() {
                continue;
            }
            if items.len() == 1 {
                bounds.push(items[0].bounds);
            } else {
                bounds.push(items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
                    acc.include_bounds(item.bounds);
                    acc
                }));
            }
        }
        bounds
    }

    fn selection_arrange_items(&self) -> Vec<SelectionArrangeItem> {
        let mut items = Vec::new();
        for object in &self.state.document.objects {
            if !self
                .state
                .selection
                .text_objects
                .iter()
                .any(|object_id| object_id == &object.id)
            {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            items.push(SelectionArrangeItem {
                original_bounds: AxisBounds::from_array(bounds),
                bounds: AxisBounds::from_array(bounds),
                node_ids: Vec::new(),
                text_object_ids: vec![object.id.clone()],
                mirror_x: None,
                mirror_y: None,
            });
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return items;
        };
        for component in selected_component_summaries(self) {
            let fragment_items =
                component_selection_items(&self.state.document, &entry, &component);
            if fragment_items.is_empty() {
                continue;
            }
            let bounds =
                fragment_items
                    .iter()
                    .skip(1)
                    .fold(fragment_items[0].bounds, |mut acc, item| {
                        acc.include_bounds(item.bounds);
                        acc
                    });
            let node_ids = component_movable_node_ids(entry.fragment, &component);
            if node_ids.is_empty() {
                continue;
            }
            items.push(SelectionArrangeItem {
                original_bounds: bounds,
                bounds,
                node_ids,
                text_object_ids: Vec::new(),
                mirror_x: None,
                mirror_y: None,
            });
        }
        items
    }

    fn build_selection_move_drag(
        &self,
        start: Point,
        preserve_selection_after_drag: bool,
    ) -> Option<SelectionMoveDrag> {
        if self.state.selection.is_empty() {
            return None;
        }
        let mut node_ids = selected_movable_node_ids(self);
        let text_ids = selected_text_object_ids(self);
        if node_ids.is_empty() && text_ids.is_empty() {
            return None;
        }

        let mut node_originals = Vec::new();
        let mut mode = SelectionMoveMode::Translate;
        if let Some(entry) = self.state.document.editable_fragment() {
            node_ids.sort();
            for node_id in &node_ids {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == node_id) {
                    node_originals.push(NodeMoveOriginal {
                        node_id: node.id.clone(),
                        position: node.position,
                    });
                }
            }
            if node_ids.len() == 1 && text_ids.is_empty() && self.state.selection.bonds.is_empty() {
                if let Some((pivot, length)) = terminal_node_drag_axis(entry, &node_ids[0]) {
                    mode = SelectionMoveMode::TerminalNode {
                        node_id: node_ids[0].clone(),
                        pivot,
                        length,
                    };
                }
            }
        }

        let mut text_originals = Vec::new();
        for object in &self.state.document.objects {
            if text_ids.contains(&object.id) {
                text_originals.push(TextMoveOriginal {
                    object_id: object.id.clone(),
                    translate: object.transform.translate,
                });
            }
        }

        if node_originals.is_empty() && text_originals.is_empty() {
            return None;
        }

        Some(SelectionMoveDrag {
            start,
            node_originals,
            text_originals,
            mode,
            preserve_selection_after_drag,
            undo_pushed: false,
            changed: false,
        })
    }

    fn apply_selection_move_drag(&mut self, point: Point, alt_key: bool) {
        let Some(mut drag) = self.selection_drag.take() else {
            return;
        };
        let changed = selection_drag_changes_document(&drag, point, alt_key);
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        if changed {
            apply_selection_drag_to_document(self, &drag, point, alt_key);
            drag.changed = true;
        }
        self.selection_drag = Some(drag);
    }

    fn build_selection_rotate_drag(&self, start: Point) -> Option<SelectionRotateDrag> {
        if self.state.selection.is_empty() {
            return None;
        }
        let mut node_ids = selected_movable_node_ids(self);
        let text_ids = selected_text_object_ids(self);
        if node_ids.is_empty() && text_ids.is_empty() {
            return None;
        }

        let center = self.selection_rotation_bounds().map(|bounds| {
            Point::new(
                (bounds.min_x + bounds.max_x) * 0.5,
                (bounds.min_y + bounds.max_y) * 0.5,
            )
        })?;
        let mut node_originals = Vec::new();
        if let Some(entry) = self.state.document.editable_fragment() {
            node_ids.sort();
            for node_id in &node_ids {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == node_id) {
                    node_originals.push(NodeMoveOriginal {
                        node_id: node.id.clone(),
                        position: node.position,
                    });
                }
            }
        }

        let mut text_originals = Vec::new();
        for object in &self.state.document.objects {
            if text_ids.contains(&object.id) {
                text_originals.push(TextMoveOriginal {
                    object_id: object.id.clone(),
                    translate: object.transform.translate,
                });
            }
        }

        if node_originals.is_empty() && text_originals.is_empty() {
            return None;
        }

        Some(SelectionRotateDrag {
            center,
            start_angle: angle_between(center, start),
            node_originals,
            text_originals,
            undo_pushed: false,
            changed: false,
        })
    }

    fn apply_selection_rotate_drag(&mut self, point: Point, alt_key: bool) {
        let Some(mut drag) = self.selection_rotate_drag.take() else {
            return;
        };
        let angle = selection_rotate_delta_degrees(&drag, point, alt_key);
        let changed = angle.abs() > crate::EPSILON;
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        if changed {
            apply_selection_rotation_to_document(self, &drag, angle);
            drag.changed = true;
        }
        self.selection_rotate_drag = Some(drag);
    }

    pub(super) fn selection_render_list(&self) -> Vec<RenderPrimitive> {
        if self.selection_rotate_drag.is_some() {
            return Vec::new();
        }
        let mut out = Vec::new();
        render_selected_text_boxes(self, &mut out);
        render_selected_arrow_handles(self, &mut out);
        render_selected_fragment_content(self, &mut out);
        out
    }

    fn selection_rotation_bounds(&self) -> Option<AxisBounds> {
        let mut out = None;
        for object in &self.state.document.objects {
            if !self.state.selection.text_objects.contains(&object.id) {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            include_optional_bounds(&mut out, AxisBounds::from_array(bounds));
        }
        for object in &self.state.document.objects {
            if !self.state.selection.arrow_objects.contains(&object.id) {
                continue;
            }
            if let Some(bounds) = scene_object_selection_bounds(object) {
                include_optional_bounds(&mut out, bounds);
            }
        }
        let Some(entry) = self.state.document.editable_fragment() else {
            return out;
        };
        for component in selected_component_summaries(self) {
            for item in component_selection_items(&self.state.document, &entry, &component) {
                include_optional_bounds(&mut out, item.bounds);
            }
        }
        out
    }
}

fn selected_text_object_ids(engine: &Engine) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = engine
        .state
        .selection
        .text_objects
        .iter()
        .cloned()
        .collect();
    for object in &engine.state.document.objects {
        if matches!(object.object_type.as_str(), "bracket" | "symbol")
            && engine.state.selection.arrow_objects.contains(&object.id)
        {
            ids.insert(object.id.clone());
        }
    }
    ids
}

fn selected_movable_node_ids(engine: &Engine) -> Vec<String> {
    let mut node_ids: BTreeSet<String> = engine.state.selection.nodes.iter().cloned().collect();
    node_ids.extend(engine.state.selection.label_nodes.iter().cloned());
    let Some(entry) = engine.state.document.editable_fragment() else {
        return node_ids.into_iter().collect();
    };
    for bond_id in &engine.state.selection.bonds {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| &bond.id == bond_id) else {
            continue;
        };
        node_ids.insert(bond.begin.clone());
        node_ids.insert(bond.end.clone());
    }
    node_ids.into_iter().collect()
}

fn object_arrow_curve(object: &crate::SceneObject) -> f64 {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.0)
}

fn snapped_arrow_endpoint(pivot: Point, point: Point, alt_key: bool) -> Point {
    let length = pivot.distance(point);
    if length <= crate::EPSILON {
        return pivot;
    }
    let angle = if alt_key {
        angle_between(pivot, point)
    } else {
        nearest_angle(angle_between(pivot, point), GLOBAL_SNAP_ANGLES)
    };
    pivot.translated(direction_from_angle(angle).scaled(length))
}

fn snapped_arrow_curve_from_point(start: Point, end: Point, point: Point, alt_key: bool) -> f64 {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let chord_length = start.distance(end);
    if chord_length <= crate::EPSILON {
        return 0.0;
    }
    let mid = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
    let ux = chord.x / chord_length;
    let uy = chord.y / chord_length;
    let normal_x = -uy;
    let normal_y = ux;
    let sagitta = (point.x - mid.x) * normal_x + (point.y - mid.y) * normal_y;
    let mut degrees = (4.0 * (2.0 * sagitta / chord_length).atan()).to_degrees();
    degrees = degrees.clamp(-270.0, 270.0);
    if !alt_key {
        degrees = (degrees / 15.0).round() * 15.0;
    }
    if degrees.abs() < 0.5 {
        0.0
    } else {
        degrees
    }
}

fn update_arrow_object_points(
    engine: &mut Engine,
    object_id: &str,
    start: Point,
    end: Point,
) -> bool {
    let Some(object) = engine
        .state
        .document
        .objects
        .iter_mut()
        .find(|object| object.id == object_id && object.object_type == "line")
    else {
        return false;
    };
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let next_points = json!([[start.x - tx, start.y - ty], [end.x - tx, end.y - ty]]);
    if object.payload.extra.get("points") == Some(&next_points) {
        return false;
    }
    object
        .payload
        .extra
        .insert("points".to_string(), next_points);
    true
}

fn update_arrow_object_curve(engine: &mut Engine, object_id: &str, curve: f64) -> bool {
    let Some(object) = engine
        .state
        .document
        .objects
        .iter_mut()
        .find(|object| object.id == object_id && object.object_type == "line")
    else {
        return false;
    };
    let mut arrow_head = object
        .payload
        .extra
        .get("arrowHead")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let Some(arrow_head_object) = arrow_head.as_object_mut() else {
        return false;
    };
    let rounded_curve = (curve * 1000.0).round() / 1000.0;
    arrow_head_object.insert("curve".to_string(), json!(rounded_curve));
    let kind = arrow_head_object
        .get("kind")
        .and_then(JsonValue::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase();
    if kind != "hollow" && kind != "open" {
        let next_kind = if rounded_curve < -crate::EPSILON {
            "curved"
        } else if rounded_curve > crate::EPSILON {
            "curved-mirror"
        } else {
            "solid"
        };
        arrow_head_object.insert("kind".to_string(), json!(next_kind));
    }
    if object.payload.extra.get("arrowHead") == Some(&arrow_head) {
        return false;
    }
    object
        .payload
        .extra
        .insert("arrowHead".to_string(), arrow_head);
    true
}

fn component_movable_node_ids(
    fragment: &crate::MoleculeFragment,
    component: &ComponentSelection,
) -> Vec<String> {
    let mut node_ids: BTreeSet<String> = component.node_ids.iter().cloned().collect();
    node_ids.extend(component.label_node_ids.iter().cloned());
    for bond_id in &component.bond_ids {
        let Some(bond) = fragment.bonds.iter().find(|bond| &bond.id == bond_id) else {
            continue;
        };
        node_ids.insert(bond.begin.clone());
        node_ids.insert(bond.end.clone());
    }
    node_ids.into_iter().collect()
}

#[derive(Clone, Copy)]
enum AlignAxis {
    XMin,
    XMax,
    XCenter,
    YMin,
    YMax,
    YCenter,
}

#[derive(Clone, Copy)]
enum DistributeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
enum FlipAxis {
    Horizontal,
    Vertical,
}

fn align_items(items: &mut [SelectionArrangeItem], axis: AlignAxis) -> bool {
    if items.len() < 2 {
        return false;
    }
    let target = match axis {
        AlignAxis::XMin => items
            .iter()
            .map(|item| item.bounds.min_x)
            .fold(f64::INFINITY, f64::min),
        AlignAxis::XMax => items
            .iter()
            .map(|item| item.bounds.max_x)
            .fold(f64::NEG_INFINITY, f64::max),
        AlignAxis::XCenter => {
            let min_x = items
                .iter()
                .map(|item| item.bounds.min_x)
                .fold(f64::INFINITY, f64::min);
            let max_x = items
                .iter()
                .map(|item| item.bounds.max_x)
                .fold(f64::NEG_INFINITY, f64::max);
            (min_x + max_x) * 0.5
        }
        AlignAxis::YMin => items
            .iter()
            .map(|item| item.bounds.min_y)
            .fold(f64::INFINITY, f64::min),
        AlignAxis::YMax => items
            .iter()
            .map(|item| item.bounds.max_y)
            .fold(f64::NEG_INFINITY, f64::max),
        AlignAxis::YCenter => {
            let min_y = items
                .iter()
                .map(|item| item.bounds.min_y)
                .fold(f64::INFINITY, f64::min);
            let max_y = items
                .iter()
                .map(|item| item.bounds.max_y)
                .fold(f64::NEG_INFINITY, f64::max);
            (min_y + max_y) * 0.5
        }
    };

    let mut changed = false;
    for item in items {
        let (dx, dy) = match axis {
            AlignAxis::XMin => (target - item.bounds.min_x, 0.0),
            AlignAxis::XMax => (target - item.bounds.max_x, 0.0),
            AlignAxis::XCenter => (target - item.bounds.center_x(), 0.0),
            AlignAxis::YMin => (0.0, target - item.bounds.min_y),
            AlignAxis::YMax => (0.0, target - item.bounds.max_y),
            AlignAxis::YCenter => (0.0, target - item.bounds.center_y()),
        };
        changed |= item.translate(dx, dy);
    }
    changed
}

fn distribute_items(items: &mut [SelectionArrangeItem], axis: DistributeAxis) -> bool {
    if items.len() < 3 {
        return false;
    }
    match axis {
        DistributeAxis::Horizontal => items.sort_by(|a, b| {
            a.bounds
                .min_x
                .total_cmp(&b.bounds.min_x)
                .then(a.bounds.max_x.total_cmp(&b.bounds.max_x))
        }),
        DistributeAxis::Vertical => items.sort_by(|a, b| {
            a.bounds
                .min_y
                .total_cmp(&b.bounds.min_y)
                .then(a.bounds.max_y.total_cmp(&b.bounds.max_y))
        }),
    }
    let span_min = match axis {
        DistributeAxis::Horizontal => items.first().unwrap().bounds.min_x,
        DistributeAxis::Vertical => items.first().unwrap().bounds.min_y,
    };
    let span_max = match axis {
        DistributeAxis::Horizontal => items.last().unwrap().bounds.max_x,
        DistributeAxis::Vertical => items.last().unwrap().bounds.max_y,
    };
    let occupied: f64 = items
        .iter()
        .map(|item| match axis {
            DistributeAxis::Horizontal => item.bounds.width(),
            DistributeAxis::Vertical => item.bounds.height(),
        })
        .sum();
    let gap = (span_max - span_min - occupied) / (items.len() - 1) as f64;
    let mut cursor = span_min;
    let mut changed = false;
    for item in items {
        let (dx, dy) = match axis {
            DistributeAxis::Horizontal => {
                let dx = cursor - item.bounds.min_x;
                cursor += item.bounds.width() + gap;
                (dx, 0.0)
            }
            DistributeAxis::Vertical => {
                let dy = cursor - item.bounds.min_y;
                cursor += item.bounds.height() + gap;
                (0.0, dy)
            }
        };
        changed |= item.translate(dx, dy);
    }
    changed
}

fn flip_items(items: &mut [SelectionArrangeItem], axis: FlipAxis) -> bool {
    if items.is_empty() {
        return false;
    }
    let mut bounds = items[0].bounds;
    for item in items.iter().skip(1) {
        bounds.include_bounds(item.bounds);
    }
    let pivot = match axis {
        FlipAxis::Horizontal => bounds.center_x(),
        FlipAxis::Vertical => bounds.center_y(),
    };
    let mut changed = false;
    for item in items {
        match axis {
            FlipAxis::Horizontal => changed |= item.flip_horizontal(pivot),
            FlipAxis::Vertical => changed |= item.flip_vertical(pivot),
        }
    }
    changed
}

impl AxisBounds {
    fn width(self) -> f64 {
        (self.max_x - self.min_x).max(0.0)
    }

    fn height(self) -> f64 {
        (self.max_y - self.min_y).max(0.0)
    }

    fn center_x(self) -> f64 {
        (self.min_x + self.max_x) * 0.5
    }

    fn center_y(self) -> f64 {
        (self.min_y + self.max_y) * 0.5
    }
}

impl SelectionArrangeItem {
    fn translate(&mut self, dx: f64, dy: f64) -> bool {
        if dx.abs() <= crate::EPSILON && dy.abs() <= crate::EPSILON {
            return false;
        }
        self.bounds.min_x += dx;
        self.bounds.max_x += dx;
        self.bounds.min_y += dy;
        self.bounds.max_y += dy;
        true
    }

    fn flip_horizontal(&mut self, pivot_x: f64) -> bool {
        let min_x = 2.0 * pivot_x - self.bounds.max_x;
        let max_x = 2.0 * pivot_x - self.bounds.min_x;
        let changed = (self.bounds.min_x - min_x).abs() > crate::EPSILON
            || (self.bounds.max_x - max_x).abs() > crate::EPSILON
            || self.node_ids.len() > 1;
        self.bounds.min_x = min_x;
        self.bounds.max_x = max_x;
        self.mirror_x = Some(pivot_x);
        changed
    }

    fn flip_vertical(&mut self, pivot_y: f64) -> bool {
        let min_y = 2.0 * pivot_y - self.bounds.max_y;
        let max_y = 2.0 * pivot_y - self.bounds.min_y;
        let changed = (self.bounds.min_y - min_y).abs() > crate::EPSILON
            || (self.bounds.max_y - max_y).abs() > crate::EPSILON
            || self.node_ids.len() > 1;
        self.bounds.min_y = min_y;
        self.bounds.max_y = max_y;
        self.mirror_y = Some(pivot_y);
        changed
    }
}

fn apply_arrange_items_to_document(engine: &mut Engine, items: &[SelectionArrangeItem]) {
    for item in items {
        for object_id in &item.text_object_ids {
            let Some(object) = engine
                .state
                .document
                .objects
                .iter_mut()
                .find(|object| object.id == *object_id)
            else {
                continue;
            };
            let Some(current_bounds) = text_object_world_bounds(object) else {
                continue;
            };
            object.transform.translate = [
                round2(object.transform.translate[0] + item.bounds.min_x - current_bounds[0]),
                round2(object.transform.translate[1] + item.bounds.min_y - current_bounds[1]),
            ];
        }
    }

    let stroke_width = engine.options.bond_stroke_world_cm().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for item in items {
        for node_id in &item.node_ids {
            let Some(node) = entry
                .fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
            else {
                continue;
            };
            let original_world = Point::new(
                object_translate[0] + node.position[0],
                object_translate[1] + node.position[1],
            );
            let scale_x = if item.original_bounds.width() <= crate::EPSILON {
                0.0
            } else {
                (original_world.x - item.original_bounds.min_x) / item.original_bounds.width()
            };
            let scale_y = if item.original_bounds.height() <= crate::EPSILON {
                0.0
            } else {
                (original_world.y - item.original_bounds.min_y) / item.original_bounds.height()
            };
            let mut next_world = Point::new(
                item.bounds.min_x + item.bounds.width() * scale_x,
                item.bounds.min_y + item.bounds.height() * scale_y,
            );
            if let Some(pivot_x) = item.mirror_x {
                next_world.x = 2.0 * pivot_x - original_world.x;
            }
            if let Some(pivot_y) = item.mirror_y {
                next_world.y = 2.0 * pivot_y - original_world.y;
            }
            node.position = [
                round2(next_world.x - object_translate[0]),
                round2(next_world.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
    drop(entry);
    engine.refresh_symbol_chemistry();
}

fn terminal_node_drag_axis(
    entry: crate::EditableFragment<'_>,
    node_id: &str,
) -> Option<(Point, f64)> {
    let incident: Vec<_> = entry
        .fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if incident.len() != 1 {
        return None;
    }
    let bond = incident[0];
    let other_id = if bond.begin == node_id {
        &bond.end
    } else {
        &bond.begin
    };
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)?;
    let other = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == *other_id)?;
    let node_point = entry.world_point_for_node(node);
    let pivot = entry.world_point_for_node(other);
    let length = if pivot.distance(node_point) <= crate::EPSILON {
        DEFAULT_BOND_LENGTH
    } else {
        pivot.distance(node_point)
    };
    Some((pivot, length))
}

fn selection_drag_changes_document(drag: &SelectionMoveDrag, point: Point, alt_key: bool) -> bool {
    match &drag.mode {
        SelectionMoveMode::TerminalNode { pivot, length, .. } => {
            if point.distance(drag.start) <= crate::EPSILON {
                return false;
            }
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            drag.node_originals.iter().any(|node| {
                (node.position[0] - target.x).abs() > 1.0e-9
                    || (node.position[1] - target.y).abs() > 1.0e-9
            })
        }
        SelectionMoveMode::Translate => point.distance(drag.start) > crate::EPSILON,
    }
}

fn apply_selection_drag_to_document(
    engine: &mut Engine,
    drag: &SelectionMoveDrag,
    point: Point,
    alt_key: bool,
) {
    let delta_x = point.x - drag.start.x;
    let delta_y = point.y - drag.start.y;

    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .objects
            .iter_mut()
            .find(|object| object.id == original.object_id)
        else {
            continue;
        };
        if matches!(drag.mode, SelectionMoveMode::Translate) {
            object.transform.translate = [
                round2(original.translate[0] + delta_x),
                round2(original.translate[1] + delta_y),
            ];
        }
    }

    let stroke_width = engine.options.bond_stroke_world_cm().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    match &drag.mode {
        SelectionMoveMode::Translate => {
            for original in &drag.node_originals {
                if let Some(node) = entry
                    .fragment
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == original.node_id)
                {
                    node.position = [
                        round2(original.position[0] + delta_x),
                        round2(original.position[1] + delta_y),
                    ];
                }
            }
        }
        SelectionMoveMode::TerminalNode {
            node_id,
            pivot,
            length,
        } => {
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            if let Some(node) = entry
                .fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
            {
                node.position = [
                    round2(target.x - object_translate[0]),
                    round2(target.y - object_translate[1]),
                ];
            }
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
}

fn selection_rotate_delta_degrees(drag: &SelectionRotateDrag, point: Point, alt_key: bool) -> f64 {
    let raw = signed_angle_delta(drag.start_angle, angle_between(drag.center, point));
    if alt_key {
        return raw;
    }
    (raw / 15.0).round() * 15.0
}

fn signed_angle_delta(start: f64, end: f64) -> f64 {
    let mut delta = (end - start) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta <= -180.0 {
        delta += 360.0;
    }
    delta
}

fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

fn apply_selection_rotation_to_document(
    engine: &mut Engine,
    drag: &SelectionRotateDrag,
    angle: f64,
) {
    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .objects
            .iter_mut()
            .find(|object| object.id == original.object_id)
        else {
            continue;
        };
        let next = rotate_point_around(
            Point::new(original.translate[0], original.translate[1]),
            drag.center,
            angle,
        );
        object.transform.translate = [round2(next.x), round2(next.y)];
    }

    let stroke_width = engine.options.bond_stroke_world_cm().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for original in &drag.node_originals {
        let original_world = Point::new(
            object_translate[0] + original.position[0],
            object_translate[1] + original.position[1],
        );
        let next = rotate_point_around(original_world, drag.center, angle);
        if let Some(node) = entry
            .fragment
            .nodes
            .iter_mut()
            .find(|node| node.id == original.node_id)
        {
            node.position = [
                round2(next.x - object_translate[0]),
                round2(next.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
}

fn terminal_drag_target(pivot: Point, length: f64, point: Point, alt_key: bool) -> Point {
    if alt_key {
        return point;
    }
    let angle = nearest_angle(angle_between(pivot, point), GLOBAL_SNAP_ANGLES);
    pivot.translated(direction_from_angle(angle).scaled(length))
}

fn clear_select_hover_overlay(engine: &mut Engine) {
    engine.state.overlay.hover_bond_center = None;
    engine.state.overlay.hover_arrow = None;
    engine.state.overlay.hover_text_box = None;
    engine.state.overlay.hover_endpoint = None;
    engine.state.overlay.preview = None;
}

fn render_selected_text_boxes(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let selected_text_objects: BTreeSet<&str> = engine
        .state
        .selection
        .text_objects
        .iter()
        .map(String::as_str)
        .collect();
    for object in &engine.state.document.objects {
        if !selected_text_objects.contains(object.id.as_str()) {
            continue;
        }
        let Some(bounds) = text_object_world_bounds(object) else {
            continue;
        };
        push_selection_box(
            out,
            AxisBounds::from_array(bounds),
            RenderRole::SelectionTextBox,
        );
    }
}

fn render_selected_arrow_handles(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    for object in &engine.state.document.objects {
        if !engine.state.selection.arrow_objects.contains(&object.id) {
            continue;
        }
        if let Some(bounds) = scene_object_selection_bounds(object) {
            push_selection_box(out, bounds, RenderRole::SelectionBox);
        }
    }
}

fn scene_object_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    if matches!(object.object_type.as_str(), "bracket" | "symbol" | "shape") {
        return object_bbox_selection_bounds(object)
            .map(|bounds| bounds.expanded(crate::px_to_cm(3.0)));
    }
    arrow_object_selection_bounds(object)
}

fn arrow_object_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let points = line_object_points(object);
    if points.len() < 2 {
        return None;
    }
    let mut handles = arrow_object_handle_points(object, &points).into_iter();
    let first = handles.next()?;
    let mut bounds = AxisBounds::around_point(first, 0.0);
    for handle in handles {
        bounds.include_point(handle);
    }
    Some(bounds.expanded(crate::px_to_cm(4.0)))
}

fn object_bbox_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    Some(AxisBounds::new(
        tx + x,
        ty + y,
        tx + x + width,
        ty + y + height,
    ))
}

fn render_selected_fragment_content(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return;
    };

    for component in selected_component_summaries(engine) {
        let items = component_selection_items(&engine.state.document, &entry, &component);
        if items.is_empty() {
            continue;
        }
        if items.len() == 1 {
            let item = items[0];
            push_selection_item_box(out, item);
            push_selection_bond_dot(out, item.center);
            continue;
        }
        let group_bounds = items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
            acc.include_bounds(item.bounds);
            acc
        });
        push_selection_box(out, group_bounds, RenderRole::SelectionBox);
        for item in items {
            push_selection_bond_dot(out, item.center);
        }
    }
}

fn selected_component_summaries(engine: &Engine) -> Vec<ComponentSelection> {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return Vec::new();
    };
    let selected_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .nodes
        .iter()
        .map(String::as_str)
        .collect();
    let selected_bonds: BTreeSet<&str> = engine
        .state
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect();
    let selected_label_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .label_nodes
        .iter()
        .map(String::as_str)
        .collect();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut components = Vec::new();

    for node in &entry.fragment.nodes {
        if visited.contains(&node.id) {
            continue;
        }
        let component_node_ids = connected_component_node_ids(entry.fragment, &node.id);
        for node_id in &component_node_ids {
            visited.insert(node_id.clone());
        }
        let component_bond_ids: Vec<String> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                component_node_ids.contains(&bond.begin) && component_node_ids.contains(&bond.end)
            })
            .map(|bond| bond.id.clone())
            .collect();

        let component_selected_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_label_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_label_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_bonds: Vec<String> = component_bond_ids
            .iter()
            .filter(|bond_id| selected_bonds.contains(bond_id.as_str()))
            .cloned()
            .collect();
        if component_selected_nodes.is_empty()
            && component_selected_label_nodes.is_empty()
            && component_selected_bonds.is_empty()
        {
            continue;
        }
        components.push(ComponentSelection {
            node_ids: component_selected_nodes,
            label_node_ids: component_selected_label_nodes,
            bond_ids: component_selected_bonds,
        });
    }
    components
}

fn bracket_object_ids_containing_component(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component_node_ids: &[String],
) -> Vec<String> {
    let mut sample_points = Vec::new();
    for node_id in component_node_ids {
        if let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) {
            sample_points.push(entry.world_point_for_node(node));
        }
    }
    for bond in &entry.fragment.bonds {
        if !component_node_ids.contains(&bond.begin) || !component_node_ids.contains(&bond.end) {
            continue;
        }
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        sample_points.push(midpoint(
            entry.world_point_for_node(begin),
            entry.world_point_for_node(end),
        ));
    }

    document
        .objects
        .iter()
        .filter(|object| object.object_type == "bracket" && object.visible)
        .filter_map(|object| {
            let bounds = object_bbox_selection_bounds(object)?;
            if sample_points
                .iter()
                .any(|point| point_in_bounds(*point, bounds))
            {
                Some(object.id.clone())
            } else {
                None
            }
        })
        .collect()
}

fn component_selection_items(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component: &ComponentSelection,
) -> Vec<FragmentSelectionItem> {
    let mut items = Vec::new();
    for node_id in &component.label_node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
        else {
            continue;
        };
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Label,
            bounds: AxisBounds::from_array(bounds),
            center: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
        });
    }
    for node_id in &component.node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let center = entry.world_point_for_node(node);
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Node,
            bounds: AxisBounds::around_point(center, SELECTION_NODE_BOX_SIZE / 2.0),
            center,
        });
    }
    for bond_id in &component.bond_ids {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == *bond_id) else {
            continue;
        };
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let bounds = fragment_bond_visual_bounds(document, entry.object, entry.fragment, bond)
            .map(AxisBounds::from_array)
            .unwrap_or_else(|| {
                AxisBounds::new(begin_point.x, begin_point.y, end_point.x, end_point.y)
            });
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Bond,
            bounds,
            center: midpoint(begin_point, end_point),
        });
    }
    items
}

fn add_hit_to_selection(selection: &mut SelectionState, hit: SelectHit) {
    match hit {
        SelectHit::TextObject { object_id } => push_unique(&mut selection.text_objects, object_id),
        SelectHit::ArrowObject { object_id } => {
            push_unique(&mut selection.arrow_objects, object_id)
        }
        SelectHit::Label { node_id } => push_unique(&mut selection.label_nodes, node_id),
        SelectHit::Node { node_id } => push_unique(&mut selection.nodes, node_id),
        SelectHit::Bond { bond_id } => push_unique(&mut selection.bonds, bond_id),
    }
}

fn selection_contains_hit(selection: &SelectionState, hit: &SelectHit) -> bool {
    match hit {
        SelectHit::TextObject { object_id } => selection.text_objects.contains(object_id),
        SelectHit::ArrowObject { object_id } => selection.arrow_objects.contains(object_id),
        SelectHit::Label { node_id } => selection.label_nodes.contains(node_id),
        SelectHit::Node { node_id } => selection.nodes.contains(node_id),
        SelectHit::Bond { bond_id } => selection.bonds.contains(bond_id),
    }
}

fn merge_selection(
    current: SelectionState,
    next: SelectionState,
    additive: bool,
) -> SelectionState {
    if !additive {
        return next;
    }
    let mut merged = current;
    merged.region = merged.region || next.region;
    for object_id in next.text_objects {
        push_unique(&mut merged.text_objects, object_id);
    }
    for object_id in next.arrow_objects {
        push_unique(&mut merged.arrow_objects, object_id);
    }
    for node_id in next.label_nodes {
        push_unique(&mut merged.label_nodes, node_id);
    }
    for node_id in next.nodes {
        push_unique(&mut merged.nodes, node_id);
    }
    for bond_id in next.bonds {
        push_unique(&mut merged.bonds, bond_id);
    }
    merged
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn push_selection_box(out: &mut Vec<RenderPrimitive>, bounds: AxisBounds, role: RenderRole) {
    out.push(RenderPrimitive::Rect {
        role,
        object_id: None,
        node_id: None,
        x: bounds.min_x,
        y: bounds.min_y,
        width: (bounds.max_x - bounds.min_x).max(0.0),
        height: (bounds.max_y - bounds.min_y).max(0.0),
        fill: Some("rgba(47,111,237,0.08)".to_string()),
        stroke: Some("rgba(47,111,237,0.86)".to_string()),
        stroke_width: SELECTION_BOX_STROKE_WIDTH,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
}

fn push_selection_item_box(out: &mut Vec<RenderPrimitive>, item: FragmentSelectionItem) {
    let role = match item.kind {
        FragmentItemKind::Node => RenderRole::SelectionNode,
        FragmentItemKind::Label => RenderRole::SelectionTextBox,
        FragmentItemKind::Bond => RenderRole::SelectionBond,
    };
    push_selection_box(out, item.bounds, role);
}

fn push_selection_bond_dot(out: &mut Vec<RenderPrimitive>, center: Point) {
    out.push(RenderPrimitive::Circle {
        role: RenderRole::SelectionBondDot,
        object_id: None,
        node_id: None,
        center,
        radius: SELECTION_BOND_DOT_RADIUS,
        fill: "rgba(47,111,237,0.9)".to_string(),
        stroke: "#ffffff".to_string(),
        stroke_width: crate::px_to_cm(1.0),
    });
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn include_optional_bounds(target: &mut Option<AxisBounds>, bounds: AxisBounds) {
    if let Some(existing) = target {
        existing.include_bounds(bounds);
    } else {
        *target = Some(bounds);
    }
}

fn point_in_bounds(point: Point, bounds: AxisBounds) -> bool {
    point.x >= bounds.min_x
        && point.x <= bounds.max_x
        && point.y >= bounds.min_y
        && point.y <= bounds.max_y
}

fn bounds_intersect(a: AxisBounds, b: AxisBounds) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

fn polygon_bounds(points: &[Point]) -> AxisBounds {
    let mut bounds = AxisBounds::around_point(points[0], 0.0);
    for point in &points[1..] {
        bounds.include_point(*point);
    }
    bounds
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = *polygon.last().unwrap_or(&point);
    for current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y + 1.0e-12)
                    + current.x);
        if intersects {
            inside = !inside;
        }
        previous = *current;
    }
    inside
}

fn segment_intersects_bounds(start: Point, end: Point, bounds: AxisBounds) -> bool {
    if point_in_bounds(start, bounds) || point_in_bounds(end, bounds) {
        return true;
    }
    let corners = [
        Point::new(bounds.min_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.max_y),
        Point::new(bounds.min_x, bounds.max_y),
    ];
    (0..4).any(|index| segments_intersect(start, end, corners[index], corners[(index + 1) % 4]))
}

fn rect_intersects_polygon(
    bounds: AxisBounds,
    polygon: &[Point],
    polygon_bounds: AxisBounds,
) -> bool {
    if !bounds_intersect(bounds, polygon_bounds) {
        return false;
    }
    let rect_points = [
        Point::new(bounds.min_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.max_y),
        Point::new(bounds.min_x, bounds.max_y),
    ];
    if rect_points
        .iter()
        .any(|point| point_in_polygon(*point, polygon))
    {
        return true;
    }
    if polygon.iter().any(|point| point_in_bounds(*point, bounds)) {
        return true;
    }
    (0..4).any(|edge_index| {
        let rect_start = rect_points[edge_index];
        let rect_end = rect_points[(edge_index + 1) % 4];
        polygon.iter().enumerate().any(|(index, start)| {
            let end = polygon[(index + 1) % polygon.len()];
            segments_intersect(rect_start, rect_end, *start, end)
        })
    })
}

fn segment_intersects_polygon(
    start: Point,
    end: Point,
    polygon: &[Point],
    polygon_bounds: AxisBounds,
) -> bool {
    if !bounds_intersect(
        AxisBounds::new(start.x, start.y, end.x, end.y),
        polygon_bounds,
    ) {
        return false;
    }
    if point_in_polygon(start, polygon) || point_in_polygon(end, polygon) {
        return true;
    }
    polygon.iter().enumerate().any(|(index, edge_start)| {
        let edge_end = polygon[(index + 1) % polygon.len()];
        segments_intersect(start, end, *edge_start, edge_end)
    })
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y)
}

fn on_segment(a: Point, b: Point, c: Point) -> bool {
    b.x >= a.x.min(c.x) - 1.0e-9
        && b.x <= a.x.max(c.x) + 1.0e-9
        && b.y >= a.y.min(c.y) - 1.0e-9
        && b.y <= a.y.max(c.y) + 1.0e-9
}

fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let o1 = orientation(a1, a2, b1);
    let o2 = orientation(a1, a2, b2);
    let o3 = orientation(b1, b2, a1);
    let o4 = orientation(b1, b2, a2);
    if (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0) {
        return true;
    }
    (o1.abs() <= 1.0e-9 && on_segment(a1, b1, a2))
        || (o2.abs() <= 1.0e-9 && on_segment(a1, b2, a2))
        || (o3.abs() <= 1.0e-9 && on_segment(b1, a1, b2))
        || (o4.abs() <= 1.0e-9 && on_segment(b1, a2, b2))
}

fn connected_component_node_ids(
    fragment: &crate::MoleculeFragment,
    start_node_id: &str,
) -> Vec<String> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_node_id.to_string());
    queue.push_back(start_node_id.to_string());
    while let Some(current) = queue.pop_front() {
        for bond in &fragment.bonds {
            let neighbor = if bond.begin == current {
                Some(bond.end.as_str())
            } else if bond.end == current {
                Some(bond.begin.as_str())
            } else {
                None
            };
            let Some(neighbor) = neighbor else {
                continue;
            };
            if visited.insert(neighbor.to_string()) {
                queue.push_back(neighbor.to_string());
            }
        }
    }
    visited.into_iter().collect()
}
