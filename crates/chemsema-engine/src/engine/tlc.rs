use super::*;

impl Engine {
    pub fn tlc_spot_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        let mut best: Option<(f64, TlcSpotHit)> = None;
        for object in self.state.document.scene_objects() {
            let Some(geometry) = tlc_plate_geometry(object) else {
                continue;
            };
            for (lane_index, lane_x) in geometry.lane_centers.iter().enumerate() {
                let Some(spots) = geometry.spots.get(lane_index) else {
                    continue;
                };
                for (spot_index, rf) in spots.iter().enumerate() {
                    let local_center = Point::new(
                        *lane_x,
                        geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * *rf,
                    );
                    let center = rotate_point(local_center, geometry.center, geometry.rotate);
                    let distance = center.distance(point);
                    if distance > geometry.spot_radius + px_to_pt(6.0) {
                        continue;
                    }
                    let hit = TlcSpotHit {
                        object_id: object.id.clone(),
                        lane_index,
                        spot_index,
                        rf: round2(*rf),
                        center,
                        guide_points: tlc_lane_guide_points(&geometry, lane_index),
                    };
                    match &best {
                        Some((best_distance, _)) if *best_distance <= distance => {}
                        _ => best = Some((distance, hit)),
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }

    pub fn begin_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let hit = self.tlc_spot_hit_test(point)?;
        self.tlc_spot_drag = Some(TlcSpotDragState {
            initial_rf: hit.rf,
            hit: hit.clone(),
            changed: false,
            undo_pushed: false,
        });
        Some(hit)
    }

    pub fn update_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let command = self.tlc_spot_drag_command()?;
        let mut next = None;
        self.with_transient_command(command, |engine| {
            next = engine.update_tlc_spot_drag_untracked(point);
            next.is_some()
        });
        next
    }

    pub(super) fn update_tlc_spot_drag_untracked(&mut self, point: Point) -> Option<TlcSpotHit> {
        let drag = self.tlc_spot_drag.clone()?;
        let next_rf = self.tlc_spot_rf_at_point(&drag.hit.object_id, drag.hit.lane_index, point)?;
        let changed = (drag.hit.rf - next_rf).abs() > 0.0001;
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
        }
        let next = self.update_tlc_spot_to_point(
            &drag.hit.object_id,
            drag.hit.lane_index,
            drag.hit.spot_index,
            point,
        )?;
        if let Some(active_drag) = &mut self.tlc_spot_drag {
            active_drag.changed |= changed;
            active_drag.undo_pushed |= changed;
            active_drag.hit = next.clone();
        }
        Some(next)
    }

    pub fn finish_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let had_drag = self.tlc_spot_drag.is_some();
        let next = if had_drag {
            self.update_tlc_spot_drag(point)
        } else {
            None
        };
        let changed = self.tlc_spot_drag.as_ref().is_some_and(|drag| drag.changed);
        let undo_pushed = self
            .tlc_spot_drag
            .as_ref()
            .is_some_and(|drag| drag.undo_pushed);
        self.tlc_spot_drag = None;
        if had_drag && undo_pushed && !changed {
            self.undo_stack.pop();
        }
        next
    }

    pub(super) fn tlc_spot_drag_command(&self) -> Option<EditorCommand> {
        let drag = self.tlc_spot_drag.as_ref()?;
        Some(EditorCommand::MoveTlcSpot {
            object_id: drag.hit.object_id.clone(),
            lane_index: drag.hit.lane_index,
            spot_index: drag.hit.spot_index,
            before_rf: drag.initial_rf,
        })
    }

    pub fn tlc_lane_guide_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        if self.tlc_spot_hit_test(point).is_some() {
            return None;
        }
        for object in self.state.document.scene_objects() {
            let Some(geometry) = tlc_plate_geometry(object) else {
                continue;
            };
            for (lane_index, spots) in geometry.spots.iter().enumerate() {
                let guide_points = tlc_lane_guide_points(&geometry, lane_index);
                if !point_in_polygon(point, &guide_points) {
                    continue;
                }
                let rf = spots.first().copied().unwrap_or(0.15);
                let lane_x = *geometry.lane_centers.get(lane_index)?;
                let local_center = Point::new(
                    lane_x,
                    geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
                );
                return Some(TlcSpotHit {
                    object_id: object.id.clone(),
                    lane_index,
                    spot_index: 0,
                    rf: round2(rf),
                    center: rotate_point(local_center, geometry.center, geometry.rotate),
                    guide_points,
                });
            }
        }
        None
    }

    pub(super) fn update_tlc_spot_to_point(
        &mut self,
        object_id: &str,
        lane_index: usize,
        spot_index: usize,
        point: Point,
    ) -> Option<TlcSpotHit> {
        let object = self.state.document.find_scene_object_mut(object_id)?;
        let geometry = tlc_plate_geometry(object)?;
        let local_point = rotate_point(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        let rf = ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
            .clamp(0.0, 1.0);
        let lanes = object.payload.extra.get_mut("lanes")?.as_array_mut()?;
        let lane = lanes.get_mut(lane_index)?.as_object_mut()?;
        let spots = lane.get_mut("spots")?.as_array_mut()?;
        let spot = spots.get_mut(spot_index)?.as_object_mut()?;
        spot.insert("rf".to_string(), json!(round2(rf)));
        let lane_x = *geometry.lane_centers.get(lane_index)?;
        let local_center = Point::new(
            lane_x,
            geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
        );
        Some(TlcSpotHit {
            object_id: object_id.to_string(),
            lane_index,
            spot_index,
            rf: round2(rf),
            center: rotate_point(local_center, geometry.center, geometry.rotate),
            guide_points: tlc_lane_guide_points(&geometry, lane_index),
        })
    }

    pub(super) fn tlc_spot_rf_at_point(
        &self,
        object_id: &str,
        lane_index: usize,
        point: Point,
    ) -> Option<f64> {
        let object = self.state.document.find_scene_object(object_id)?;
        let geometry = tlc_plate_geometry(object)?;
        let local_point = rotate_point(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        geometry.lane_centers.get(lane_index)?;
        Some(round2(
            ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
                .clamp(0.0, 1.0),
        ))
    }
}
