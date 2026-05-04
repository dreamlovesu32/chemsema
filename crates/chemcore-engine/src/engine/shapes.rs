use super::*;

impl Engine {
    pub(super) fn pointer_down_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.shape_drag = Some(ShapeDragState {
            start: point,
            current: point,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_shape(&mut self, event: PointerEvent) {
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

    pub(super) fn pointer_up_shape(&mut self, event: PointerEvent) {
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

    pub(super) fn shape_preview_document(&self) -> Option<ChemcoreDocument> {
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

    pub(super) fn insert_shape_from_drag(&mut self, drag: &ShapeDragState) -> bool {
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

    pub(super) fn shape_scene_object(
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
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some(bbox),
                extra,
            },
        })
    }

    pub(super) fn pending_shape_style(&self) -> JsonValue {
        let color = self.state.tool.shape_color.clone();
        let stroke_width = self.options.graphic_stroke_world_cm().value();
        match self.state.tool.shape_style {
            ShapeStyle::Solid => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
            }),
            ShapeStyle::Dashed => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [SHAPE_DASH_LENGTH],
            }),
            ShapeStyle::Shaded => json!({
                "kind": "shape",
                "fill": color,
                "stroke": color,
                "strokeWidth": stroke_width,
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
                "strokeWidth": stroke_width,
                "dashArray": [],
                "shadow": true,
                "shadowSize": 4.0,
            }),
        }
    }

    pub(super) fn next_shape_z_index(&self) -> i32 {
        self.state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(10)
            + 1
    }
}
