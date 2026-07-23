use super::*;

impl Engine {
    pub(super) fn execute_load_document_command(
        &mut self,
        command: EditorCommand,
        format: DocumentCommandFormat,
        content: &str,
        bytes: &[u8],
    ) -> Result<CommandResult, String> {
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        self.load_document_content(format, content, bytes)?;
        let mut result = self.command_result_from_diff(
            Some(command),
            before_revision,
            &before_document,
            &self.state.document,
        );
        result.output = Some(json!({
            "format": document_command_format_name(format),
            "summary": self.inspect_document_output(&["summary".to_string()])
        }));
        self.last_command_result = Some(result.clone());
        Ok(result)
    }

    pub(super) fn execute_export_document_command(
        &mut self,
        command: EditorCommand,
        format: DocumentCommandFormat,
    ) -> Result<CommandResult, String> {
        Ok(self.readonly_command_result(
            Some(command),
            self.export_document_output(&self.state.document, format)?,
        ))
    }

    pub(super) fn execute_convert_document_command(
        &mut self,
        command: EditorCommand,
        from: DocumentCommandFormat,
        to: DocumentCommandFormat,
        content: &str,
        bytes: &[u8],
    ) -> Result<CommandResult, String> {
        let document = document_from_command_content(from, content, bytes)?;
        Ok(
            self.readonly_command_result(
                Some(command),
                self.export_document_output(&document, to)?,
            ),
        )
    }

    pub(super) fn load_document_content(
        &mut self,
        format: DocumentCommandFormat,
        content: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        match format {
            DocumentCommandFormat::Json | DocumentCommandFormat::Ccjs => {
                self.load_document_json(content)
            }
            DocumentCommandFormat::Cdxml => self.load_cdxml_document(content),
            DocumentCommandFormat::Cdx => self.load_cdx_document(bytes),
            DocumentCommandFormat::Sdf => self.load_sdf_document(content),
            DocumentCommandFormat::Svg => Err(
                "SVG is an export format and cannot be loaded as an editable document.".to_string(),
            ),
        }
    }

    pub(super) fn export_document_output(
        &self,
        document: &ChemSemaDocument,
        format: DocumentCommandFormat,
    ) -> Result<JsonValue, String> {
        let format_name = document_command_format_name(format);
        match format {
            DocumentCommandFormat::Json | DocumentCommandFormat::Ccjs => {
                let content = serde_json::to_string(document).map_err(|error| error.to_string())?;
                Ok(json!({
                    "format": format_name,
                    "mediaType": "application/json",
                    "encoding": "utf-8",
                    "content": content
                }))
            }
            DocumentCommandFormat::Cdxml => Ok(json!({
                "format": format_name,
                "mediaType": "chemical/x-cdxml",
                "encoding": "utf-8",
                "content": crate::document_to_cdxml(document)
            })),
            DocumentCommandFormat::Cdx => Ok(json!({
                "format": format_name,
                "mediaType": "chemical/x-cdx",
                "encoding": "bytes",
                "bytes": crate::document_to_cdx(document)?
            })),
            DocumentCommandFormat::Sdf => Ok(json!({
                "format": format_name,
                "mediaType": "chemical/x-mdl-sdfile",
                "encoding": "utf-8",
                "content": crate::document_to_sdf(document)?
            })),
            DocumentCommandFormat::Svg => Ok(json!({
                "format": format_name,
                "mediaType": "image/svg+xml",
                "encoding": "utf-8",
                "content": crate::document_to_svg(document)
            })),
        }
    }

    pub(super) fn inspect_document_output(&self, include: &[String]) -> JsonValue {
        let include_all = include.is_empty();
        let wants = |name: &str| {
            include_all || include.iter().any(|value| value.eq_ignore_ascii_case(name))
        };
        let mut output = serde_json::Map::new();
        if wants("summary") {
            output.insert("summary".to_string(), self.document_summary_json());
        }
        if wants("objects") {
            output.insert("objects".to_string(), self.document_objects_json());
        }
        if wants("molecules") {
            output.insert("molecules".to_string(), self.document_molecules_json());
        }
        if wants("resources") {
            output.insert("resources".to_string(), self.document_resources_json());
        }
        if wants("styles") {
            output.insert("styles".to_string(), self.document_styles_json());
        }
        JsonValue::Object(output)
    }

    pub(super) fn plan_bond_command_output(
        &self,
        begin: CommandAnchor,
        cursor: Option<Point>,
        angle: Option<f64>,
        bond_length: Option<f64>,
        order: u8,
        variant: BondVariant,
    ) -> JsonValue {
        let anchor = bond_anchor_from_command(begin.clone());
        let default_angle =
            default_angle_for_anchor_for_variant(&self.state.document, &anchor, variant);
        let (angle_deg, angle_source) = if let Some(angle) = angle {
            (normalize_angle(angle), "explicit-angle")
        } else if let Some(cursor) = cursor {
            (
                snapped_angle_for_anchor(&self.state.document, &anchor, cursor),
                "cursor-snap",
            )
        } else {
            (default_angle, "default-angle")
        };
        let length = bond_length
            .unwrap_or_else(|| self.options.bond_length_world_pt().value())
            .max(crate::EPSILON);
        let end_point =
            endpoint_from_angle_for_document(&self.state.document, &anchor, angle_deg, length);
        let end = CommandAnchor {
            node_id: None,
            object_id: begin.object_id.clone(),
            x: end_point.x,
            y: end_point.y,
        };
        let command = json!({
            "type": "add-bond",
            "begin": begin,
            "end": end,
            "order": order,
            "variant": variant,
        });
        json!({
            "schema": "chemsema.plan.bond.v1",
            "begin": command["begin"].clone(),
            "end": command["end"].clone(),
            "angleDeg": angle_deg,
            "angleSource": angle_source,
            "defaultAngleDeg": default_angle,
            "bondLength": length,
            "order": order,
            "variant": variant,
            "globalSnapAngles": GLOBAL_SNAP_ANGLES,
            "keypadSlots": bond_plan_keypad_slots(
                &self.state.document,
                &anchor,
                default_angle,
                length,
            ),
            "command": command,
        })
    }

