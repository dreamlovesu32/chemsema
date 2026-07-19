use super::*;

#[derive(Debug, Clone)]
pub(crate) struct DocumentDiff {
    pub(crate) value: Value,
}

impl DocumentDiff {
    #[cfg(test)]
    pub(crate) fn equal(&self) -> bool {
        self.value
            .get("equal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

pub(crate) fn diff_command(args: &[String]) -> Result<(), String> {
    let mut before = None;
    let mut after = None;
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if before.is_none() => before = Some(value.to_string()),
            value if after.is_none() => after = Some(value.to_string()),
            value => return Err(format!("Unexpected diff argument '{value}'.")),
        }
        index += 1;
    }
    let before = before.ok_or_else(|| "diff requires a before document.".to_string())?;
    let after = after.ok_or_else(|| "diff requires an after document.".to_string())?;
    let before_bytes =
        fs::read(&before).map_err(|error| format!("Failed to read {before}: {error}"))?;
    let after_bytes =
        fs::read(&after).map_err(|error| format!("Failed to read {after}: {error}"))?;
    let before_document = engine_document(&load_engine_from_file(&before)?)?;
    let after_document = if before_bytes == after_bytes {
        before_document.clone()
    } else {
        engine_document(&load_engine_from_file(&after)?)?
    };
    let diff = document_diff(&before_document, &after_document)?;
    write_json_value(diff.value, output.as_deref(), pretty)
}

pub(crate) fn document_diff(
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
) -> Result<DocumentDiff, String> {
    let before_value = serde_json::to_value(before).map_err(|error| error.to_string())?;
    let after_value = serde_json::to_value(after).map_err(|error| error.to_string())?;
    let mut changes = Vec::new();
    let document = compare_single_json(
        "document",
        "document",
        before_value.get("document"),
        after_value.get("document"),
        &mut changes,
    );
    let page = compare_single_json(
        "document:page",
        "document.page",
        before_value.pointer("/document/page"),
        after_value.pointer("/document/page"),
        &mut changes,
    );
    let objects = compare_object_collection(before, after, &mut changes)?;
    let resources = compare_map_collection(
        "resource:",
        "resources",
        before_value.get("resources"),
        after_value.get("resources"),
        &mut changes,
    );
    let styles = compare_map_collection(
        "style:",
        "styles",
        before_value.get("styles"),
        after_value.get("styles"),
        &mut changes,
    );
    let (nodes, bonds) = compare_molecule_entities(before, after, &mut changes)?;
    sort_changes(&mut changes);
    let equal = changes.is_empty();
    let counts = json!({
        "document": collection_change_count(&document),
        "page": collection_change_count(&page),
        "objects": collection_change_count(&objects),
        "resources": collection_change_count(&resources),
        "styles": collection_change_count(&styles),
        "nodes": collection_change_count(&nodes),
        "bonds": collection_change_count(&bonds),
        "fields": changes.len(),
        "total": collection_change_count(&document)
            + collection_change_count(&page)
            + collection_change_count(&objects)
            + collection_change_count(&resources)
            + collection_change_count(&styles)
            + collection_change_count(&nodes)
            + collection_change_count(&bonds),
    });
    Ok(DocumentDiff {
        value: json!({
            "schema": "chemsema.document.diff.v1",
            "ok": true,
            "equal": equal,
            "document": document,
            "page": page,
            "objects": objects,
            "resources": resources,
            "styles": styles,
            "nodes": nodes,
            "bonds": bonds,
            "changes": changes,
            "unexpectedChanges": [],
            "counts": counts,
        }),
    })
}

fn compare_single_json(
    selector: &str,
    path: &str,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<Value>,
) -> Value {
    if before == after {
        json!({ "updated": false })
    } else {
        diff_json_fields(
            selector,
            path,
            before.unwrap_or(&Value::Null),
            after.unwrap_or(&Value::Null),
            changes,
        );
        json!({ "updated": true })
    }
}

fn compare_object_collection(
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
    changes: &mut Vec<Value>,
) -> Result<Value, String> {
    let before_map = flatten_objects_json(before)?;
    let after_map = flatten_objects_json(after)?;
    compare_id_maps("object:", "objects", &before_map, &after_map, changes)
}

fn compare_map_collection(
    selector_prefix: &str,
    path_prefix: &str,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<Value>,
) -> Value {
    let before_map = value_object_map(before);
    let after_map = value_object_map(after);
    compare_id_maps(
        selector_prefix,
        path_prefix,
        &before_map,
        &after_map,
        changes,
    )
    .unwrap_or_else(|error| json!({ "error": error }))
}

fn compare_molecule_entities(
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
    changes: &mut Vec<Value>,
) -> Result<(Value, Value), String> {
    let before_value = serde_json::to_value(before).map_err(|error| error.to_string())?;
    let after_value = serde_json::to_value(after).map_err(|error| error.to_string())?;
    let before_nodes = molecule_entity_map(&before_value, "nodes");
    let after_nodes = molecule_entity_map(&after_value, "nodes");
    let before_bonds = molecule_entity_map(&before_value, "bonds");
    let after_bonds = molecule_entity_map(&after_value, "bonds");
    Ok((
        compare_id_maps(
            "node:",
            "resources.*.data.nodes",
            &before_nodes,
            &after_nodes,
            changes,
        )?,
        compare_id_maps(
            "bond:",
            "resources.*.data.bonds",
            &before_bonds,
            &after_bonds,
            changes,
        )?,
    ))
}

fn flatten_objects_json(document: &ChemSemaDocument) -> Result<BTreeMap<String, Value>, String> {
    let mut out = BTreeMap::new();
    for object in &document.objects {
        flatten_object_json(object, &mut out)?;
    }
    Ok(out)
}

fn flatten_object_json(
    object: &SceneObject,
    out: &mut BTreeMap<String, Value>,
) -> Result<(), String> {
    out.insert(
        object.id.clone(),
        serde_json::to_value(object).map_err(|error| error.to_string())?,
    );
    for child in &object.children {
        flatten_object_json(child, out)?;
    }
    Ok(())
}

fn value_object_map(value: Option<&Value>) -> BTreeMap<String, Value> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn molecule_entity_map(document: &Value, key: &str) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let Some(resources) = document.get("resources").and_then(Value::as_object) else {
        return out;
    };
    for resource in resources.values() {
        let Some(items) = resource
            .get("data")
            .and_then(|data| data.get(key))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for item in items {
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                out.insert(id.to_string(), item.clone());
            }
        }
    }
    out
}

