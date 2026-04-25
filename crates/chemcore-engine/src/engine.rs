use crate::{
    anchor_from_point, bond_center_focus_length, can_draw_bond, can_focus_bond_center,
    can_focus_endpoint, default_angle_for_anchor_for_variant, endpoint_from_angle_for_document,
    hit_test_bond_center, hit_test_endpoint, hit_test_endpoint_excluding, render_document,
    select_at, snapped_angle_for_anchor, Bond, BondAnchor, BondLinePattern, BondLineStyles,
    BondLineWeight, BondLineWeights, BondPreview, BondStereo, BondVariant, ChemcoreDocument,
    DoubleBond, DoubleBondPlacement, DragState, EditorOptions, EndpointHit, OverlayState, Point,
    PointerEvent, RenderPrimitive, RenderRole, SelectionState, Tool, ToolState,
    BOND_CENTER_FOCUS_WIDTH, BOND_CENTER_HIT_RADIUS, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD,
    ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub selection: SelectionState,
    pub overlay: OverlayState,
}

pub struct Engine {
    state: EngineState,
    drag: Option<DragState>,
    options: EditorOptions,
    next_id: u64,
    undo_stack: Vec<ChemcoreDocument>,
    redo_stack: Vec<ChemcoreDocument>,
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
            options: EditorOptions::default(),
            next_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
        let document: ChemcoreDocument =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
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
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            out.push(RenderPrimitive::Circle {
                role: RenderRole::HoverEndpoint,
                object_id: None,
                center: hover.point,
                radius: ENDPOINT_FOCUS_RADIUS,
                fill: "rgba(47,111,237,0.24)".to_string(),
                stroke: "rgba(47,111,237,0.78)".to_string(),
                stroke_width: 1.4,
            });
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
                    stroke_width: 1.2,
                });
            }
        }
        if let Some(preview) = &self.state.overlay.preview {
            out.push(RenderPrimitive::Circle {
                role: RenderRole::PreviewEnd,
                object_id: None,
                center: preview.end,
                radius: 5.0,
                fill: "#ffffff".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: 1.2,
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
            self.clear_interaction();
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
                        self.options.bond_length,
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
            self.state.selection = select_at(&self.state.document, event.point());
            self.clear_interaction();
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
                        self.options.bond_length,
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
                self.options.bond_length,
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
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
            stroke_width: self.options.bond_stroke_width,
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
        refresh_generated_node_label_geometry_for_node(
            entry.fragment,
            entry.object.transform.translate,
            &begin_id,
            self.options.bond_stroke_width,
        );
        refresh_generated_node_label_geometry_for_node(
            entry.fragment,
            entry.object.transform.translate,
            &end_id,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();

        let endpoint = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == end_id)
            .map(|node| EndpointHit {
                node_id: node.id.clone(),
                point: entry.world_point_for_node(node),
                distance: 0.0,
                label_anchor: None,
            });
        self.state.overlay.hover_endpoint = endpoint;
        true
    }

    fn preview_document(&self) -> Option<ChemcoreDocument> {
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
            stroke_width: self.options.bond_stroke_width,
            line_styles: self.pending_line_styles(),
            line_weights: self.pending_line_weights(),
            meta: serde_json::Value::Null,
        });
        entry.update_bounds();
        Some(document)
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
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
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn delete_selection(&mut self) -> bool {
        if self.state.selection.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let selection = self.state.selection.clone();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };

        let selected_nodes: BTreeSet<String> = selection.nodes.into_iter().collect();
        let selected_bonds: BTreeSet<String> = selection.bonds.into_iter().collect();
        entry.fragment.bonds.retain(|bond| {
            !selected_bonds.contains(&bond.id)
                && !selected_nodes.contains(&bond.begin)
                && !selected_nodes.contains(&bond.end)
        });

        let connected_nodes: BTreeSet<String> = entry
            .fragment
            .bonds
            .iter()
            .flat_map(|bond| [bond.begin.clone(), bond.end.clone()])
            .collect();
        entry.fragment.nodes.retain(|node| {
            !selected_nodes.contains(&node.id) && connected_nodes.contains(&node.id)
        });
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn replace_hovered_endpoint_label(&mut self, label: &str) -> bool {
        let Some(hovered_node_id) = self
            .state
            .overlay
            .hover_endpoint
            .as_ref()
            .map(|hit| hit.node_id.clone())
        else {
            return false;
        };

        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == hovered_node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let node = &mut entry.fragment.nodes[node_index];

        if !apply_node_label_replacement(node, label) {
            self.undo_stack.pop();
            return false;
        }

        let node_position = node.position;
        refresh_generated_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            &hovered_node_id,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: hovered_node_id,
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.state.document.clone());
        self.restore_document(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.state.document.clone());
        self.restore_document(next);
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

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.state.document.clone());
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
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)?;
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)?;
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let dx = end_point.x - begin_point.x;
        let dy = end_point.y - begin_point.y;
        let length = dx.hypot(dy);
        if length <= crate::EPSILON {
            return Some(DoubleBondPlacement::Left);
        }
        let normal_x = -dy / length;
        let normal_y = dx / length;
        let mut score = 0.0;
        for other in &entry.fragment.bonds {
            if other.id == bond.id {
                continue;
            }
            if other.begin == bond.begin || other.end == bond.begin {
                let other_id = if other.begin == bond.begin {
                    &other.end
                } else {
                    &other.begin
                };
                if let Some(neighbor) = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| &node.id == other_id)
                {
                    let point = entry.world_point_for_node(neighbor);
                    score +=
                        (point.x - begin_point.x) * normal_x + (point.y - begin_point.y) * normal_y;
                }
            } else if other.begin == bond.end || other.end == bond.end {
                let other_id = if other.begin == bond.end {
                    &other.end
                } else {
                    &other.begin
                };
                if let Some(neighbor) = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| &node.id == other_id)
                {
                    let point = entry.world_point_for_node(neighbor);
                    score +=
                        (point.x - end_point.x) * normal_x + (point.y - end_point.y) * normal_y;
                }
            }
        }
        if score <= 0.0 {
            Some(DoubleBondPlacement::Left)
        } else {
            Some(DoubleBondPlacement::Right)
        }
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
        let begin_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|other| {
                other.id != bond.id && (other.begin == bond.begin || other.end == bond.begin)
            })
            .count();
        let end_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|other| {
                other.id != bond.id && (other.begin == bond.end || other.end == bond.end)
            })
            .count();
        begin_connections + end_connections >= 2
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
                    DoubleBondPlacement::Right
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
        let begin_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == begin_id || bond.end == begin_id)
            .count();
        let end_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == end_id || bond.end == end_id)
            .count();
        begin_connections + end_connections >= 2
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

    fn selection_render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        let Some(entry) = self.state.document.editable_fragment() else {
            return out;
        };
        let selected_bonds: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        let selected_nodes: BTreeSet<&str> = self
            .state
            .selection
            .nodes
            .iter()
            .map(String::as_str)
            .collect();

        for bond in &entry.fragment.bonds {
            if !selected_bonds.contains(bond.id.as_str()) {
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
            out.push(RenderPrimitive::Line {
                role: RenderRole::SelectionBond,
                object_id: None,
                bond_id: None,
                from: entry.world_point_for_node(begin),
                to: entry.world_point_for_node(end),
                stroke: "rgba(47,111,237,0.72)".to_string(),
                stroke_width: self.options.bond_stroke_width + 5.0,
                dash_array: Vec::new(),
            });
        }

        for node in &entry.fragment.nodes {
            if !selected_nodes.contains(node.id.as_str()) {
                continue;
            }
            out.push(RenderPrimitive::Circle {
                role: RenderRole::SelectionNode,
                object_id: None,
                center: entry.world_point_for_node(node),
                radius: ENDPOINT_FOCUS_RADIUS,
                fill: "rgba(47,111,237,0.16)".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: 1.6,
            });
        }
        out
    }
}

