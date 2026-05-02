use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbbreviationComponent {
    pub label: String,
    pub kind: String,
    pub name: String,
    pub structure: String,
    pub left_anchor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_attachment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbbreviationRecognition {
    pub label: String,
    pub canonical_label: String,
    pub kind: String,
    pub formula: String,
    pub anchor_atom: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<AbbreviationComponent>,
}

#[derive(Debug, Clone, Copy)]
struct FragmentDef {
    label: &'static str,
    aliases: &'static [&'static str],
    kind: &'static str,
    name: &'static str,
    structure: &'static str,
    left_anchor: &'static str,
    right_attachment: Option<&'static str>,
}

impl FragmentDef {
    fn component(self, input_label: &str) -> AbbreviationComponent {
        AbbreviationComponent {
            label: canonical_label_for(input_label, self.label),
            kind: self.kind.to_string(),
            name: self.name.to_string(),
            structure: self.structure.to_string(),
            left_anchor: self.left_anchor.to_string(),
            right_attachment: self.right_attachment.map(ToString::to_string),
        }
    }

    fn matches(self, text: &str) -> bool {
        self.label == text || self.aliases.contains(&text)
    }
}

const OPEN_FRAGMENTS: &[FragmentDef] = &[
    FragmentDef {
        label: "CO2",
        aliases: &["COO"],
        kind: "linker",
        name: "ester/carboxyl linker",
        structure: "-C(=O)O-",
        left_anchor: "C",
        right_attachment: Some("O"),
    },
    FragmentDef {
        label: "OCO",
        aliases: &[],
        kind: "linker",
        name: "reverse ester linker",
        structure: "-O-C(=O)-",
        left_anchor: "O",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "SO2",
        aliases: &[],
        kind: "linker",
        name: "sulfonyl linker",
        structure: "-S(=O)2-",
        left_anchor: "S",
        right_attachment: Some("S"),
    },
    FragmentDef {
        label: "SO",
        aliases: &[],
        kind: "linker",
        name: "sulfinyl linker",
        structure: "-S(=O)-",
        left_anchor: "S",
        right_attachment: Some("S"),
    },
    FragmentDef {
        label: "CH2",
        aliases: &[],
        kind: "linker",
        name: "methylene linker",
        structure: "-CH2-",
        left_anchor: "C",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "NH",
        aliases: &[],
        kind: "linker",
        name: "imino linker",
        structure: "-NH-",
        left_anchor: "N",
        right_attachment: Some("N"),
    },
    FragmentDef {
        label: "CO",
        aliases: &[],
        kind: "linker",
        name: "carbonyl linker",
        structure: "-C(=O)-",
        left_anchor: "C",
        right_attachment: Some("C"),
    },
    FragmentDef {
        label: "O",
        aliases: &[],
        kind: "linker",
        name: "oxy linker",
        structure: "-O-",
        left_anchor: "O",
        right_attachment: Some("O"),
    },
];

const N_BRIDGE_FRAGMENT: FragmentDef = FragmentDef {
    label: "N",
    aliases: &[],
    kind: "bridge",
    name: "substituted nitrogen bridge",
    structure: "-N(-)-",
    left_anchor: "N",
    right_attachment: Some("N"),
};

