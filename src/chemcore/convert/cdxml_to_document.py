from __future__ import annotations

from pathlib import Path
from typing import Any
import json
import re
import xml.etree.ElementTree as ET

from chemcore.cdxml.cdxml_fragment_display import extract_display_fragments
from chemcore.cdxml.cdxml_layout import (
    _normalize_text_block,
    attach_texts_near_molecules,
    extract_line_graphics,
    extract_text_blocks_non_substituent,
    split_tables_by_entry,
)


DEFAULT_STYLES: dict[str, dict[str, Any]] = {
    "style_molecule_default": {
        "kind": "molecule",
        "stroke": "#111111",
        "strokeWidth": 1.2,
        "fontFamily": "Helvetica",
        "fontSize": 11,
    },
    "style_text_default": {
        "kind": "text",
        "fontFamily": "Helvetica",
        "fontSize": 12,
        "fontWeight": 400,
        "fill": "#111111",
        "stroke": None,
    },
    "style_line_default": {
        "kind": "stroke",
        "stroke": "#222222",
        "strokeWidth": 0.72,
        "lineCap": "round",
        "lineJoin": "round",
    },
    "style_arrow_default": {
        "kind": "stroke",
        "stroke": "#222222",
        "strokeWidth": 0.72,
        "lineCap": "butt",
        "lineJoin": "miter",
    },
    "style_shape_default": {
        "kind": "shape",
        "fill": None,
        "stroke": "#222222",
        "strokeWidth": 1.0,
    },
}

ELEMENT_SYMBOLS = [
    None,
    "H", "He",
    "Li", "Be", "B", "C", "N", "O", "F", "Ne",
    "Na", "Mg", "Al", "Si", "P", "S", "Cl", "Ar",
    "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
    "Ga", "Ge", "As", "Se", "Br", "Kr",
    "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe",
    "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd", "Tb", "Dy",
    "Ho", "Er", "Tm", "Yb", "Lu",
    "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn",
    "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf",
    "Es", "Fm", "Md", "No", "Lr",
    "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn",
    "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
]


def _hex_to_rgb(color: str | None) -> tuple[int, int, int] | None:
    if not color or not isinstance(color, str):
        return None
    color = color.strip()
    if not re.fullmatch(r"#[0-9a-fA-F]{6}", color):
        return None
    return (int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16))


def _rgb_to_hex(rgb: tuple[int, int, int]) -> str:
    return "#" + "".join(f"{max(0, min(255, c)):02x}" for c in rgb)


def _mix_colors(color: str, target: tuple[int, int, int], ratio: float) -> str:
    rgb = _hex_to_rgb(color)
    if rgb is None:
        return color
    out = tuple(round(c + (t - c) * ratio) for c, t in zip(rgb, target))
    return _rgb_to_hex(out)


def _build_shaded_gradient(fill: str | None) -> dict[str, Any] | None:
    if not fill:
        return None
    return {
        "type": "linear",
        "x1": "0%",
        "y1": "0%",
        "x2": "0%",
        "y2": "100%",
        "stops": [
            {"offset": "0%", "color": _mix_colors(fill, (255, 255, 255), 0.45)},
            {"offset": "18%", "color": _mix_colors(fill, (255, 255, 255), 0.22)},
            {"offset": "55%", "color": fill},
            {"offset": "100%", "color": _mix_colors(fill, (0, 0, 0), 0.12)},
        ],
    }


def _compact_json(value: Any) -> Any:
    if isinstance(value, dict):
        compacted = {key: _compact_json(item) for key, item in value.items()}
        return {key: item for key, item in compacted.items() if item is not None and item != {} and item != []}
    if isinstance(value, list):
        return [_compact_json(item) for item in value]
    return value


def _script_for_face(face: Any) -> str:
    value = int(face or 0)
    if (value & 32) and not (value & 64):
        return "subscript"
    if (value & 64) and not (value & 32):
        return "superscript"
    return "normal"


def _normalized_display_run(run: dict[str, Any]) -> dict[str, Any]:
    face = int(run.get("face") or 0)
    return {
        "text": str(run.get("text") or ""),
        "fontFamily": run.get("fontFamily") or "Arial",
        "fontSize": round(float(run.get("fontSize") or run.get("size") or 10.0), 2),
        "fill": run.get("fill") or "#111111",
        "fontWeight": 700 if (face & 1) else 400,
        "fontStyle": "italic" if (face & 2) else "normal",
        "script": _script_for_face(face),
    }


def _normalized_display_runs(runs: list[dict[str, Any]] | None) -> list[dict[str, Any]]:
    return [_normalized_display_run(run) for run in (runs or [])]


def _atomic_number_for_element(value: Any) -> int:
    try:
        atomic_number = int(value or 6)
    except Exception:
        atomic_number = 6
    if atomic_number <= 0 or atomic_number >= len(ELEMENT_SYMBOLS):
        return 6
    return atomic_number


def _element_symbol_for_atomic_number(atomic_number: int) -> str:
    return ELEMENT_SYMBOLS[atomic_number] or "C"


def _base_name(cdxml_base: str) -> str:
    return Path(cdxml_base).name