fn apply_node_label_replacement(node: &mut crate::Node, label: &str) -> bool {
    let Some(replacement) = classify_node_label_replacement(label) else {
        return false;
    };

    match replacement {
        NodeLabelReplacement::Carbon => {
            let changed = node.element != "C"
                || node.atomic_number != 6
                || node.is_placeholder
                || node.label.is_some();
            if !changed {
                return false;
            }
            node.element = "C".to_string();
            node.atomic_number = 6;
            node.is_placeholder = false;
            node.label = None;
            true
        }
        NodeLabelReplacement::Element {
            element,
            atomic_number,
        } => {
            let next_label = make_centered_node_label(element, node.position);
            let changed = node.element != element
                || node.atomic_number != atomic_number
                || node.is_placeholder
                || !same_node_label(node.label.as_ref(), Some(&next_label));
            if !changed {
                return false;
            }
            node.element = element.to_string();
            node.atomic_number = atomic_number;
            node.is_placeholder = false;
            node.label = Some(next_label);
            true
        }
        NodeLabelReplacement::Abbreviation(text) => {
            let next_label = make_centered_node_label(text, node.position);
            let changed = node.element != "C"
                || node.atomic_number != 6
                || !node.is_placeholder
                || !same_node_label(node.label.as_ref(), Some(&next_label));
            if !changed {
                return false;
            }
            node.element = "C".to_string();
            node.atomic_number = 6;
            node.is_placeholder = true;
            node.label = Some(next_label);
            true
        }
    }
}