const TERMINAL_FRAGMENTS: &[FragmentDef] = &[
    terminal("R", &[], "R group / generic substituent", "R", "R"),
    terminal("Me", &["CH3"], "methyl", "-CH3", "C"),
    terminal("Et", &["C2H5"], "ethyl", "-CH2CH3", "C"),
    terminal("Pr", &[], "propyl", "-CH2CH2CH3", "C"),
    terminal("nPr", &["n-Pr"], "n-propyl", "-CH2CH2CH3", "C"),
    terminal("iPr", &["i-Pr"], "isopropyl", "-CH(CH3)2", "C"),
    terminal("Bu", &[], "butyl", "-CH2CH2CH2CH3", "C"),
    terminal("nBu", &["n-Bu"], "n-butyl", "-CH2CH2CH2CH3", "C"),
    terminal("iBu", &["i-Bu"], "isobutyl", "-CH2CH(CH3)2", "C"),
    terminal("sBu", &["s-Bu"], "sec-butyl", "-CH(CH3)CH2CH3", "C"),
    terminal("tBu", &["t-Bu"], "tert-butyl", "-C(CH3)3", "C"),
    terminal("Ph", &[], "phenyl", "-C6H5", "C"),
    terminal("Bn", &[], "benzyl", "-CH2Ph", "C"),
    terminal("Bz", &[], "benzoyl", "-C(=O)Ph", "C"),
    terminal("Ac", &[], "acetyl", "-C(=O)CH3", "C"),
    terminal("Piv", &[], "pivaloyl", "-C(=O)tBu", "C"),
    terminal("CHO", &[], "formyl", "-C(=O)H", "C"),
    terminal("CN", &[], "cyano", "-C#N", "C"),
    terminal("NCO", &[], "isocyanato", "-N=C=O", "N"),
    terminal("NCS", &[], "isothiocyanato", "-N=C=S", "N"),
    terminal("SCN", &[], "thiocyanato", "-S-C#N", "S"),
    terminal("NO2", &[], "nitro", "-N(=O)O", "N"),
    terminal("N3", &[], "azido", "-N3", "N"),
    terminal("H", &[], "hydrogen terminator", "-H", "H"),
    terminal("F", &[], "fluoro", "-F", "F"),
    terminal("Cl", &[], "chloro", "-Cl", "Cl"),
    terminal("Br", &[], "bromo", "-Br", "Br"),
    terminal("I", &[], "iodo", "-I", "I"),
    terminal("OH", &[], "hydroxy", "-OH", "O"),
    terminal("NH2", &[], "amino", "-NH2", "N"),
    terminal("Ts", &["Tos"], "tosyl", "-S(=O)2-p-Tol", "S"),
    terminal("Bs", &[], "brosyl", "-S(=O)2-p-BrPh", "S"),
    terminal("Ms", &[], "mesyl", "-S(=O)2CH3", "S"),
    terminal("Tf", &[], "triflyl", "-S(=O)2CF3", "S"),
    terminal("SO3H", &[], "sulfonic acid", "-S(=O)2OH", "S"),
    terminal("SO2H", &[], "sulfinic acid style label", "-S(=O)OH", "S"),
    terminal("SO3", &[], "sulfonate fragment", "-S(=O)3-", "S"),
    terminal("SO4", &[], "sulfate fragment", "SO4", "S"),
    terminal("SO4H", &[], "sulfate monoacid", "SO4H", "O"),
    terminal("PO2", &[], "phosphoryl fragment", "PO2", "P"),
    terminal("PO3", &[], "phosphate fragment", "PO3", "P"),
    terminal("PO3H2", &[], "phosphonic acid", "-P(=O)(OH)2", "P"),
    terminal("PO4", &[], "phosphate", "PO4", "P"),
    terminal("PO4H2", &[], "phosphate acid form", "PO4H2", "O"),
    terminal("Boc", &[], "tert-butyloxycarbonyl", "-C(=O)O-tBu", "C"),
    terminal("Cbz", &[], "benzyloxycarbonyl", "-C(=O)OCH2Ph", "C"),
    terminal(
        "Fmoc",
        &["FMOC"],
        "fluorenylmethoxycarbonyl",
        "-C(=O)OCH2-fluorenyl",
        "C",
    ),
    terminal("TMS", &[], "trimethylsilyl", "-Si(CH3)3", "Si"),
    terminal(
        "TBDMS",
        &[],
        "tert-butyldimethylsilyl",
        "-Si(CH3)2tBu",
        "Si",
    ),
    terminal("TBDPS", &[], "tert-butyldiphenylsilyl", "-Si(Ph)2tBu", "Si"),
    terminal("CCl3", &[], "trichloromethyl", "-CCl3", "C"),
    terminal("CF3", &[], "trifluoromethyl", "-CF3", "C"),
    terminal("CPh3", &[], "trityl", "-CPh3", "C"),
    terminal("Cp", &[], "cyclopentadienyl", "Cp", "C"),
    terminal("Cy", &[], "cyclohexyl", "-C6H11", "C"),
    terminal("Mes", &[], "mesityl", "2,4,6-trimethylphenyl", "C"),
    terminal("NHPh", &[], "anilino", "-NHPh", "N"),
    terminal("Indole", &[], "indolyl / indole template", "Indole", "C"),
    terminal("ster", &[], "generic steric label", "ster", "C"),
];

const fn terminal(
    label: &'static str,
    aliases: &'static [&'static str],
    name: &'static str,
    structure: &'static str,
    left_anchor: &'static str,
) -> FragmentDef {
    FragmentDef {
        label,
        aliases,
        kind: "terminal",
        name,
        structure,
        left_anchor,
        right_attachment: None,
    }
}

pub fn recognize_abbreviation_label(label: &str) -> Option<AbbreviationRecognition> {
    recognize_abbreviation_label_for_connection_count(label, 1)
}

pub fn recognize_abbreviation_label_for_connection_count(
    label: &str,
    connection_count: usize,
) -> Option<AbbreviationRecognition> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    if connection_count == 1 {
        parse_terminal_composite(trimmed).or_else(|| recognize_terminal(trimmed))
    } else if connection_count == 2 {
        recognize_bridge(trimmed)
    } else {
        None
    }
}

pub fn recognized_abbreviation_meta(label: &str) -> Option<Value> {
    recognized_abbreviation_meta_for_connection_count(label, 1)
}

