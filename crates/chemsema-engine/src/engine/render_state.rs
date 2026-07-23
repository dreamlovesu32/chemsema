use super::*;

impl Engine {
    pub(super) fn push_interaction_render_primitives(&self, out: &mut Vec<RenderPrimitive>) {
        if let Some(hover) = &self.state.overlay.hover_text_box {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::HoverTextBox,
                object_id: hover.object_id.clone(),
                node_id: hover.node_id.clone(),
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
                let hovered_image_bounds = self
                    .state
                    .document
                    .find_scene_object(&hover.object_id)
                    .filter(|object| object.object_type == "image")
                    .and_then(|object| {
                        select::object_selection_bounds_for_render(&self.state.document, object)
                    });
                if let Some(bounds) = hovered_image_bounds {
                    out.push(RenderPrimitive::Rect {
                        role: RenderRole::HoverObjectBox,
                        object_id: Some(hover.object_id.clone()),
                        node_id: None,
                        x: bounds[0],
                        y: bounds[1],
                        width: bounds[2] - bounds[0],
                        height: bounds[3] - bounds[1],
                        fill: None,
                        stroke: Some("rgba(47,111,237,0.76)".to_string()),
                        stroke_width: HOVER_STROKE_WIDTH,
                        rx: None,
                        ry: None,
                        dash_array: Vec::new(),
                        fill_gradient: None,
                    });
                }
                for handle in &hover.handles {
                    out.push(RenderPrimitive::Circle {
                        role: RenderRole::HoverArrowHandle,
                        object_id: Some(hover.object_id.clone()),
                        node_id: None,
                        center: *handle,
                        radius: crate::px_to_pt(1.5),
                        fill: "#ffffff".to_string(),
                        stroke: "rgba(47,111,237,0.82)".to_string(),
                        stroke_width: HOVER_STROKE_WIDTH,
                    });
                }
            }
        }
        if let Some(hover) = &self.state.overlay.hover_shape {
            for handle in &hover.handles {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::HoverShapeHandle,
                    object_id: Some(hover.object_id.clone()),
                    node_id: None,
                    center: *handle,
                    radius: crate::px_to_pt(1.5),
                    fill: "#ffffff".to_string(),
                    stroke: "rgba(47,111,237,0.82)".to_string(),
                    stroke_width: HOVER_STROKE_WIDTH,
                });
            }
        }
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            if let Some(label_anchor) = &hover.label_anchor {
                out.push(RenderPrimitive::Rect {
                    role: RenderRole::HoverLabelGlyph,
                    object_id: None,
                    node_id: Some(hover.node_id.clone()),
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
                    node_id: Some(hover.node_id.clone()),
                    center: hover.point,
                    radius: endpoint_hover_radius_for_node(
                        &self.state.document,
                        &hover.object_id,
                        &hover.node_id,
                    ),
                    fill: "rgba(47,111,237,0.82)".to_string(),
                    stroke: "rgba(47,111,237,0.96)".to_string(),
                    stroke_width: 0.0,
                });
            }
        }
        if let Some(hover) = &self.state.overlay.hover_bond_center {
            let focus_length = bond_center_focus_length(hover.begin, hover.end);
            if focus_length > crate::EPSILON {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::HoverBondCenter,
                    object_id: None,
                    node_id: None,
                    bond_id: None,
                    points: centered_oriented_rect_points(
                        hover.begin,
                        hover.end,
                        focus_length,
                        hover.width,
                    ),
                    fill: "rgba(47,111,237,0.72)".to_string(),
                    stroke: "none".to_string(),
                    stroke_width: 0.0,
                });
            }
        }
        if self.state.tool.active_tool == Tool::Bond {
            if let Some(preview) = &self.state.overlay.preview {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::PreviewEnd,
                    object_id: None,
                    node_id: None,
                    center: preview.end,
                    radius: self.options.bold_bond_width_world_pt().value()
                        * crate::ENDPOINT_HOVER_RADIUS_BOLD_WIDTH_SCALE,
                    fill: "rgba(47,111,237,0.82)".to_string(),
                    stroke: "none".to_string(),
                    stroke_width: 0.0,
                });
            }
        }
        if let Some((point, count)) = self.template_chain_count_label() {
            out.push(RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                object_id: Some("__preview_chain_count".to_string()),
                node_id: None,
                x: point.x,
                y: point.y,
                baseline_offset: None,
                dominant_baseline: Some("central".to_string()),
                text: count.to_string(),
                font_size: 8.0,
                font_family: Some("Arial".to_string()),
                fill: Some("#000000".to_string()),
                text_anchor: Some("middle".to_string()),
                line_height: None,
                preserve_lines: false,
                box_width: None,
                runs: Vec::new(),
                rotate: 0.0,
                rotate_center: None,
            });
        }
    }

    pub fn interaction_render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_overlay_document() {
            let mut primitives = render_document(&preview_document);
            mark_preview_primitives(&mut primitives);
            primitives.retain(|primitive| render_role_is_preview(primitive.role()));
            primitives
        } else if let Some(object_id) = self.object_edit_preview_object_id() {
            let object_ids = BTreeSet::from([object_id.to_string()]);
            render_document_targets(
                &self.state.document,
                &BTreeSet::new(),
                &BTreeSet::new(),
                &object_ids,
            )
        } else {
            Vec::new()
        };
        if !self.has_active_creation_drag() {
            out.extend(self.selection_render_list());
        }
        self.push_interaction_render_primitives(&mut out);
        out
    }

    pub(super) fn has_active_creation_drag(&self) -> bool {
        self.drag.as_ref().is_some_and(|drag| drag.has_dragged)
            || self
                .arrow_drag
                .as_ref()
                .is_some_and(|drag| drag.has_dragged)
            || self.template_drag.is_some()
            || self
                .shape_drag
                .as_ref()
                .is_some_and(|drag| drag.has_dragged)
            || self
                .orbital_drag
                .as_ref()
                .is_some_and(|drag| drag.has_dragged)
            || self
                .bracket_drag
                .as_ref()
                .is_some_and(|drag| drag.has_dragged)
    }

    pub(super) fn object_edit_preview_object_id(&self) -> Option<&str> {
        self.arrow_edit_drag
            .as_ref()
            .map(|drag| drag.object_id.as_str())
            .or_else(|| {
                self.shape_edit_drag
                    .as_ref()
                    .map(|drag| drag.object_id.as_str())
            })
            .or_else(|| {
                self.bracket_edit_drag
                    .as_ref()
                    .map(|drag| drag.object_id.as_str())
            })
    }

    pub fn render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_document() {
            render_document(&preview_document)
        } else {
            render_document(&self.state.document)
        };
        out.extend(self.selection_render_list());
        self.push_interaction_render_primitives(&mut out);
        out
    }

    pub fn render_targets(
        &self,
        node_ids: &BTreeSet<String>,
        bond_ids: &BTreeSet<String>,
        object_ids: &BTreeSet<String>,
    ) -> Vec<RenderPrimitive> {
        if let Some(preview_document) = self.preview_document() {
            return render_document_targets(&preview_document, node_ids, bond_ids, object_ids);
        }
        render_document_targets(&self.state.document, node_ids, bond_ids, object_ids)
    }

    pub fn preview_render_targets(&self) -> CommandTargets {
        let Some(drag) = self.drag.as_ref().filter(|drag| drag.has_dragged) else {
            return CommandTargets::default();
        };
        let mut nodes = BTreeSet::new();
        nodes.insert(
            drag.anchor
                .node_id
                .clone()
                .unwrap_or_else(|| "__preview_node_begin".to_string()),
        );
        nodes.insert(
            drag.target
                .as_ref()
                .and_then(|target| target.node_id.clone())
                .unwrap_or_else(|| "__preview_node_end".to_string()),
        );
        CommandTargets {
            nodes: nodes.into_iter().collect(),
            bonds: vec!["__preview_bond".to_string()],
            ..Default::default()
        }
    }
}