#[derive(Clone, Copy)]
enum NodeLabelReplacement<'a> {
    Carbon,
    Element { element: &'a str, atomic_number: u8 },
    Abbreviation(&'a str),
}

fn classify_node_label_replacement(label: &str) -> Option<NodeLabelReplacement<'_>> {
    match label {
        "C" => Some(NodeLabelReplacement::Carbon),
        "H" => Some(NodeLabelReplacement::Element {
            element: "H",
            atomic_number: 1,
        }),
        "N" => Some(NodeLabelReplacement::Element {
            element: "N",
            atomic_number: 7,
        }),
        "O" => Some(NodeLabelReplacement::Element {
            element: "O",
            atomic_number: 8,
        }),
        "S" => Some(NodeLabelReplacement::Element {
            element: "S",
            atomic_number: 16,
        }),
        "P" => Some(NodeLabelReplacement::Element {
            element: "P",
            atomic_number: 15,
        }),
        "F" => Some(NodeLabelReplacement::Element {
            element: "F",
            atomic_number: 9,
        }),
        "Cl" => Some(NodeLabelReplacement::Element {
            element: "Cl",
            atomic_number: 17,
        }),
        "Br" => Some(NodeLabelReplacement::Element {
            element: "Br",
            atomic_number: 35,
        }),
        "I" => Some(NodeLabelReplacement::Element {
            element: "I",
            atomic_number: 53,
        }),
        "Si" => Some(NodeLabelReplacement::Element {
            element: "Si",
            atomic_number: 14,
        }),
        "Na" => Some(NodeLabelReplacement::Element {
            element: "Na",
            atomic_number: 11,
        }),
        "B" => Some(NodeLabelReplacement::Element {
            element: "B",
            atomic_number: 5,
        }),
        "D" => Some(NodeLabelReplacement::Element {
            element: "D",
            atomic_number: 1,
        }),
        "Me" => Some(NodeLabelReplacement::Abbreviation("Me")),
        "Ph" => Some(NodeLabelReplacement::Abbreviation("Ph")),
        _ => None,
    }
}

fn make_centered_node_label(text: &str, position: [f64; 2]) -> crate::NodeLabel {
    let font_size = 15.0;
    let (label_position, label_box) = estimated_centered_label_geometry(text, position, font_size);
    crate::NodeLabel {
        text: text.to_string(),
        source_text: Some(text.to_string()),
        position: Some(label_position),
        box_field: Some(label_box),
        runs: vec![crate::LabelRun {
            text: text.to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(font_size),
            fill: Some("#000000".to_string()),
            font_weight: Some(700),
            font_style: Some("normal".to_string()),
            script: Some("normal".to_string()),
            face: None,
        }],
        line_runs: Vec::new(),
        lines: Vec::new(),
        align: Some("center".to_string()),
        layout: None,
        attachment: None,
        anchor: Some("middle".to_string()),
        font_family: Some("Arial".to_string()),
        fill: Some("#000000".to_string()),
        font_size: Some(font_size),
        glyph_polygons: Vec::new(),
        box_value: Some(label_box),
        meta: serde_json::Value::Null,
    }
}