pub fn recognized_abbreviation_meta_for_connection_count(
    label: &str,
    connection_count: usize,
) -> Option<Value> {
    let recognition = recognize_abbreviation_label_for_connection_count(label, connection_count)?;
    let expansion = expansion_for_recognition(&recognition);
    Some(json!({
        "kind": "functional-group",
        "status": "recognized",
        "label": recognition.label,
        "canonicalLabel": recognition.canonical_label,
        "groupKind": recognition.kind,
        "formula": recognition.formula,
        "anchorAtom": recognition.anchor_atom,
        "components": recognition.components,
        "expansion": expansion,
    }))
}

pub fn recognized_abbreviation_uses_whole_label_layout(
    label: &str,
    connection_count: usize,
) -> bool {
    recognize_abbreviation_label_for_connection_count(label, connection_count).is_some_and(
        |recognition| {
            recognition.kind == "terminal-fragment"
                && recognition.components.len() == 1
                && canonical_abbreviation_uses_whole_label_layout(&recognition.canonical_label)
        },
    )
}

pub fn canonical_abbreviation_uses_whole_label_layout(canonical_label: &str) -> bool {
    let mut chars = canonical_label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase() && chars.any(|character| character.is_ascii_uppercase())
}

pub fn invalid_abbreviation_meta(label: &str) -> Value {
    json!({
        "kind": "functional-label",
        "status": "invalid",
        "label": label.trim(),
    })
}

fn recognize_terminal(label: &str) -> Option<AbbreviationRecognition> {
    let terminal = find_terminal(label)?;
    let component = terminal.component(label);
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label: component.label.clone(),
        kind: "terminal-fragment".to_string(),
        formula: component.structure.clone(),
        anchor_atom: component.left_anchor.clone(),
        components: vec![component],
    })
}

fn parse_terminal_composite(label: &str) -> Option<AbbreviationRecognition> {
    let mut rest = label;
    let mut components = Vec::new();
    while !rest.is_empty() {
        if let Some((fragment, matched)) = match_prefix(rest, OPEN_FRAGMENTS) {
            components.push(fragment.component(matched));
            rest = &rest[matched.len()..];
            continue;
        }
        if let Some((terminal, matched)) = match_prefix(rest, TERMINAL_FRAGMENTS) {
            components.push(terminal.component(matched));
            rest = &rest[matched.len()..];
            break;
        }
        return None;
    }
    if !rest.is_empty() || components.len() < 2 {
        return None;
    }
    if components
        .last()
        .and_then(|component| component.right_attachment.as_ref())
        .is_some()
    {
        return None;
    }
    let canonical_label = components
        .iter()
        .map(|component| component.label.as_str())
        .collect::<String>();
    let formula = composite_formula(&components);
    let anchor_atom = components
        .first()
        .map(|component| component.left_anchor.clone())
        .unwrap_or_default();
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label,
        kind: "composite-fragment".to_string(),
        formula,
        anchor_atom,
        components,
    })
}

fn recognize_bridge(label: &str) -> Option<AbbreviationRecognition> {
    if let Some((fragment, matched)) = OPEN_FRAGMENTS
        .iter()
        .find_map(|fragment| fragment.matches(label).then_some((*fragment, label)))
    {
        let mut component = fragment.component(matched);
        component.kind = "bridge".to_string();
        return Some(AbbreviationRecognition {
            label: label.to_string(),
            canonical_label: component.label.clone(),
            kind: "bridge-fragment".to_string(),
            formula: component.structure.clone(),
            anchor_atom: component.left_anchor.clone(),
            components: vec![component],
        });
    }

    let suffix = label.strip_prefix('N')?;
    if suffix.is_empty() {
        return None;
    }
    let terminal = find_terminal(suffix)?;
    let n_component = N_BRIDGE_FRAGMENT.component("N");
    let terminal_component = terminal.component(suffix);
    let canonical_label = format!("N{}", terminal_component.label);
    let formula = format!("-N({})-", terminal_component.label);
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label,
        kind: "bridge-fragment".to_string(),
        formula,
        anchor_atom: "N".to_string(),
        components: vec![n_component, terminal_component],
    })
}

fn match_prefix<'a>(
    text: &'a str,
    fragments: &'static [FragmentDef],
) -> Option<(FragmentDef, &'a str)> {
    fragments
        .iter()
        .filter_map(|fragment| {
            std::iter::once(fragment.label)
                .chain(fragment.aliases.iter().copied())
                .filter(|candidate| text.starts_with(candidate))
                .max_by_key(|candidate| candidate.len())
                .map(|matched| (*fragment, matched))
        })
        .max_by_key(|(_, matched)| matched.len())
}

fn find_terminal(label: &str) -> Option<FragmentDef> {
    TERMINAL_FRAGMENTS
        .iter()
        .copied()
        .find(|fragment| fragment.matches(label))
}

