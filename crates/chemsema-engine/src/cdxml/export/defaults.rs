use super::*;

pub(super) fn cdxml_editing_scale(document: &ChemSemaDocument) -> f64 {
    document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .and_then(Value::as_f64)
        .filter(|value| *value > crate::EPSILON)
        .unwrap_or(1.0)
}

pub(super) fn export_cdxml_defaults(document: &ChemSemaDocument) -> CdxmlDefaults {
    let mut defaults = CdxmlDefaults::default();
    if let Some(import_defaults) = document
        .document
        .meta
        .get("import")
        .and_then(|value| value.get("cdxml"))
        .and_then(|value| value.get("defaults"))
    {
        if let Some(value) = import_defaults.get("bondLength").and_then(Value::as_f64) {
            defaults.bond_length = value;
        }
        if let Some(value) = import_defaults.get("lineWidth").and_then(Value::as_f64) {
            defaults.line_width = value;
        }
        if let Some(value) = import_defaults.get("boldWidth").and_then(Value::as_f64) {
            defaults.bold_width = value;
        }
        if let Some(value) = import_defaults.get("hashSpacing").and_then(Value::as_f64) {
            defaults.hash_spacing = value;
        }
        if let Some(value) = import_defaults.get("bondSpacing").and_then(Value::as_f64) {
            defaults.bond_spacing = value;
        }
        if let Some(value) = import_defaults.get("marginWidth").and_then(Value::as_f64) {
            defaults.margin_width = value;
        }
        if let Some(value) = import_defaults.get("chainAngle").and_then(Value::as_f64) {
            defaults.chain_angle = value;
        }
        if let Some(value) = import_defaults
            .get("labelJustification")
            .and_then(value_cdxml_justification)
        {
            defaults.label_justification = value;
        }
        if let Some(value) = import_defaults
            .get("captionJustification")
            .and_then(value_cdxml_justification)
        {
            defaults.caption_justification = value;
        }
        if let Some(value) = import_defaults
            .get("fractionalWidths")
            .and_then(Value::as_bool)
        {
            defaults.fractional_widths = value;
        }
        if let Some(value) = import_defaults
            .get("interpretChemically")
            .and_then(Value::as_bool)
        {
            defaults.interpret_chemically = Some(value);
        }
        if let Some(value) = import_defaults
            .get("showAtomQuery")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_query = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomEnhancedStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_enhanced_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showAtomNumber")
            .and_then(Value::as_bool)
        {
            defaults.show_atom_number = value;
        }
        if let Some(value) = import_defaults
            .get("showResidueID")
            .and_then(Value::as_bool)
        {
            defaults.show_residue_id = value;
        }
        if let Some(value) = import_defaults
            .get("showBondQuery")
            .and_then(Value::as_bool)
        {
            defaults.show_bond_query = value;
        }
        if let Some(value) = import_defaults.get("showBondRxn").and_then(Value::as_bool) {
            defaults.show_bond_rxn = value;
        }
        if let Some(value) = import_defaults
            .get("showBondStereo")
            .and_then(Value::as_bool)
        {
            defaults.show_bond_stereo = value;
        }
        if let Some(value) = import_defaults
            .get("showTerminalCarbonLabels")
            .and_then(Value::as_bool)
        {
            defaults.show_terminal_carbon_labels = value;
        }
        if let Some(value) = import_defaults
            .get("showNonTerminalCarbonLabels")
            .and_then(Value::as_bool)
        {
            defaults.show_non_terminal_carbon_labels = value;
        }
        if let Some(value) = import_defaults
            .get("hideImplicitHydrogens")
            .and_then(Value::as_bool)
        {
            defaults.hide_implicit_hydrogens = value;
        }
        if let Some(value) = import_defaults.get("printMargins").and_then(value_margins) {
            defaults.print_margins = value;
        }
    }
    if let Some(value) = document.style.defaults.get("bondLength") {
        defaults.bond_length = *value;
    }
    if let Some(value) = document.style.defaults.get("chainAngle") {
        defaults.chain_angle = *value;
    }
    if let Some(value) = document.style.defaults.get("lineWidth") {
        defaults.line_width = *value;
    }
    if let Some(value) = document.style.defaults.get("boldWidth") {
        defaults.bold_width = *value;
    }
    if let Some(value) = document.style.defaults.get("hashSpacing") {
        defaults.hash_spacing = *value;
    }
    if let Some(value) = document.style.defaults.get("bondSpacing") {
        defaults.bond_spacing = *value;
    }
    if let Some(value) = document.style.defaults.get("marginWidth") {
        defaults.margin_width = *value;
    }
    defaults.label_size = document.style.label_style.font_size;
    defaults.caption_size = document.style.caption_style.font_size;
    defaults.label_face = cdxml_face_for_document_text_style(&document.style.label_style);
    defaults.caption_face = cdxml_face_for_document_text_style(&document.style.caption_style);
    if let Some(style) = document.styles.get("style_molecule_default") {
        if let Some(value) = style_number_value(style, "strokeWidth") {
            defaults.line_width = value;
        }
    }
    for resource in document.resources.values() {
        let ResourceData::Fragment(fragment) = &resource.data else {
            continue;
        };
        if let Some(bond) = fragment.bonds.first() {
            defaults.line_width = bond.stroke_width;
            if let Some(value) = bond.bold_width {
                defaults.bold_width = value;
            }
            if let Some(value) = bond.hash_spacing {
                defaults.hash_spacing = value;
            }
            if let Some(value) = bond.margin_width {
                defaults.margin_width = value;
            }
            break;
        }
    }
    if let Some(value) = document.objects.iter().find_map(|object| {
        (object.object_type == "symbol")
            .then(|| {
                object
                    .payload
                    .extra
                    .get("symbolLineWidth")
                    .and_then(Value::as_f64)
            })
            .flatten()
    }) {
        defaults.line_width = value;
    }
    defaults
}

pub(super) fn cdxml_face_for_document_text_style(style: &DocumentTextStyle) -> u32 {
    let mut face = 0;
    if style.font_weight >= 600 {
        face |= 1;
    }
    if style.font_style.eq_ignore_ascii_case("italic") {
        face |= 2;
    }
    if style.underline {
        face |= 4;
    }
    if style.outline {
        face |= 8;
    }
    if style.shadow {
        face |= 16;
    }
    face |= match style.script.trim().to_ascii_lowercase().as_str() {
        "subscript" => 32,
        "superscript" => 64,
        "chemical" => 96,
        _ => 0,
    };
    face
}

pub(super) fn value_cdxml_justification(value: &Value) -> Option<CdxmlJustification> {
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(CdxmlJustification::Auto),
        "left" | "start" => Some(CdxmlJustification::Left),
        "center" | "middle" => Some(CdxmlJustification::Center),
        "right" | "end" => Some(CdxmlJustification::Right),
        "full" => Some(CdxmlJustification::Full),
        "above" => Some(CdxmlJustification::Above),
        "below" => Some(CdxmlJustification::Below),
        "best" => Some(CdxmlJustification::Best),
        _ => None,
    }
}

pub(super) fn value_margins(value: &Value) -> Option<[f64; 4]> {
    let values = value.as_array()?;
    Some([
        values.first()?.as_f64()?,
        values.get(1)?.as_f64()?,
        values.get(2)?.as_f64()?,
        values.get(3)?.as_f64()?,
    ])
}
