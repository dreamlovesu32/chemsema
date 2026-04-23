from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
import xml.etree.ElementTree as ET

from .cdxml_shared import ECP_TYPE, PLACEHOLDER_TYPES, parse_bbox_center, parse_xy


def _localname(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _is_tag(el: ET.Element, name: str) -> bool:
    return _localname(el.tag) == name


def _parse_bbox(bb: Optional[str]) -> Optional[Tuple[float, float, float, float]]:
    if not bb:
        return None
    parts = bb.strip().split()
    if len(parts) < 4:
        return None
    try:
        return tuple(float(v) for v in parts[:4])  # type: ignore[return-value]
    except Exception:
        return None


def _bbox_contains_point(
    bbox: Tuple[float, float, float, float],
    point: Tuple[float, float],
    pad: float = 0.0,
) -> bool:
    x1, y1, x2, y2 = bbox
    x, y = point
    return (x1 - pad) <= x <= (x2 + pad) and (y1 - pad) <= y <= (y2 + pad)


def _extract_node_label(node: ET.Element) -> Optional[dict[str, Any]]:
    text_el = next((ch for ch in list(node) if _is_tag(ch, "t")), None)
    if text_el is None:
        return None

    text = (text_el.attrib.get("UTF8Text") or "".join(text_el.itertext()) or "").strip()
    if not text:
        return None

    pos = parse_xy(text_el.attrib.get("p"))
    bbox = _parse_bbox(text_el.attrib.get("BoundingBox"))
    if pos is None and bbox is not None:
        pos = (bbox[0], bbox[1])
    if pos is None:
        return None

    parent_font = text_el.attrib.get("font", "3")
    parent_color = text_el.attrib.get("color", "0")
    parent_size = float(text_el.attrib.get("size", "10.0") or 10.0)
    runs: List[dict[str, Any]] = []
    for run in list(text_el):
        if not _is_tag(run, "s"):
            continue
        run_text = "".join(run.itertext())
        if not run_text:
            continue
        runs.append(
            {
                "text": run_text,
                "face": int(run.attrib.get("face", "0") or 0),
                "size": float(run.attrib.get("size", str(parent_size)) or parent_size),
                "font": run.attrib.get("font", parent_font),
                "color": run.attrib.get("color", parent_color),
            }
        )

    line_starts_raw = (text_el.attrib.get("LineStarts") or "").strip()
    line_starts: List[int] = []
    if line_starts_raw:
        try:
            line_starts = [int(value) for value in line_starts_raw.split() if value.strip()]
        except Exception:
            line_starts = []

    lines: Optional[List[str]] = None
    if "\n" in text:
        lines = [part for part in text.splitlines() if part]
    elif len(line_starts) > 1 and len(text) == len(line_starts):
        # ChemDraw uses LineStarts for stacked hetero labels such as "NH".
        # Preserve them explicitly so downstream renderers do not need to guess.
        lines = list(text)

    return {
        "text": text,
        "sourceText": text,
        "position": [round(float(pos[0]), 2), round(float(pos[1]), 2)],
        "bbox": [round(v, 2) for v in bbox] if bbox else None,
        "align": (text_el.attrib.get("Justification") or text_el.attrib.get("LabelJustification") or "Left").lower(),
        "labelAlignment": (text_el.attrib.get("LabelAlignment") or "").lower(),
        "font": parent_font,
        "color": parent_color,
        "size": parent_size,
        "lineStarts": line_starts or None,
        "lines": lines,
        "runs": runs or None,
        "sourceRuns": [dict(run) for run in runs] if runs else None,
    }


def extract_display_fragments(
    cdxml_path: str,
    table_bboxes: Optional[List[Tuple[float, float, float, float]]] = None,
) -> List[dict[str, Any]]:
    xml_path = Path(f"{cdxml_path}.cdxml")
    if not (xml_path.exists() and xml_path.is_file()):
        raise FileNotFoundError(str(xml_path))

    root = ET.parse(str(xml_path)).getroot()
    page = next((el for el in root.iter() if _is_tag(el, "page")), None)
    if page is None:
        return []

    fragments: List[dict[str, Any]] = []
    for frag in page.iter():
        if frag is page or not _is_tag(frag, "fragment"):
            continue

        bbox = _parse_bbox(frag.attrib.get("BoundingBox"))
        center = parse_bbox_center(frag.attrib.get("BoundingBox"))
        if bbox is None or center is None:
            continue
        if table_bboxes and any(_bbox_contains_point(tb, center, pad=4.0) for tb in table_bboxes):
            continue

        top_nodes = [ch for ch in list(frag) if _is_tag(ch, "n") and ch.attrib.get("id")]
        top_bonds = [ch for ch in list(frag) if _is_tag(ch, "b") and ch.attrib.get("id")]
        if len(top_nodes) < 2 or not top_bonds:
            continue

        node_ids = {node.attrib["id"] for node in top_nodes}
        nodes: List[dict[str, Any]] = []
        for node in top_nodes:
            node_id = node.attrib["id"]
            pos = parse_xy(node.attrib.get("p"))
            if pos is None:
                continue
            label = _extract_node_label(node)
            node_type = node.attrib.get("NodeType", "")
            nodes.append(
                {
                    "id": node_id,
                    "position": [round(float(pos[0]), 2), round(float(pos[1]), 2)],
                    "element": node.attrib.get("Element"),
                    "nodeType": node_type or None,
                    "charge": int(node.attrib.get("Charge", "0") or 0),
                    "numHydrogens": int(node.attrib.get("NumHydrogens", "0") or 0),
                    "geometry": node.attrib.get("Geometry"),
                    "labelDisplay": node.attrib.get("LabelDisplay"),
                    "isExternalConnectionPoint": node_type == ECP_TYPE,
                    "isPlaceholder": node_type in PLACEHOLDER_TYPES,
                    "label": label,
                }
            )

        bonds: List[dict[str, Any]] = []
        for bond in top_bonds:
            begin = bond.attrib.get("B")
            end = bond.attrib.get("E")
            if begin not in node_ids or end not in node_ids:
                continue
            bonds.append(
                {
                    "id": bond.attrib["id"],
                    "begin": begin,
                    "end": end,
                    "order": int(bond.attrib.get("Order", "1") or 1),
                    "display": bond.attrib.get("Display"),
                    "doublePosition": bond.attrib.get("DoublePosition"),
                }
            )

        if len(nodes) < 2 or not bonds:
            continue

        fragments.append(
            {
                "id": frag.attrib.get("id"),
                "bbox": [round(v, 2) for v in bbox],
                "center": [round(float(center[0]), 2), round(float(center[1]), 2)],
                "z": int(frag.attrib.get("Z", "0") or 0),
                "nodes": nodes,
                "bonds": bonds,
            }
        )

    return fragments