fn composite_formula(components: &[AbbreviationComponent]) -> String {
    let mut formula = String::new();
    for (index, component) in components.iter().enumerate() {
        let part = component.structure.as_str();
        if index == 0 {
            formula.push_str(part.trim_end_matches('-'));
        } else if index + 1 == components.len() {
            formula.push_str(part.trim_start_matches('-'));
        } else {
            formula.push_str(part.trim_matches('-'));
        }
    }
    formula
}

fn canonical_label_for(input_label: &str, canonical: &str) -> String {
    match input_label {
        "COO" => "CO2".to_string(),
        "Tos" => "Ts".to_string(),
        "FMOC" => "Fmoc".to_string(),
        "C2H5" => "Et".to_string(),
        "CH3" => "Me".to_string(),
        "n-Pr" => "nPr".to_string(),
        "i-Pr" => "iPr".to_string(),
        "n-Bu" => "nBu".to_string(),
        "i-Bu" => "iBu".to_string(),
        "s-Bu" => "sBu".to_string(),
        "t-Bu" => "tBu".to_string(),
        _ => canonical.to_string(),
    }
}

#[derive(Default)]
struct ExpansionBuilder {
    atom_counts: BTreeMap<String, usize>,
    atoms: Vec<Value>,
    bonds: Vec<Value>,
}

struct FragmentExpansion {
    left_atom: String,
    right_atom: Option<String>,
    complete: bool,
}

impl ExpansionBuilder {
    fn add_atom(&mut self, element: &str, num_hydrogens: Option<u8>) -> String {
        self.add_labeled_atom(element, num_hydrogens, None)
    }

    fn add_labeled_atom(
        &mut self,
        element: &str,
        num_hydrogens: Option<u8>,
        label: Option<&str>,
    ) -> String {
        let key = element.to_ascii_lowercase();
        let key = if key.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            key
        } else {
            "x".to_string()
        };
        let next = self.atom_counts.entry(key.clone()).or_insert(0);
        *next += 1;
        let id = format!("{key}{next}");
        let mut atom = json!({
            "id": id,
            "element": element,
        });
        if let Some(num_hydrogens) = num_hydrogens {
            atom["numHydrogens"] = json!(num_hydrogens);
        }
        if let Some(label) = label {
            atom["label"] = json!(label);
        }
        self.atoms.push(atom);
        id
    }

    fn add_bond(&mut self, begin: &str, end: &str, order: u8) {
        self.bonds.push(json!({
            "begin": begin,
            "end": end,
            "order": order,
        }));
    }
}

fn expansion_for_recognition(recognition: &AbbreviationRecognition) -> Value {
    let mut builder = ExpansionBuilder::default();
    let connection_kind = if recognition.kind == "bridge-fragment" {
        "bridge"
    } else {
        "terminal"
    };
    let mut complete = true;
    let attachments = if connection_kind == "bridge" {
        build_bridge_expansion(&mut builder, &recognition.components, &mut complete)
    } else {
        build_terminal_expansion(&mut builder, &recognition.components, &mut complete)
    };
    json!({
        "schema": "chemcore.functionalGroupExpansion.v1",
        "connectionKind": connection_kind,
        "complete": complete,
        "atoms": builder.atoms,
        "bonds": builder.bonds,
        "attachments": attachments,
    })
}

fn build_terminal_expansion(
    builder: &mut ExpansionBuilder,
    components: &[AbbreviationComponent],
    complete: &mut bool,
) -> Vec<Value> {
    let mut first_atom = None;
    let mut previous_right = None;
    for component in components {
        let fragment = expand_component(builder, component);
        *complete &= fragment.complete;
        if first_atom.is_none() {
            first_atom = Some(fragment.left_atom.clone());
        }
        if let Some(previous) = previous_right.as_deref() {
            builder.add_bond(previous, &fragment.left_atom, 1);
        }
        previous_right = fragment
            .right_atom
            .clone()
            .or_else(|| Some(fragment.left_atom.clone()));
    }
    first_atom
        .map(|atom_id| vec![json!({ "role": "external", "atomId": atom_id })])
        .unwrap_or_default()
}

fn build_bridge_expansion(
    builder: &mut ExpansionBuilder,
    components: &[AbbreviationComponent],
    complete: &mut bool,
) -> Vec<Value> {
    if components.len() == 2
        && components
            .first()
            .is_some_and(|component| component.label == "N")
    {
        let nitrogen = builder.add_atom("N", None);
        let substituent = expand_component(builder, &components[1]);
        *complete &= substituent.complete;
        builder.add_bond(&nitrogen, &substituent.left_atom, 1);
        return vec![
            json!({ "role": "left", "atomId": nitrogen }),
            json!({ "role": "right", "atomId": nitrogen }),
        ];
    }
    let Some(component) = components.first() else {
        return Vec::new();
    };
    let fragment = expand_component(builder, component);
    *complete &= fragment.complete;
    vec![
        json!({ "role": "left", "atomId": fragment.left_atom }),
        json!({ "role": "right", "atomId": fragment.right_atom.unwrap_or(fragment.left_atom) }),
    ]
}