fn compare_id_maps(
    selector_prefix: &str,
    path_prefix: &str,
    before: &BTreeMap<String, Value>,
    after: &BTreeMap<String, Value>,
    changes: &mut Vec<Value>,
) -> Result<Value, String> {
    let before_ids = before.keys().cloned().collect::<BTreeSet<_>>();
    let after_ids = after.keys().cloned().collect::<BTreeSet<_>>();
    let created = after_ids
        .difference(&before_ids)
        .map(|id| format!("{selector_prefix}{id}"))
        .collect::<Vec<_>>();
    let deleted = before_ids
        .difference(&after_ids)
        .map(|id| format!("{selector_prefix}{id}"))
        .collect::<Vec<_>>();
    let mut updated = Vec::new();
    for id in before_ids.intersection(&after_ids) {
        let before_value = before
            .get(id)
            .ok_or_else(|| format!("Missing before value for {id}."))?;
        let after_value = after
            .get(id)
            .ok_or_else(|| format!("Missing after value for {id}."))?;
        if before_value != after_value {
            updated.push(format!("{selector_prefix}{id}"));
            diff_json_fields(
                &format!("{selector_prefix}{id}"),
                &format!("{path_prefix}.{id}"),
                before_value,
                after_value,
                changes,
            );
        }
    }
    Ok(json!({
        "created": created,
        "deleted": deleted,
        "updated": updated,
    }))
}

fn diff_json_fields(
    selector: &str,
    path: &str,
    before: &Value,
    after: &Value,
    changes: &mut Vec<Value>,
) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(before_map), Value::Object(after_map)) => {
            let keys = before_map
                .keys()
                .chain(after_map.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            for key in keys {
                diff_json_fields(
                    selector,
                    &format!("{path}.{key}"),
                    before_map.get(&key).unwrap_or(&Value::Null),
                    after_map.get(&key).unwrap_or(&Value::Null),
                    changes,
                );
            }
        }
        (Value::Array(before_items), Value::Array(after_items)) => {
            let len = before_items.len().max(after_items.len());
            for index in 0..len {
                diff_json_fields(
                    selector,
                    &format!("{path}[{index}]"),
                    before_items.get(index).unwrap_or(&Value::Null),
                    after_items.get(index).unwrap_or(&Value::Null),
                    changes,
                );
            }
        }
        _ => changes.push(json!({
            "selector": selector,
            "path": path,
            "before": before,
            "after": after,
        })),
    }
}

fn collection_change_count(value: &Value) -> usize {
    if value.get("updated").and_then(Value::as_bool).is_some() {
        return usize::from(value.get("updated").and_then(Value::as_bool) == Some(true));
    }
    ["created", "updated", "deleted"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(Value::as_array))
        .map(Vec::len)
        .sum()
}

fn sort_changes(changes: &mut [Value]) {
    changes.sort_by(|left, right| {
        let left_key = (
            left.get("selector").and_then(Value::as_str).unwrap_or(""),
            left.get("path").and_then(Value::as_str).unwrap_or(""),
        );
        let right_key = (
            right.get("selector").and_then(Value::as_str).unwrap_or(""),
            right.get("path").and_then(Value::as_str).unwrap_or(""),
        );
        left_key.cmp(&right_key)
    });
}