fn same_node_label(current: Option<&crate::NodeLabel>, next: Option<&crate::NodeLabel>) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some(current), Some(next)) => current.text == next.text && current.align == next.align,
        _ => false,
    }
}

fn estimated_centered_label_geometry(
    text: &str,
    center: [f64; 2],
    font_size: f64,
) -> ([f64; 2], [f64; 4]) {
    let char_count = text.chars().count().max(1) as f64;
    let width = (font_size * 0.68 * char_count + font_size * 0.10).max(font_size * 0.72);
    let height = (font_size * 0.84).max(8.0);
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let x1 = center[0] - half_width;
    let y1 = center[1] - half_height;
    let x2 = center[0] + half_width;
    let y2 = center[1] + half_height;
    ([center[0], y2], [x1, y1, x2, y2])
}

fn refresh_generated_node_label_geometry_for_node(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    node_id: &str,
    stroke_width: f64,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let Some(label_text) = fragment.nodes[node_index]
        .label
        .as_ref()
        .and_then(|label| is_generated_centered_label(label).then(|| label.text.clone()))
    else {
        return;
    };
    let world_center =
        generated_label_center_world(fragment, node_id, object_translate, stroke_width);
    let local_center = [
        world_center.x - object_translate[0],
        world_center.y - object_translate[1],
    ];
    fragment.nodes[node_index].label = Some(make_centered_node_label(&label_text, local_center));
}

fn is_generated_centered_label(label: &crate::NodeLabel) -> bool {
    label.align.as_deref() == Some("center")
        && label.anchor.as_deref() == Some("middle")
        && label.glyph_polygons.is_empty()
        && label.runs.len() == 1
}

fn generated_label_center_world(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Point {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Point::new(object_translate[0], object_translate[1]);
    };
    let node_world = Point::new(
        object_translate[0] + node.position[0],
        object_translate[1] + node.position[1],
    );
    let connected: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if connected.len() != 1 || connected[0].order != 2 {
        return node_world;
    }
    let bond = connected[0];
    let Some(other) = fragment.nodes.iter().find(|other| {
        (bond.begin == node_id && other.id == bond.end)
            || (bond.end == node_id && other.id == bond.begin)
    }) else {
        return node_world;
    };
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(DoubleBondPlacement::Center);
    if placement == DoubleBondPlacement::Center {
        return node_world;
    }
    let other_world = Point::new(
        object_translate[0] + other.position[0],
        object_translate[1] + other.position[1],
    );
    let dx = other_world.x - node_world.x;
    let dy = other_world.y - node_world.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return node_world;
    }
    let side = if placement == DoubleBondPlacement::Left {
        -1.0
    } else {
        1.0
    };
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let offset =
        0.5 * double_bond_offset_distance_for_points(node_world, other_world, stroke_width) * side;
    Point::new(
        node_world.x + normal_x * offset,
        node_world.y + normal_y * offset,
    )
}

fn double_bond_offset_distance_for_points(start: Point, end: Point, stroke_width: f64) -> f64 {
    const DOUBLE_BOND_OFFSET: f64 = 2.2;
    const VIEWER_BOND_STROKE: f64 = crate::DEFAULT_BOND_STROKE;
    DOUBLE_BOND_OFFSET
        * (stroke_width / VIEWER_BOND_STROKE)
        * ((start.distance(end) / DEFAULT_BOND_LENGTH.max(crate::EPSILON)).max(0.01))
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
        let side_score =
            (other_node.position[0] - shared_node.position[0]) * normal_x
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

    pub fn set_bond_length(&mut self, length: f64) {
        self.options.bond_length = if length > 0.0 {
            length
        } else {
            DEFAULT_BOND_LENGTH
        };
    }
}