fn expand_component(
    builder: &mut ExpansionBuilder,
    component: &AbbreviationComponent,
) -> FragmentExpansion {
    match component.label.as_str() {
        "CO2" => expand_co2(builder),
        "OCO" => expand_oco(builder),
        "SO2" => expand_sulfur_oxide_linker(builder, 2),
        "SO" => expand_sulfur_oxide_linker(builder, 1),
        "CH2" => single_atom_fragment(builder, "C", Some(2), Some("C"), true),
        "NH" => single_atom_fragment(builder, "N", Some(1), Some("N"), true),
        "CO" => expand_carbonyl_linker(builder),
        "O" => single_atom_fragment(builder, "O", None, Some("O"), true),
        "Me" => expand_alkyl_chain(builder, 1),
        "Et" => expand_alkyl_chain(builder, 2),
        "Pr" => expand_alkyl_chain(builder, 3),
        "nPr" => expand_alkyl_chain(builder, 3),
        "Bu" => expand_alkyl_chain(builder, 4),
        "nBu" => expand_alkyl_chain(builder, 4),
        "iPr" => expand_isopropyl(builder),
        "iBu" => expand_isobutyl(builder),
        "sBu" => expand_sec_butyl(builder),
        "tBu" => expand_tert_butyl(builder),
        "Ph" => expand_phenyl(builder),
        "Bn" => expand_benzyl(builder),
        "Bz" => expand_benzoyl(builder),
        "Ac" => expand_acetyl(builder),
        "Piv" => expand_pivaloyl(builder),
        "CHO" => expand_formyl(builder),
        "CN" => expand_cyano(builder),
        "NCO" => expand_linear_three_atom(builder, "N", "C", "O", 2, 2),
        "NCS" => expand_linear_three_atom(builder, "N", "C", "S", 2, 2),
        "SCN" => expand_linear_three_atom(builder, "S", "C", "N", 1, 3),
        "NO2" => expand_nitro(builder),
        "N3" => expand_linear_three_atom(builder, "N", "N", "N", 1, 3),
        "H" | "F" | "Cl" | "Br" | "I" => {
            single_atom_fragment(builder, &component.label, None, None, true)
        }
        "OH" => single_atom_fragment(builder, "O", Some(1), None, true),
        "NH2" => single_atom_fragment(builder, "N", Some(2), None, true),
        "Ts" => expand_aryl_sulfonyl(builder, Some("Me"), None),
        "Bs" => expand_aryl_sulfonyl(builder, None, Some("Br")),
        "Ms" => expand_methanesulfonyl(builder),
        "Tf" => expand_triflyl(builder),
        "SO3H" => expand_sulfonic_acid(builder),
        "SO2H" => expand_sulfinic_acid(builder),
        "SO3" => expand_sulfur_oxo_terminal(builder, 3),
        "PO3H2" => expand_phosphonic_acid(builder),
        "Boc" => expand_boc(builder),
        "Cbz" => expand_cbz(builder),
        "Fmoc" => expand_fmoc(builder),
        "TMS" => expand_silyl(builder, 3, 0, 0),
        "TBDMS" => expand_silyl(builder, 2, 0, 1),
        "TBDPS" => expand_silyl(builder, 0, 2, 1),
        "CCl3" => expand_trihalomethyl(builder, "Cl"),
        "CF3" => expand_trihalomethyl(builder, "F"),
        "CPh3" => expand_triphenylmethyl(builder),
        "Cy" => expand_cyclohexyl(builder),
        "Mes" => expand_mesityl(builder),
        "NHPh" => expand_anilino(builder),
        _ => expand_opaque_component(builder, component),
    }
}

fn single_atom_fragment(
    builder: &mut ExpansionBuilder,
    element: &str,
    num_hydrogens: Option<u8>,
    right_element: Option<&str>,
    complete: bool,
) -> FragmentExpansion {
    let atom = builder.add_atom(element, num_hydrogens);
    FragmentExpansion {
        left_atom: atom.clone(),
        right_atom: right_element.map(|_| atom),
        complete,
    }
}

fn expand_opaque_component(
    builder: &mut ExpansionBuilder,
    component: &AbbreviationComponent,
) -> FragmentExpansion {
    let element = if component
        .left_anchor
        .chars()
        .all(|ch| ch.is_ascii_alphabetic())
    {
        component.left_anchor.as_str()
    } else {
        "*"
    };
    let atom = builder.add_labeled_atom(element, None, Some(&component.label));
    FragmentExpansion {
        left_atom: atom,
        right_atom: None,
        complete: false,
    }
}