    pub(super) fn document_summary_json(&self) -> JsonValue {
        let objects = self.state.document.scene_objects();
        let mut object_types = BTreeMap::<String, usize>::new();
        for object in &objects {
            *object_types.entry(object.object_type.clone()).or_default() += 1;
        }
        let molecule_count = self.state.document.editable_fragments().len();
        let node_count = self
            .state
            .document
            .editable_fragments()
            .iter()
            .map(|entry| entry.fragment.nodes.len())
            .sum::<usize>();
        let bond_count = self
            .state
            .document
            .editable_fragments()
            .iter()
            .map(|entry| entry.fragment.bonds.len())
            .sum::<usize>();
        json!({
            "title": &self.state.document.document.title,
            "documentId": &self.state.document.document.id,
            "format": &self.state.document.format,
            "page": &self.state.document.document.page,
            "revision": self.revision,
            "documentStylePreset": &self.document_style_preset,
            "counts": {
                "objects": objects.len(),
                "objectTypes": object_types,
                "molecules": molecule_count,
                "nodes": node_count,
                "bonds": bond_count,
                "styles": self.state.document.styles.len(),
                "resources": self.state.document.resources.len()
            },
            "renderBounds": self.render_bounds(RenderBoundsScope::Document),
            "import": self.state.document.document.meta.get("import").cloned()
        })
    }

    pub(super) fn document_objects_json(&self) -> JsonValue {
        JsonValue::Array(
            self.state
                .document
                .scene_objects()
                .into_iter()
                .map(|object| {
                    json!({
                        "id": &object.id,
                        "type": &object.object_type,
                        "name": &object.name,
                        "visible": object.visible,
                        "locked": object.locked,
                        "zIndex": object.z_index,
                        "styleRef": &object.style_ref,
                        "resourceRef": &object.payload.resource_ref,
                        "bbox": &object.payload.bbox,
                        "transform": &object.transform,
                        "childCount": object.children.len()
                    })
                })
                .collect(),
        )
    }

    pub(super) fn document_molecules_json(&self) -> JsonValue {
        JsonValue::Array(
            self.state
                .document
                .editable_fragments()
                .into_iter()
                .map(|entry| {
                    json!({
                        "objectId": &entry.object.id,
                        "resourceRef": &entry.object.payload.resource_ref,
                        "nodeCount": entry.fragment.nodes.len(),
                        "bondCount": entry.fragment.bonds.len(),
                        "bbox": entry.fragment.bbox,
                        "nodes": entry.fragment.nodes.iter().map(|node| {
                            json!({
                                "id": &node.id,
                                "element": &node.element,
                                "atomicNumber": node.atomic_number,
                                "position": &node.position,
                                "charge": node.charge,
                                "label": node.label.as_ref().map(|label| {
                                    json!({
                                        "text": &label.text,
                                        "sourceText": &label.source_text,
                                        "bbox": label.bbox()
                                    })
                                })
                            })
                        }).collect::<Vec<_>>(),
                        "bonds": entry.fragment.bonds.iter().map(|bond| {
                            json!({
                                "id": &bond.id,
                                "begin": &bond.begin,
                                "end": &bond.end,
                                "order": bond.order,
                                "stereo": &bond.stereo,
                                "lineStyles": &bond.line_styles
                            })
                        }).collect::<Vec<_>>()
                    })
                })
                .collect(),
        )
    }

    pub(super) fn document_resources_json(&self) -> JsonValue {
        JsonValue::Array(
            self.state
                .document
                .resources
                .iter()
                .map(|(id, resource)| {
                    let mut item = serde_json::Map::new();
                    item.insert("id".to_string(), json!(id));
                    item.insert("type".to_string(), json!(&resource.resource_type));
                    item.insert("encoding".to_string(), json!(&resource.encoding));
                    match &resource.data {
                        ResourceData::Fragment(fragment) => {
                            item.insert("kind".to_string(), json!("fragment"));
                            item.insert("nodeCount".to_string(), json!(fragment.nodes.len()));
                            item.insert("bondCount".to_string(), json!(fragment.bonds.len()));
                        }
                        ResourceData::Text(text) => {
                            item.insert("kind".to_string(), json!("text"));
                            item.insert("textLength".to_string(), json!(text.len()));
                        }
                        ResourceData::Json(value) => {
                            item.insert("kind".to_string(), json!("json"));
                            item.insert("jsonType".to_string(), json!(json_value_type_name(value)));
                        }
                    }
                    JsonValue::Object(item)
                })
                .collect(),
        )
    }

    pub(super) fn document_styles_json(&self) -> JsonValue {
        JsonValue::Array(
            self.state
                .document
                .styles
                .iter()
                .map(|(id, style)| {
                    json!({
                        "id": id,
                        "kind": style.get("kind").and_then(JsonValue::as_str),
                        "stroke": style.get("stroke").and_then(JsonValue::as_str),
                        "fill": style.get("fill").cloned(),
                        "strokeWidth": style.get("strokeWidth").and_then(JsonValue::as_f64),
                        "fontFamily": style.get("fontFamily").and_then(JsonValue::as_str),
                        "fontSize": style.get("fontSize").and_then(JsonValue::as_f64)
                    })
                })
                .collect(),
        )
    }
}
