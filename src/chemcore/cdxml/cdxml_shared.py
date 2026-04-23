from __future__ import annotations

from typing import Dict, Optional, Set, Tuple
import xml.etree.ElementTree as ET


PLACEHOLDER_TYPES: Set[str] = {"Fragment", "Nickname", "Unspecified"}
ECP_TYPE = "ExternalConnectionPoint"
ABBREV_TEMPLATE_SMILES: Dict[str, str] = {
    "Me": "*C",
    "Et": "*CC",
    "i-Pr": "*C(C)C",
    "iPr": "*C(C)C",
    "t-Bu": "*C(C)(C)C",
    "tBu": "*C(C)(C)C",
    "Ph": "*c1ccccc1",
    "Bn": "*Cc1ccccc1",
    "CF3": "*C(F)(F)F",
    "N3": "*N=[N+]=[N-]",
}


def is_tag(el: ET.Element, name: str) -> bool:
    return el.tag.endswith(name)


def parse_xy(p: Optional[str]) -> Optional[Tuple[float, float]]:
    if not p:
        return None
    s = p.strip().split()
    if len(s) < 2:
        return None
    try:
        return float(s[0]), float(s[1])
    except Exception:
        return None


def parse_bbox_center(bb: Optional[str]) -> Optional[Tuple[float, float]]:
    if not bb:
        return None
    s = bb.strip().split()
    if len(s) < 4:
        return None
    try:
        x1, y1, x2, y2 = map(float, s[:4])
        return (x1 + x2) / 2.0, (y1 + y2) / 2.0
    except Exception:
        return None


def parse_xyz(s: Optional[str]) -> Optional[Tuple[float, float, float]]:
    if not s:
        return None
    parts = s.strip().split()
    if len(parts) < 3:
        return None
    try:
        return float(parts[0]), float(parts[1]), float(parts[2])
    except Exception:
        return None