fn expand_alkyl_chain(builder: &mut ExpansionBuilder, len: usize) -> FragmentExpansion {
    let mut atoms = Vec::new();
    for index in 0..len {
        let hydrogens = if len == 1 {
            3
        } else if index + 1 == len {
            3
        } else {
            2
        };
        atoms.push(builder.add_atom("C", Some(hydrogens)));
    }
    for pair in atoms.windows(2) {
        builder.add_bond(&pair[0], &pair[1], 1);
    }
    FragmentExpansion {
        left_atom: atoms[0].clone(),
        right_atom: None,
        complete: true,
    }
}

fn expand_isopropyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let center = builder.add_atom("C", Some(1));
    for _ in 0..2 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: center,
        right_atom: None,
        complete: true,
    }
}

fn expand_isobutyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let left = builder.add_atom("C", Some(2));
    let center = builder.add_atom("C", Some(1));
    builder.add_bond(&left, &center, 1);
    for _ in 0..2 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: left,
        right_atom: None,
        complete: true,
    }
}

fn expand_sec_butyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let left = builder.add_atom("C", Some(1));
    let methyl = builder.add_atom("C", Some(3));
    let methylene = builder.add_atom("C", Some(2));
    let terminal = builder.add_atom("C", Some(3));
    builder.add_bond(&left, &methyl, 1);
    builder.add_bond(&left, &methylene, 1);
    builder.add_bond(&methylene, &terminal, 1);
    FragmentExpansion {
        left_atom: left,
        right_atom: None,
        complete: true,
    }
}

fn expand_tert_butyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let center = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&center, &methyl, 1);
    }
    FragmentExpansion {
        left_atom: center,
        right_atom: None,
        complete: true,
    }
}

fn expand_phenyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    expand_phenyl_ring(builder).0
}

fn expand_phenyl_ring(builder: &mut ExpansionBuilder) -> (FragmentExpansion, Vec<String>) {
    let mut ring = Vec::new();
    for index in 0..6 {
        ring.push(builder.add_atom("C", Some(if index == 0 { 0 } else { 1 })));
    }
    for index in 0..6 {
        let next = (index + 1) % 6;
        builder.add_bond(
            &ring[index],
            &ring[next],
            if index % 2 == 0 { 2 } else { 1 },
        );
    }
    (
        FragmentExpansion {
            left_atom: ring[0].clone(),
            right_atom: None,
            complete: true,
        },
        ring,
    )
}

fn expand_cyclohexyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let mut ring = Vec::new();
    for index in 0..6 {
        ring.push(builder.add_atom("C", Some(if index == 0 { 1 } else { 2 })));
    }
    for index in 0..6 {
        builder.add_bond(&ring[index], &ring[(index + 1) % 6], 1);
    }
    FragmentExpansion {
        left_atom: ring[0].clone(),
        right_atom: None,
        complete: true,
    }
}

fn expand_carbonyl_linker(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxygen, 2);
    FragmentExpansion {
        left_atom: carbon.clone(),
        right_atom: Some(carbon),
        complete: true,
    }
}

fn expand_co2(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: Some(oxygen),
        complete: true,
    }
}

fn expand_oco(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let oxygen = builder.add_atom("O", Some(0));
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    builder.add_bond(&oxygen, &carbon, 1);
    builder.add_bond(&carbon, &oxo, 2);
    FragmentExpansion {
        left_atom: oxygen,
        right_atom: Some(carbon),
        complete: true,
    }
}

fn expand_sulfur_oxide_linker(
    builder: &mut ExpansionBuilder,
    oxo_count: usize,
) -> FragmentExpansion {
    let sulfur = builder.add_atom("S", Some(0));
    for _ in 0..oxo_count {
        let oxygen = builder.add_atom("O", Some(0));
        builder.add_bond(&sulfur, &oxygen, 2);
    }
    FragmentExpansion {
        left_atom: sulfur.clone(),
        right_atom: Some(sulfur),
        complete: true,
    }
}

fn expand_sulfur_oxo_terminal(
    builder: &mut ExpansionBuilder,
    oxo_count: usize,
) -> FragmentExpansion {
    let sulfur = builder.add_atom("S", Some(0));
    for _ in 0..oxo_count {
        let oxygen = builder.add_atom("O", Some(0));
        builder.add_bond(&sulfur, &oxygen, 2);
    }
    FragmentExpansion {
        left_atom: sulfur,
        right_atom: None,
        complete: true,
    }
}