def _normalize_display_text(text: str) -> str:
    text = _normalize_text_block(text)
    if not text:
        return ""

    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if not lines:
        return ""

    merged: list[str] = [lines[0]]
    for line in lines[1:]:
        prev = merged[-1]
        if prev.count("(") > prev.count(")"):
            merged[-1] = prev + line
        elif line[:1] in {",", ".", ";", ":", "%", ")"}:
            merged[-1] = prev + line
        elif " " not in prev and " " not in line and len(prev) <= 12 and len(line) <= 12:
            merged[-1] = prev + line
        elif prev.endswith(","):
            merged[-1] = prev + " " + line
        else:
            merged.append(line)

    text = "\n".join(merged)
    text = re.sub(r"\bN\s+2\b", "N2", text)
    text = re.sub(r"\bH\s+2\s*O\b", "H2O", text)
    text = re.sub(r"\bLi\s+3\b", "Li3", text)
    text = re.sub(r"\bPO\s+4\b", "PO4", text)
    return text


def _localname(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _hex_from_rgb01(r: float, g: float, b: float) -> str:
    return f"#{round(r * 255):02x}{round(g * 255):02x}{round(b * 255):02x}"


def _load_cdxml_tables(cdxml_base: str) -> tuple[dict[str, str], dict[str, str]]:
    root = ET.parse(f"{cdxml_base}.cdxml").getroot()
    colors: dict[str, str] = {"0": "#000000"}
    fonts: dict[str, str] = {"3": "Arial"}

    color_table = next((el for el in root if _localname(el.tag) == "colortable"), None)
    if color_table is not None:
        for idx, color_el in enumerate([el for el in list(color_table) if _localname(el.tag) == "color"], start=1):
            try:
                colors[str(idx)] = _hex_from_rgb01(
                    float(color_el.attrib.get("r", "0")),
                    float(color_el.attrib.get("g", "0")),
                    float(color_el.attrib.get("b", "0")),
                )
            except Exception:
                continue

    colors.update(
        {
            "0": "#000000",
            "3": "#000000",
            "4": "#d61f1f",
            "5": "#fff24a",
            "7": "#55f0f5",
            "8": "#1b32d8",
            "10": "#cfcfcf",
        }
    )

    font_table = next((el for el in root if _localname(el.tag) == "fonttable"), None)
    if font_table is not None:
        for font_el in [el for el in list(font_table) if _localname(el.tag) == "font"]:
            font_id = font_el.attrib.get("id")
            name = font_el.attrib.get("name")
            if font_id and name:
                fonts[font_id] = name

    return colors, fonts


def _parse_bbox(bb: str | None) -> tuple[float, float, float, float] | None:
    if not bb:
        return None
    parts = bb.strip().split()
    if len(parts) < 4:
        return None
    try:
        x1, y1, x2, y2 = (float(v) for v in parts[:4])
        return min(x1, x2), min(y1, y2), max(x1, x2), max(y1, y2)
    except Exception:
        return None


def _extract_shape_graphics(cdxml_base: str, color_table: dict[str, str]) -> list[dict[str, Any]]:
    root = ET.parse(f"{cdxml_base}.cdxml").getroot()
    shapes: list[dict[str, Any]] = []
    for el in root.iter():
        if _localname(el.tag) != "graphic":
            continue
        if "SupersededBy" in el.attrib:
            continue
        if el.attrib.get("GraphicType") != "Rectangle":
            continue
        bbox = _parse_bbox(el.attrib.get("BoundingBox"))
        if bbox is None:
            continue
        rect_type = el.attrib.get("RectangleType", "")
        stroke = color_table.get(el.attrib.get("color", "0"), "#000000")
        fill = None
        dash = None
        stroke_width = 1.0
        shaded = False
        if "Shaded" in rect_type:
            fill = stroke
            stroke = None
            shaded = True
        if "Dashed" in rect_type:
            stroke_width = 0.7
            dash = [3.2, 2.8]
        shapes.append(
            {
                "id": el.attrib.get("id"),
                "bbox": bbox,
                "shapeKind": "roundRect" if "RoundEdge" in rect_type else "rect",
                "cornerRadius": float(el.attrib.get("CornerRadius", "0") or 0) / 100.0,
                "fill": fill,
                "stroke": stroke,
                "strokeWidth": stroke_width,
                "shaded": shaded,
                "dash": dash,
                "z": int(el.attrib.get("Z", "0") or 0),
            }
        )
    return shapes


def _style_id_for_text_run(
    runs: list[dict[str, Any]],
    colors: dict[str, str],
    fonts: dict[str, str],
    styles: dict[str, dict[str, Any]],
    style_cache: dict[tuple[Any, ...], str],
) -> str:
    if not runs:
        return "style_text_default"
    first = runs[0]
    face = int(first.get("face") or 0)
    key = (
        fonts.get(str(first.get("font") or "3"), "Arial"),
        round(float(first.get("size") or 10.0), 2),
        colors.get(str(first.get("color") or "0"), "#111111"),
        700 if (face & 1) else 400,
        "italic" if (face & 2) else "normal",
        "none",
    )
    if key in style_cache:
        return style_cache[key]
    style_id = f"style_text_{len(style_cache) + 1:03d}"
    styles[style_id] = {
        "kind": "text",
        "fontFamily": key[0],
        "fontSize": key[1],
        "fill": key[2],
        "fontWeight": key[3],
        "fontStyle": key[4],
        "textDecoration": key[5],
        "stroke": None,
    }
    style_cache[key] = style_id
    return style_id


def _should_subscript_digit(text: str, index: int) -> bool:
    if not text[index].isdigit():
        return False
    if index == 0:
        return False
    prev = text[index - 1]
    if prev.isspace():
        return False
    return prev.isalpha() or prev in ")]}"


def _expand_formula_run(run: dict[str, Any]) -> list[dict[str, Any]]:
    text = str(run.get("text", ""))
    face = int(run.get("face") or 0)
    if not text or face in {32, 64}:
        return [run]
    style_face = face & 0b11

    def is_subscript_face(value: int) -> bool:
        return (value & 32) != 0 and (value & 64) == 0

    parts: list[dict[str, Any]] = []
    buffer: list[str] = []
    current_face = face
    i = 0
    while i < len(text):
        ch = text[i]
        next_face = face
        if ch.isdigit() and _should_subscript_digit(text, i):
            next_face = style_face | 32
        if next_face != current_face and buffer:
            parts.append({**run, "text": "".join(buffer), "face": current_face})
            buffer = []
        current_face = next_face
        buffer.append(ch)
        i += 1
        while i < len(text) and text[i].isdigit() and is_subscript_face(current_face):
            buffer.append(text[i])
            i += 1
        if i < len(text) and buffer and is_subscript_face(current_face) and not text[i].isdigit():
            parts.append({**run, "text": "".join(buffer), "face": current_face})
            buffer = []
            current_face = face

    if buffer:
        parts.append({**run, "text": "".join(buffer), "face": current_face})
    return parts


def _expand_display_runs(runs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    expanded: list[dict[str, Any]] = []
    for run in runs:
        normalized_run = dict(run)
        if normalized_run.get("fontFamily") == "Symbol" and re.fullmatch(r"\s*\d+\s*", str(normalized_run.get("text", ""))):
            normalized_run["fontFamily"] = "Arial"
        expanded.extend(_expand_formula_run(normalized_run))
    return _inherit_script_run_style_faces(expanded)


def _normalize_molecule_run_face(face: Any) -> int:
    value = int(face or 0)
    style_face = value & 0b11
    script_face = value & (32 | 64)
    if script_face in {32, 64}:
        return style_face | script_face
    return style_face


def _inherit_script_run_style_faces(runs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    normalized = [dict(run) for run in runs]

    def style_bits(face: Any) -> int:
        return int(face or 0) & 0b11

    def is_script_only_face(face: Any) -> bool:
        value = int(face or 0)
        return (value & (32 | 64)) in {32, 64} and (value & 0b11) == 0

    for index, run in enumerate(normalized):
        face = int(run.get("face") or 0)
        if not is_script_only_face(face):
            continue

        inherited_style = 0
        for neighbor_index in range(index - 1, -1, -1):
            inherited_style = style_bits(normalized[neighbor_index].get("face"))
            if inherited_style:
                break
        if not inherited_style:
            for neighbor_index in range(index + 1, len(normalized)):
                inherited_style = style_bits(normalized[neighbor_index].get("face"))
                if inherited_style:
                    break
        if inherited_style:
            run["face"] = face | inherited_style

    return normalized


def _normalize_molecule_display_runs(runs: list[dict[str, Any]] | None) -> list[dict[str, Any]]:
    normalized: list[dict[str, Any]] = []
    for run in runs or []:
        run_copy = dict(run)
        run_copy["face"] = _normalize_molecule_run_face(run_copy.get("face"))
        normalized.extend(_expand_formula_run(run_copy))
    return _inherit_script_run_style_faces(normalized)


def _resolve_fragment_label_styles(
    fragments: list[dict[str, Any]],
    colors: dict[str, str],
    fonts: dict[str, str],
) -> None:
    for fragment in fragments:
        for node in fragment.get("nodes", []):
            label = node.get("label")
            if not label:
                continue
            label["fill"] = colors.get(str(label.get("color") or "0"), "#111111")
            label["fontFamily"] = fonts.get(str(label.get("font") or "3"), "Arial")
            for run in label.get("runs") or []:
                run["fill"] = colors.get(str(run.get("color") or label.get("color") or "0"), "#111111")
                run["fontFamily"] = fonts.get(str(run.get("font") or label.get("font") or "3"), label["fontFamily"])


_FORMULA_TOKEN_RE = re.compile(r"[A-Z][a-z]?\d*")


def _reorient_fragment_formula_labels(fragments: list[dict[str, Any]]) -> None:
    for fragment in fragments:
        for node in fragment.get("nodes", []):
            label = node.get("label")
            if not label:
                continue
            text = str(label.get("text") or "")
            if not text or "-" in text or " " in text:
                continue
            if label.get("align") != "right" or label.get("labelAlignment") != "right":
                continue

            tokens = _FORMULA_TOKEN_RE.findall(text)
            if len(tokens) < 2 or "".join(tokens) != text:
                continue

            reversed_tokens = list(reversed(tokens))
            label["text"] = "".join(reversed_tokens)

            base_face = 0
            if label.get("sourceRuns"):
                base_face = _normalize_molecule_run_face(label["sourceRuns"][0].get("face"))
            elif label.get("runs"):
                base_face = _normalize_molecule_run_face(label["runs"][0].get("face"))

            rebuilt_runs: list[dict[str, Any]] = []
            for token in reversed_tokens:
                match = re.fullmatch(r"([A-Z][a-z]?)(\d*)", token)
                if not match:
                    rebuilt_runs.append({"text": token, "face": base_face})
                    continue
                element, digits = match.groups()
                rebuilt_runs.append({"text": element, "face": base_face})
                if digits:
                    rebuilt_runs.append({"text": digits, "face": base_face | 32})

            if label.get("runs"):
                template = dict(label["runs"][0])
                normalized_runs: list[dict[str, Any]] = []
                for run in rebuilt_runs:
                    merged = dict(template)
                    merged.update(run)
                    normalized_runs.append(merged)
                label["runs"] = normalized_runs


def _normalize_fragment_hydrogen_labels(fragments: list[dict[str, Any]]) -> None:
    for fragment in fragments:
        for node in fragment.get("nodes", []):
            label = node.get("label")
            if not label:
                continue
            if str(node.get("element") or "") != "7" or int(node.get("numHydrogens") or 0) != 1:
                continue
            if str(label.get("text") or "") != "NH":
                continue

            if label.get("labelAlignment") == "above":
                label["lines"] = ["H", "N"]
                continue

            if label.get("labelAlignment") == "right":
                label["text"] = "HN"
                if label.get("runs"):
                    template = dict(label["runs"][0])
                    first = dict(template)
                    first["text"] = "H"
                    second = dict(template)
                    second["text"] = "N"
                    label["runs"] = [first, second]


def _fragment_layout_mode(node: dict[str, Any], label: dict[str, Any] | None) -> str:
    if not label:
        return "default"
    if str(node.get("element") or "") == "7" and int(node.get("numHydrogens") or 0) == 1:
        if label.get("labelAlignment") == "above":
            return "hetero-h-above"
        if label.get("labelAlignment") == "right":
            return "hetero-h-right"
    if node.get("nodeType") in {"Fragment", "Nickname"}:
        if label.get("labelAlignment") == "above":
            return "attached-group-above"
        if node.get("labelDisplay") == "Center" or label.get("align") == "center":
            return "attached-group-center"
        return "attached-group"
    return "default"


def _connection_anchor_for_label(label: dict[str, Any] | None) -> str:
    if not label:
        return "start"
    if label.get("labelAlignment") == "right" or label.get("align") == "right":
        return "end"
    if label.get("align") == "center":
        return "center"
    return "start"


def _local_point(point: list[float] | tuple[float, float] | None, origin: tuple[float, float]) -> list[float] | None:
    if not point or len(point) < 2:
        return None
    return [round(float(point[0]) - origin[0], 2), round(float(point[1]) - origin[1], 2)]


def _local_bbox(bbox: list[float] | tuple[float, float, float, float] | None, origin: tuple[float, float]) -> list[float] | None:
    if not bbox or len(bbox) < 4:
        return None
    return [
        round(float(bbox[0]) - origin[0], 2),
        round(float(bbox[1]) - origin[1], 2),
        round(float(bbox[2]) - origin[0], 2),
        round(float(bbox[3]) - origin[1], 2),
    ]


def _attached_group_above_lines(text: str) -> list[str] | None:
    tokens = re.findall(r"[A-Z][a-z]?|\d+|[a-z]+|.", text or "")
    if len(tokens) < 2:
        return None
    if tokens[0] == "N" and len(tokens) >= 3 and tokens[1] == "H":
        return ["".join(tokens[2:]), "NH"]
    return ["".join(tokens[1:]), tokens[0]]


def _expand_label_runs(runs: list[dict[str, Any]] | None) -> list[dict[str, Any]]:
    expanded: list[dict[str, Any]] = []
    for run in runs or []:
        parts = re.findall(r"[A-Z][a-z]?|\d+|[a-z]+|.", str(run.get("text") or ""))
        for part in parts or [""]:
            run_copy = dict(run)
            run_copy["text"] = part
            face = _normalize_molecule_run_face(run_copy.get("face"))
            if re.fullmatch(r"\d+", part):
                face |= 32
            else:
                face &= ~32
            run_copy["face"] = face
            expanded.append(run_copy)
    return _inherit_script_run_style_faces(expanded)


def _split_attached_group_above_runs(
    text: str,
    runs: list[dict[str, Any]] | None,
) -> list[list[dict[str, Any]]] | None:
    tokens = re.findall(r"[A-Z][a-z]?|\d+|[a-z]+|.", text or "")
    if len(tokens) < 2:
        return None
    expanded = _expand_label_runs(runs)
    if not expanded or len(expanded) != len(tokens):
        return None
    if tokens[0] == "N" and len(tokens) >= 3 and tokens[1] == "H":
        return [
            [dict(run) for run in expanded[2:]],
            [dict(run) for run in expanded[:2]],
        ]
    return [
        [dict(run) for run in expanded[1:]],
        [dict(run) for run in expanded[:1]],
    ]


def _normalize_fragment_label(
    node: dict[str, Any],
    label: dict[str, Any] | None,
    origin: tuple[float, float],
    fragment: dict[str, Any],
) -> dict[str, Any] | None:
    if not label:
        return None
    layout_mode = _fragment_layout_mode(node, label)
    position = label.get("position")
    bbox = label.get("bbox")
    display_runs = _normalize_molecule_display_runs(label.get("runs"))
    input_runs = [dict(run) for run in (label.get("sourceRuns") or [])]
    lines = label.get("lines")
    line_runs = None
    if not lines and layout_mode == "attached-group-above":
        lines = _attached_group_above_lines(label.get("text") or "")
    if layout_mode == "attached-group-above":
        raw_line_runs = (
            _split_attached_group_above_runs(label.get("sourceText") or label.get("text") or "", input_runs or display_runs)
            or _split_attached_group_above_runs(label.get("text") or "", display_runs)
        )
        if raw_line_runs:
            line_runs = [_normalized_display_runs(line) for line in raw_line_runs]
    return _compact_json({
        "text": label.get("text") or "",
        "sourceText": label.get("sourceText") or label.get("text") or "",
        "position": _local_point(position, origin),
        "box": _local_bbox(bbox, origin),
        "runs": _normalized_display_runs(display_runs),
        "align": label.get("align") or "left",
        "layout": layout_mode,
        "attachment": label.get("labelAlignment") or None,
        "anchor": _connection_anchor_for_label(label),
        "lines": list(lines) if lines else None,
        "lineRuns": line_runs,
        "fontFamily": label.get("fontFamily") or "Arial",
        "fill": label.get("fill") or "#111111",
        "fontSize": round(float(label.get("size") or 10.0), 2),
        "meta": {
            "import": {
                "cdxml": {
                    "font": label.get("font"),
                    "color": label.get("color"),
                    "lineStarts": label.get("lineStarts"),
                    "sourceRuns": input_runs or None,
                }
            }
        },
    })


def _normalize_fragment_bond(bond: dict[str, Any]) -> dict[str, Any]:
    display = str(bond.get("display") or "")
    stereo_style = None
    stereo_end = None
    if display == "WedgeBegin":
        stereo_style = "solid-wedge"
        stereo_end = "end"
    elif display == "WedgeEnd":
        stereo_style = "solid-wedge"
        stereo_end = "begin"
    elif display == "WedgedHashBegin":
        stereo_style = "hashed-wedge"
        stereo_end = "end"
    elif display == "WedgedHashEnd":
        stereo_style = "hashed-wedge"
        stereo_end = "begin"

    double_style = str(bond.get("doublePosition") or "").lower() or "center"
    if double_style not in {"left", "right", "center"}:
        double_style = "center"

    stereo = None
    if stereo_style and stereo_end:
        stereo = {
            "kind": stereo_style,
            "wideEnd": stereo_end,
        }

    order = int(bond.get("order") or 1)
    return _compact_json({
        "id": bond.get("id"),
        "begin": bond.get("begin"),
        "end": bond.get("end"),
        "order": order,
        "stereo": stereo,
        "double": {
            "placement": double_style,
        } if order == 2 else None,
        "meta": {
            "import": {
                "cdxml": {
                    "display": bond.get("display"),
                    "doublePosition": bond.get("doublePosition"),
                }
            }
        },
    })


def _normalize_fragment(fragment: dict[str, Any]) -> dict[str, Any]:
    bbox = fragment.get("bbox") or [0.0, 0.0, 0.0, 0.0]
    origin = (float(bbox[0]), float(bbox[1]))
    local_bbox = [0.0, 0.0, round(float(bbox[2]) - origin[0], 2), round(float(bbox[3]) - origin[1], 2)]

    nodes: list[dict[str, Any]] = []
    for node in fragment.get("nodes", []):
        atomic_number = _atomic_number_for_element(node.get("element"))
        normalized = _compact_json({
            "id": node.get("id"),
            "element": _element_symbol_for_atomic_number(atomic_number),
            "atomicNumber": atomic_number,
            "position": _local_point(node.get("position"), origin),
            "charge": int(node.get("charge") or 0),
            "numHydrogens": int(node.get("numHydrogens") or 0),
            "isExternalConnectionPoint": bool(node.get("isExternalConnectionPoint")),
            "isPlaceholder": bool(node.get("isPlaceholder")),
            "label": _normalize_fragment_label(node, node.get("label"), origin, fragment),
            "meta": {
                "import": {
                    "cdxml": {
                        "nodeType": node.get("nodeType"),
                        "labelDisplay": node.get("labelDisplay"),
                        "element": node.get("element"),
                    }
                }
            },
        })
        nodes.append(normalized)

    return {
        "schema": "chemcore.molecule.fragment2d",
        "bbox": local_bbox,
        "nodes": nodes,
        "bonds": [_normalize_fragment_bond(bond) for bond in fragment.get("bonds", [])],
        "meta": {
            "import": {
                "cdxml": {
                    "fragmentId": fragment.get("id"),
                    "center": fragment.get("center"),
                    "bboxAbs": fragment.get("bbox"),
                    "z": fragment.get("z"),
                }
            }
        },
    }


def _page_from_objects(objects: list[dict[str, Any]], margin: float = 24.0) -> dict[str, Any]:
    max_x = 0.0
    max_y = 0.0

    for obj in objects:
        payload = obj.get("payload", {})
        if obj["type"] == "molecule":
            tx, ty = obj["transform"]["translate"]
            _, _, w, h = payload["bbox"]
            max_x = max(max_x, tx + w)
            max_y = max(max_y, ty + h)
        elif obj["type"] == "text":
            tx, ty = obj["transform"]["translate"]
            box = payload.get("box") or [0.0, 0.0, 160.0, 24.0]
            max_x = max(max_x, tx + float(box[2]))
            max_y = max(max_y, ty + float(box[3]))
        elif obj["type"] == "line":
            for x, y in payload.get("points", []):
                max_x = max(max_x, float(x))
                max_y = max(max_y, float(y))
        elif obj["type"] == "shape":
            tx, ty = obj["transform"]["translate"]
            _, _, w, h = payload.get("bbox") or [0.0, 0.0, 0.0, 0.0]
            max_x = max(max_x, tx + float(w))
            max_y = max(max_y, ty + float(h))

    width = max(640.0, max_x + margin)
    height = max(480.0, max_y + margin)
    return {
        "width": round(width, 2),
        "height": round(height, 2),
        "background": "#ffffff",
    }


def _zindex_from_source(source_z: Any, fallback: int) -> int:
    try:
        return int(source_z)
    except Exception:
        return fallback


def convert_cdxml_to_document(cdxml_base: str) -> dict[str, Any]:
    cdxml_base = str(Path(cdxml_base))
    color_table, font_table = _load_cdxml_tables(cdxml_base)

    text_blocks = extract_text_blocks_non_substituent(cdxml_base)
    text_items = [(item["text"], item["point"]) for item in text_blocks]
    form, table_bboxes, remaining_items = split_tables_by_entry(text_items)
    fragments = extract_display_fragments(cdxml_base, table_bboxes=table_bboxes)
    _reorient_fragment_formula_labels(fragments)
    _normalize_fragment_hydrogen_labels(fragments)
    _resolve_fragment_label_styles(fragments, color_table, font_table)
    mol_infos = [
        (
            "",
            (float(fragment["center"][0]), float(fragment["center"][1])),
            (
                float(fragment["bbox"][2] - fragment["bbox"][0]),
                float(fragment["bbox"][3] - fragment["bbox"][1]),
            ),
            None,
        )
        for fragment in fragments
    ]
    mol_match, remaining_texts = attach_texts_near_molecules(mol_infos, remaining_items)
    line_graphics = extract_line_graphics(cdxml_base)
    shape_graphics = _extract_shape_graphics(cdxml_base, color_table)

    molecules: list[dict[str, Any]] = []
    for idx, fragment in enumerate(fragments):
        label = mol_match[idx][3] if idx < len(mol_match) else ""
        molecules.append(
            {
                "center": tuple(fragment["center"]),
                "wh": (
                    float(fragment["bbox"][2] - fragment["bbox"][0]),
                    float(fragment["bbox"][3] - fragment["bbox"][1]),
                ),
                "bbox_abs": tuple(fragment["bbox"]),
                "label": label,
                "fragment": fragment,
            }
        )

    resources: dict[str, dict[str, Any]] = {}
    objects: list[dict[str, Any]] = []
    styles = {key: value.copy() for key, value in DEFAULT_STYLES.items()}
    text_style_cache: dict[tuple[Any, ...], str] = {}
    used_table_block_indices: set[int] = set()

    def _match_table_block(
        cell_text: str,
        table_bbox: tuple[float, float, float, float],
        row_y: float,
        col_index: int,
    ) -> tuple[int | None, dict[str, Any] | None]:
        target = _normalize_display_text(cell_text)
        best_idx = None
        best_block = None
        best_score = None
        for block_index, block in enumerate(text_blocks):
            if block_index in used_table_block_indices:
                continue
            normalized = _normalize_display_text(block["text"])
            if normalized != target:
                continue
            point = block["point"]
            x1, y1, x2, y2 = table_bbox
            if not (x1 - 3.0 <= point[0] <= x2 + 3.0 and y1 - 3.0 <= point[1] <= y2 + 3.0):
                continue
            score = abs(float(point[1]) - float(row_y)) + (0.5 if col_index == 1 else 0.0)
            if best_score is None or score < best_score:
                best_score = score
                best_idx = block_index
                best_block = block
        return best_idx, best_block

    for idx, molecule in enumerate(molecules, start=1):
        resource_id = f"mol_{idx:03d}"
        bbox_abs = molecule["bbox_abs"]
        mol_tx = round(float(bbox_abs[0]), 2)
        mol_ty = round(float(bbox_abs[1]), 2)
        width = round(float(bbox_abs[2] - bbox_abs[0]), 2)
        height = round(float(bbox_abs[3] - bbox_abs[1]), 2)
        resources[resource_id] = {
            "type": "molecule_fragment2d",
            "encoding": "chemcore.molecule.fragment2d",
            "data": _normalize_fragment(molecule["fragment"]),
            "meta": {
                "import": {
                    "cdxml": {
                        "fragmentId": molecule["fragment"].get("id"),
                    }
                }
            },
        }
        objects.append(
            {
                "id": f"obj_mol_{idx:03d}",
                "type": "molecule",
                "name": f"molecule {idx}",
                "visible": True,
                "locked": False,
                "zIndex": _zindex_from_source(molecule["fragment"].get("z"), 10),
                "transform": {
                    "translate": [mol_tx, mol_ty],
                    "rotate": 0,
                    "scale": [1, 1],
                },
                "styleRef": "style_molecule_default",
                "meta": {
                    "source": "cdxml",
                    "label": molecule.get("label") or "",
                    "fragmentId": molecule["fragment"].get("id"),
                },
                "payload": {
                    "resourceRef": resource_id,
                    "bbox": [0, 0, round(width, 2), round(height, 2)],
                    "role": None,
                },
            }
        )
    for idx, line in enumerate(sorted(line_graphics, key=lambda item: (item.get("z") or 0, item.get("id") or "")), start=1):
        tail = line.get("tail")
        head = line.get("head")
        if not tail or not head:
            continue
        is_arrow = bool(line.get("isArrow"))
        objects.append(
            {
                "id": f"obj_line_{idx:03d}",
                "type": "line",
                "name": f"line {idx}",
                "visible": True,
                "locked": False,
                "zIndex": _zindex_from_source(line.get("z"), 20 if is_arrow else 18),
                "transform": {
                    "translate": [0, 0],
                    "rotate": 0,
                    "scale": [1, 1],
                },
                "styleRef": "style_arrow_default" if is_arrow else "style_line_default",
                "meta": {
                    "source": "cdxml",
                    "graphicId": line.get("id"),
                    "import": {
                        "cdxml": {
                            "kind": "arrow" if is_arrow else "line",
                        }
                    },
                },
                "payload": {
                    "kind": "line",
                    "points": [
                        [round(float(tail[0]), 2), round(float(tail[1]), 2)],
                        [round(float(head[0]), 2), round(float(head[1]), 2)],
                    ],
                    "head": "end" if is_arrow else "none",
                    "tail": "none",
                    "arrowHead": {
                        "kind": (line.get("arrowheadType") or "Solid").lower(),
                        "head": (line.get("arrowheadHead") or "Full").lower(),
                        "tail": (line.get("arrowheadTail") or "None").lower(),
                        "length": round(float(line.get("headSize") or 0), 2),
                        "centerLength": round(float(line.get("arrowheadCenterSize") or 0), 2),
                        "width": round(float(line.get("arrowheadWidth") or 0), 2),
                    } if is_arrow else None,
                },
            }
        )

    for idx, shape in enumerate(sorted(shape_graphics, key=lambda item: item["z"]), start=1):
        x1, y1, x2, y2 = shape["bbox"]
        style_id = f"style_shape_{idx:03d}"
        styles[style_id] = {
            "kind": "shape",
            "fill": shape["fill"],
            "stroke": shape["stroke"],
            "strokeWidth": shape.get("strokeWidth", 1.0),
            "fillGradient": _build_shaded_gradient(shape["fill"]) if shape.get("shaded") else None,
            "dashArray": shape["dash"],
        }
        objects.append(
            {
                "id": f"obj_shape_{idx:03d}",
                "type": "shape",
                "name": f"shape {idx}",
                "visible": True,
                "locked": False,
                "zIndex": _zindex_from_source(shape.get("z"), 15),
                "transform": {
                    "translate": [round(x1, 2), round(y1, 2)],
                    "rotate": 0,
                    "scale": [1, 1],
                },
                "styleRef": style_id,
                "meta": {
                    "source": "cdxml",
                    "graphicId": shape["id"],
                },
                "payload": {
                    "kind": shape["shapeKind"],
                    "bbox": [0, 0, round(x2 - x1, 2), round(y2 - y1, 2)],
                    "cornerRadius": round(shape["cornerRadius"], 2),
                },
            }
        )

    for table_index, table in enumerate(form, start=1):
        if not table:
            continue
        bbox = table_bboxes[table_index - 1] if table_index - 1 < len(table_bboxes) else (40.0, 140.0, 440.0, 480.0)
        x0, y0, x1, y1 = bbox
        row_count = len(table)
        col_count = max(len(row) for row in table)
        if row_count == 0 or col_count == 0:
            continue
        row_step = max((y1 - y0) / max(1, row_count - 1), 18.0)
        col_step = (x1 - x0) / max(1, col_count)

        for row_index, row in enumerate(table):
            row_y = y0 + row_index * row_step
            for col_index, cell in enumerate(row):
                cell_text = _normalize_display_text(cell)
                if not cell_text:
                    continue
                cell_lines = cell_text.splitlines()
                col_center = x0 + (col_index + 0.5) * col_step
                box_width = col_step - 10.0
                align = "center"
                if col_index == 1:
                    align = "left"
                block_index, block = _match_table_block(cell_text, bbox, row_y, col_index)
                if block_index is not None:
                    used_table_block_indices.add(block_index)
                if block and block.get("bbox"):
                    bx1, by1, bx2, by2 = block["bbox"]
                    width = max(24.0, float(bx2 - bx1))
                    height = max(14.0, float(by2 - by1))
                    block_align = block.get("align") or align
                    if block_align == "center":
                        tx = (bx1 + bx2) / 2.0
                    elif block_align == "right":
                        tx = bx2
                    else:
                        tx = bx1
                    ty = by1
                else:
                    width = round(box_width, 2)
                    height = max(18.0, 15.0 * len(cell_lines))
                    tx = col_center if align == "center" else x0 + col_index * col_step + 6.0
                    ty = row_y
                    block_align = align
                font_size = float((block or {}).get("fontSize") or 10.0)
                line_height = max(font_size * 1.1, height / max(1, len(cell_lines)))
                source_runs = (block or {}).get("runs") or []
                display_runs = _normalized_display_runs(_expand_display_runs(
                    [
                        {
                            "text": run.get("text", ""),
                            "fontFamily": font_table.get(str(run.get("font") or "3"), "Arial"),
                            "fontSize": float(run.get("size") or font_size),
                            "fill": color_table.get(str(run.get("color") or "0"), "#111111"),
                            "face": int(run.get("face") or 0),
                        }
                        for run in source_runs
                    ]
                ))
                objects.append(
                    {
                        "id": f"obj_text_table_{table_index:02d}_{row_index:03d}_{col_index:02d}",
                        "type": "text",
                        "name": f"table {table_index} r{row_index} c{col_index}",
                        "visible": True,
                        "locked": False,
                        "zIndex": _zindex_from_source((block or {}).get("z"), 25),
                        "transform": {
                            "translate": [round(float(tx), 2), round(float(ty), 2)],
                            "rotate": 0,
                            "scale": [1, 1],
                        },
                        "styleRef": _style_id_for_text_run(source_runs, color_table, font_table, styles, text_style_cache),
                        "meta": {
                            "source": "cdxml",
                            "role": "table_cell",
                            "tableIndex": table_index,
                            "row": row_index,
                            "col": col_index,
                        },
                        "payload": {
                            "text": cell_text,
                            "box": [0, 0, round(width, 2), round(height, 2)],
                            "align": block_align,
                            "valign": "top",
                            "lineHeight": round(line_height, 2),
                            "fontSize": round(font_size, 2),
                            "preserveLines": True,
                            "runs": display_runs,
                        },
                    }
                )

    def _is_inside_any_table_bbox(point: tuple[float, float]) -> bool:
        x, y = point
        for x1, y1, x2, y2 in table_bboxes:
            if x1 - 2.0 <= x <= x2 + 2.0 and y1 - 2.0 <= y <= y2 + 2.0:
                return True
        return False

    display_text_blocks = []
    for block in text_blocks:
        normalized = _normalize_display_text(block["text"])
        if not normalized:
            continue
        if _is_inside_any_table_bbox(block["point"]):
            continue
        display_text_blocks.append((block, normalized))

    for idx, (block, normalized) in enumerate(display_text_blocks, start=1):
        bbox = block.get("bbox")
        line_count = max(1, len(normalized.splitlines()))
        if bbox:
            bx1, by1, bx2, by2 = bbox
            width = max(24.0, float(bx2 - bx1))
            height = max(14.0, float(by2 - by1))
            align = block.get("align") or "left"
            if align == "center":
                tx = (bx1 + bx2) / 2.0
            elif align == "right":
                tx = bx2
            else:
                tx = bx1
            ty = by1
        else:
            tx, ty = block["point"]
            width = max(80.0, min(480.0, float(max(len(line) for line in normalized.splitlines())) * 7.2))
            height = max(18.0, 18.0 * line_count)
            align = block.get("align") or "left"
        font_size = float(block.get("fontSize") or 10.0)
        line_height = max(font_size * 1.1, height / line_count)
        source_runs = block.get("runs") or []
        display_runs = _normalized_display_runs(_expand_display_runs(
            [
                {
                    "text": run.get("text", ""),
                    "fontFamily": font_table.get(str(run.get("font") or "3"), "Arial"),
                    "fontSize": float(run.get("size") or font_size),
                    "fill": color_table.get(str(run.get("color") or "0"), "#111111"),
                    "face": int(run.get("face") or 0),
                }
                for run in source_runs
            ]
        ))
        objects.append(
            {
                "id": f"obj_text_{idx:03d}",
                "type": "text",
                "name": f"text {idx}",
                "visible": True,
                "locked": False,
                "zIndex": _zindex_from_source(block.get("z"), 30),
                "transform": {
                    "translate": [round(float(tx), 2), round(float(ty), 2)],
                    "rotate": 0,
                    "scale": [1, 1],
                },
                "styleRef": _style_id_for_text_run(source_runs, color_table, font_table, styles, text_style_cache),
                "meta": {
                    "source": "cdxml",
                    "role": "free_text",
                    "z": block.get("z", 0),
                },
                "payload": {
                    "text": normalized,
                    "box": [0, 0, round(width, 2), round(height, 2)],
                    "align": align,
                    "valign": "top",
                    "lineHeight": round(line_height, 2),
                    "fontSize": round(font_size, 2),
                    "preserveLines": True,
                    "runs": display_runs,
                },
            }
        )

    document = {
        "format": {
            "name": "chemcore",
            "version": "0.1",
        },
        "document": {
            "id": f"doc_{_base_name(cdxml_base)}",
            "title": f"Imported {_base_name(cdxml_base)}",
            "page": _page_from_objects(objects),
            "meta": {
                "createdBy": "chemcore",
                "sourceFormat": "cdxml",
                "sourceBase": cdxml_base,
            },
        },
        "styles": styles,
        "objects": objects,
        "resources": resources,
    }
    return document


def document_to_jsonable(document: dict[str, Any]) -> str:
    return json.dumps(document, ensure_ascii=False, indent=2)