fn expand_acetyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methyl = builder.add_atom("C", Some(3));
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &methyl, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_benzoyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_pivaloyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let tert_butyl = expand_tert_butyl(builder);
    builder.add_bond(&carbon, &oxygen, 2);
    builder.add_bond(&carbon, &tert_butyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_formyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(1));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&carbon, &oxygen, 2);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_cyano(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let nitrogen = builder.add_atom("N", Some(0));
    builder.add_bond(&carbon, &nitrogen, 3);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_linear_three_atom(
    builder: &mut ExpansionBuilder,
    first_element: &str,
    second_element: &str,
    third_element: &str,
    first_order: u8,
    second_order: u8,
) -> FragmentExpansion {
    let first = builder.add_atom(first_element, Some(0));
    let second = builder.add_atom(second_element, Some(0));
    let third = builder.add_atom(third_element, Some(0));
    builder.add_bond(&first, &second, first_order);
    builder.add_bond(&second, &third, second_order);
    FragmentExpansion {
        left_atom: first,
        right_atom: None,
        complete: true,
    }
}

fn expand_nitro(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let nitrogen = builder.add_atom("N", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    builder.add_bond(&nitrogen, &oxo, 2);
    builder.add_bond(&nitrogen, &oxygen, 1);
    FragmentExpansion {
        left_atom: nitrogen,
        right_atom: None,
        complete: true,
    }
}

fn expand_benzyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(2));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_methanesulfonyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let methyl = builder.add_atom("C", Some(3));
    builder.add_bond(&sulfur.left_atom, &methyl, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_triflyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let cf3 = expand_trihalomethyl(builder, "F");
    builder.add_bond(&sulfur.left_atom, &cf3.left_atom, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_aryl_sulfonyl(
    builder: &mut ExpansionBuilder,
    para_methyl: Option<&str>,
    para_halogen: Option<&str>,
) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let (aryl, ring) = expand_phenyl_ring(builder);
    builder.add_bond(&sulfur.left_atom, &aryl.left_atom, 1);
    if para_methyl.is_some() {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&ring[3], &methyl, 1);
    }
    if let Some(halogen) = para_halogen {
        let atom = builder.add_atom(halogen, Some(0));
        builder.add_bond(&ring[3], &atom, 1);
    }
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_sulfonic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 2);
    let oxygen = builder.add_atom("O", Some(1));
    builder.add_bond(&sulfur.left_atom, &oxygen, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_sulfinic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let sulfur = expand_sulfur_oxide_linker(builder, 1);
    let oxygen = builder.add_atom("O", Some(1));
    builder.add_bond(&sulfur.left_atom, &oxygen, 1);
    FragmentExpansion {
        left_atom: sulfur.left_atom,
        right_atom: None,
        complete: true,
    }
}

fn expand_phosphonic_acid(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let phosphorus = builder.add_atom("P", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    builder.add_bond(&phosphorus, &oxo, 2);
    for _ in 0..2 {
        let oxygen = builder.add_atom("O", Some(1));
        builder.add_bond(&phosphorus, &oxygen, 1);
    }
    FragmentExpansion {
        left_atom: phosphorus,
        right_atom: None,
        complete: true,
    }
}

fn expand_boc(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let tert_butyl = expand_tert_butyl(builder);
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &tert_butyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_cbz(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methylene = builder.add_atom("C", Some(2));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &methylene, 1);
    builder.add_bond(&methylene, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_fmoc(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    let oxo = builder.add_atom("O", Some(0));
    let oxygen = builder.add_atom("O", Some(0));
    let methylene = builder.add_atom("C", Some(2));
    let fluorenyl = builder.add_labeled_atom("C", None, Some("fluorenyl"));
    builder.add_bond(&carbon, &oxo, 2);
    builder.add_bond(&carbon, &oxygen, 1);
    builder.add_bond(&oxygen, &methylene, 1);
    builder.add_bond(&methylene, &fluorenyl, 1);
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: false,
    }
}

fn expand_silyl(
    builder: &mut ExpansionBuilder,
    methyl_count: usize,
    phenyl_count: usize,
    tert_butyl_count: usize,
) -> FragmentExpansion {
    let silicon = builder.add_atom("Si", Some(0));
    for _ in 0..methyl_count {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(&silicon, &methyl, 1);
    }
    for _ in 0..phenyl_count {
        let phenyl = expand_phenyl(builder);
        builder.add_bond(&silicon, &phenyl.left_atom, 1);
    }
    for _ in 0..tert_butyl_count {
        let tert_butyl = expand_tert_butyl(builder);
        builder.add_bond(&silicon, &tert_butyl.left_atom, 1);
    }
    FragmentExpansion {
        left_atom: silicon,
        right_atom: None,
        complete: true,
    }
}

fn expand_trihalomethyl(builder: &mut ExpansionBuilder, halogen: &str) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let atom = builder.add_atom(halogen, Some(0));
        builder.add_bond(&carbon, &atom, 1);
    }
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_triphenylmethyl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let carbon = builder.add_atom("C", Some(0));
    for _ in 0..3 {
        let phenyl = expand_phenyl(builder);
        builder.add_bond(&carbon, &phenyl.left_atom, 1);
    }
    FragmentExpansion {
        left_atom: carbon,
        right_atom: None,
        complete: true,
    }
}

fn expand_mesityl(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let (phenyl, ring) = expand_phenyl_ring(builder);
    for atom_id in [&ring[1], &ring[3], &ring[5]] {
        let methyl = builder.add_atom("C", Some(3));
        builder.add_bond(atom_id, &methyl, 1);
    }
    phenyl
}

fn expand_anilino(builder: &mut ExpansionBuilder) -> FragmentExpansion {
    let nitrogen = builder.add_atom("N", Some(1));
    let phenyl = expand_phenyl(builder);
    builder.add_bond(&nitrogen, &phenyl.left_atom, 1);
    FragmentExpansion {
        left_atom: nitrogen,
        right_atom: None,
        complete: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(recognition: &AbbreviationRecognition) -> Vec<&str> {
        recognition
            .components
            .iter()
            .map(|component| component.label.as_str())
            .collect()
    }

    #[test]
    fn parses_terminal_abbreviations() {
        let recognition = recognize_abbreviation_label("Boc").unwrap();
        assert_eq!(recognition.kind, "terminal-fragment");
        assert_eq!(recognition.canonical_label, "Boc");
        assert_eq!(labels(&recognition), vec!["Boc"]);

        let recognition = recognize_abbreviation_label("FMOC").unwrap();
        assert_eq!(recognition.canonical_label, "Fmoc");

        let azide = recognize_abbreviation_label("N3").unwrap();
        assert_eq!(azide.canonical_label, "N3");
        assert_eq!(azide.components[0].name, "azido");

        let tert_butyl = recognize_abbreviation_label("t-Bu").unwrap();
        assert_eq!(tert_butyl.canonical_label, "tBu");
        assert_eq!(labels(&tert_butyl), vec!["tBu"]);
        assert!(recognized_abbreviation_uses_whole_label_layout("t-Bu", 1));

        let normal_butyl = recognize_abbreviation_label("n-Bu").unwrap();
        assert_eq!(normal_butyl.canonical_label, "nBu");
        assert!(recognized_abbreviation_uses_whole_label_layout("nBu", 1));
        assert!(recognized_abbreviation_uses_whole_label_layout("iPr", 1));
        assert!(!recognized_abbreviation_uses_whole_label_layout("CF3", 1));
    }

    #[test]
    fn parses_composite_abbreviations() {
        let co2et = recognize_abbreviation_label("CO2Et").unwrap();
        assert_eq!(labels(&co2et), vec!["CO2", "Et"]);
        assert_eq!(co2et.formula, "-C(=O)OCH2CH3");
        assert_eq!(
            labels(&recognize_abbreviation_label("COOCH2CH2CH3").unwrap()),
            vec!["CO2", "CH2", "CH2", "Me"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("COOSO2Me").unwrap()),
            vec!["CO2", "SO2", "Me"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("CO2Boc").unwrap()),
            vec!["CO2", "Boc"]
        );
        assert_eq!(
            labels(&recognize_abbreviation_label("NHTs").unwrap()),
            vec!["NH", "Ts"]
        );
    }

    #[test]
    fn rejects_open_fragments_without_terminal_fragment() {
        assert!(recognize_abbreviation_label("CO").is_none());
        assert!(recognize_abbreviation_label("CO2").is_none());
        assert!(recognize_abbreviation_label("COO").is_none());
        assert!(recognize_abbreviation_label("SO2").is_none());
        assert!(recognize_abbreviation_label("SO").is_none());
        assert!(recognize_abbreviation_label("CH2").is_none());
        assert!(recognize_abbreviation_label("NH").is_none());
    }

    #[test]
    fn parses_two_connection_bridge_abbreviations() {
        let so2 = recognize_abbreviation_label_for_connection_count("SO2", 2).unwrap();
        assert_eq!(so2.kind, "bridge-fragment");
        assert_eq!(labels(&so2), vec!["SO2"]);

        let so = recognize_abbreviation_label_for_connection_count("SO", 2).unwrap();
        assert_eq!(so.kind, "bridge-fragment");
        assert_eq!(so.formula, "-S(=O)-");
        assert_eq!(labels(&so), vec!["SO"]);

        let nh = recognize_abbreviation_label_for_connection_count("NH", 2).unwrap();
        assert_eq!(nh.kind, "bridge-fragment");
        assert_eq!(labels(&nh), vec!["NH"]);

        let ntos = recognize_abbreviation_label_for_connection_count("NTos", 2).unwrap();
        assert_eq!(ntos.canonical_label, "NTs");
        assert_eq!(labels(&ntos), vec!["N", "Ts"]);

        let ncl = recognize_abbreviation_label_for_connection_count("NCl", 2).unwrap();
        assert_eq!(labels(&ncl), vec!["N", "Cl"]);
    }

    #[test]
    fn rejects_terminal_abbreviations_in_two_connection_context() {
        assert!(recognize_abbreviation_label_for_connection_count("Boc", 2).is_none());
        assert!(recognize_abbreviation_label_for_connection_count("CO2Et", 2).is_none());
    }
}
