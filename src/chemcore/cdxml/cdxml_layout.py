from __future__ import annotations

from bisect import bisect_right
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple
import re
import xml.etree.ElementTree as ET

from .cdxml_shared import PLACEHOLDER_TYPES, parse_bbox_center, parse_xy, parse_xyz


TextPt = Tuple[str, Tuple[float, float]]
TextBlock = Dict[str, Any]
MolRec = Tuple[str, Tuple[float, float], Tuple[float, float], Optional[str]]
MolWithLabelRec = Tuple[str, Tuple[float, float], Tuple[float, float], str, Optional[str]]

_SUBSCRIPT_JOIN_RE = re.compile(r"\b([A-Z][A-Za-z]+)\s*\n\s*([0-9]+)")
_ALNUM_WS_RE = re.compile(r"([A-Za-z0-9])([\t\r ]+)([A-Za-z0-9])")
_PAREN_NUM_JOIN_RE = re.compile(r"(\))\s*\n\s*([0-9]+)")
_PAREN_ONLY_LINE_RE = re.compile(r"^[\s]*[\(（].*[\)）][\s]*$")
_QUESTION_LINE_RE = re.compile(r"^[\s]*\?[\s]*$")


def extract_text_blocks_non_substituent(cdxml_path: str) -> List[TextBlock]:
    xml_path = Path(f"{cdxml_path}.cdxml")
    if not (xml_path.exists() and xml_path.is_file()):
        raise FileNotFoundError(str(xml_path))

    root = ET.parse(str(xml_path)).getroot()

    def localname(tag: str) -> str:
        return tag.rsplit("}", 1)[-1]

    def is_exact(el: ET.Element, name: str) -> bool:
        return localname(el.tag) == name

    parent: Dict[ET.Element, ET.Element] = {}
    for p in root.iter():
        for ch in list(p):
            parent[ch] = p

    bonds: List[Tuple[str, str]] = []
    for el in root.iter():
        if is_exact(el, "b"):
            b = el.attrib.get("B")
            e = el.attrib.get("E")
            if b and e:
                bonds.append((b, e))
    bonded_node_ids: Set[str] = set()
    for b, e in bonds:
        bonded_node_ids.add(b)
        bonded_node_ids.add(e)

    placeholder_inline_nodes: Dict[str, Set[str]] = {}
    for el in root.iter():
        if not (is_exact(el, "n") and "id" in el.attrib):
            continue
        pid = el.attrib["id"]
        if el.attrib.get("NodeType", "") not in PLACEHOLDER_TYPES:
            continue
        inline_nodes: Set[str] = set()
        for ch in list(el):
            if is_exact(ch, "fragment"):
                for sub in ch.iter():
                    if is_exact(sub, "n") and "id" in sub.attrib:
                        inline_nodes.add(sub.attrib["id"])
        if inline_nodes:
            placeholder_inline_nodes[pid] = inline_nodes

    def placeholder_is_attached(pid: str) -> bool:
        inline = placeholder_inline_nodes.get(pid, set())
        for b, e in bonds:
            if b == pid:
                if e not in inline:
                    return True
            elif e == pid:
                if b not in inline:
                    return True
        return False

    attached_placeholder_cache: Dict[str, bool] = {}

    def nearest_ancestor_n(t_el: ET.Element) -> Optional[ET.Element]:
        cur = t_el
        while cur in parent:
            cur = parent[cur]
            if is_exact(cur, "n"):
                return cur
        return None

    def closest_placeholder_ancestor_n(t_el: ET.Element) -> Optional[ET.Element]:
        cur = t_el
        while cur in parent:
            cur = parent[cur]
            if is_exact(cur, "n"):
                nt = cur.attrib.get("NodeType", "")
                if nt in PLACEHOLDER_TYPES:
                    return cur
        return None

    def is_direct_child(child: ET.Element, maybe_parent: ET.Element) -> bool:
        return parent.get(child) is maybe_parent

    def inherited_z(el: ET.Element) -> int:
        cur: Optional[ET.Element] = el
        while cur is not None:
            value = cur.attrib.get("Z")
            if value is not None:
                try:
                    return int(value or 0)
                except ValueError:
                    return 0
            cur = parent.get(cur)
        return 0

    texts: List[TextBlock] = []
    for el in root.iter():
        if not is_exact(el, "t"):
            continue
        txt = "".join(el.itertext()).strip()
        if not txt:
            continue
        p = parse_xy(el.attrib.get("p"))
        if p is None:
            p = parse_bbox_center(el.attrib.get("BoundingBox"))
        if p is None:
            continue
        anc_n = nearest_ancestor_n(el)
        if anc_n is not None and "Element" in anc_n.attrib:
            anc_id = anc_n.attrib.get("id", "")
            # Keep standalone element-like labels used as table headers, while
            # still suppressing atom symbols that belong to real molecular graphs.
            if not anc_id or anc_id in bonded_node_ids:
                continue
        ph_n = closest_placeholder_ancestor_n(el)
        if ph_n is not None:
            pid = ph_n.attrib.get("id", "")
            if pid:
                if pid not in attached_placeholder_cache:
                    attached_placeholder_cache[pid] = placeholder_is_attached(pid)
                if attached_placeholder_cache[pid]:
                    continue
            if not is_direct_child(el, ph_n):
                continue
        bb = el.attrib.get("BoundingBox")
        bbox = None
        if bb:
            try:
                vals = [float(v) for v in bb.strip().split()[:4]]
                if len(vals) == 4:
                    bbox = (vals[0], vals[1], vals[2], vals[3])
            except Exception:
                bbox = None
        texts.append(
            {
                "text": txt,
                "point": (float(p[0]), float(p[1])),
                "bbox": bbox,
                "align": (el.attrib.get("Justification") or el.attrib.get("LabelJustification") or "Left").lower(),
                "labelAlignment": (el.attrib.get("LabelAlignment") or "").lower(),
                "fontSize": float(next((ch.attrib.get("size") for ch in list(el) if is_exact(ch, "s") and ch.attrib.get("size")), "10.0")),
                "runs": [
                    {
                        "text": "".join(ch.itertext()),
                        "font": ch.attrib.get("font"),
                        "size": float(ch.attrib.get("size", "10.0") or 10.0),
                        "face": int(ch.attrib.get("face", "0") or 0),
                        "color": ch.attrib.get("color"),
                    }
                    for ch in list(el)
                    if is_exact(ch, "s") and "".join(ch.itertext())
                ],
                "color": el.attrib.get("color"),
                "z": inherited_z(el),
            }
        )
    return texts


def extract_texts_non_substituent(cdxml_path: str) -> List[TextPt]:
    return [(item["text"], item["point"]) for item in extract_text_blocks_non_substituent(cdxml_path)]


def extract_line_graphics(cdxml_path: str) -> List[dict]:
    xml_path = Path(f"{cdxml_path}.cdxml")
    if not (xml_path.exists() and xml_path.is_file()):
        raise FileNotFoundError(str(xml_path))

    root = ET.parse(str(xml_path)).getroot()

    def _localname(tag: str) -> str:
        return tag.rsplit("}", 1)[-1]

    def _has_arrow_like_attr(attrib: dict) -> bool:
        v1 = attrib.get("ArrowheadHead", "")
        v2 = attrib.get("ArrowheadTail", "")
        v3 = attrib.get("ArrowType", "")
        v4 = attrib.get("ArrowheadType", "")

        def ok(v: str) -> bool:
            v = (v or "").strip()
            return bool(v) and v.lower() not in {"none", "0", "false"}

        return ok(v1) or ok(v2) or ok(v3) or ok(v4)

    def _parse_scaled_number(value: str | None) -> float | None:
        if value in (None, ""):
            return None
        try:
            return float(value) / 100.0
        except Exception:
            return None

    lines: List[dict] = []
    for el in root.iter():
        lname = _localname(el.tag)
        if lname not in {"arrow", "graphic"}:
            continue
        if lname == "graphic" and el.attrib.get("GraphicType") != "Line":
            continue
        if lname == "graphic" and "SupersededBy" in el.attrib:
            continue
        if lname == "graphic" and not _has_arrow_like_attr(el.attrib):
            continue
        if lname == "arrow" and not (_has_arrow_like_attr(el.attrib) or el.attrib.get("Head3D") or el.attrib.get("Tail3D")):
            continue

        head3d = parse_xyz(el.attrib.get("Head3D"))
        tail3d = parse_xyz(el.attrib.get("Tail3D"))
        head2d = (head3d[0], head3d[1]) if head3d else None
        tail2d = (tail3d[0], tail3d[1]) if tail3d else None

        center2d = None
        if head2d and tail2d:
            center2d = ((head2d[0] + tail2d[0]) / 2.0, (head2d[1] + tail2d[1]) / 2.0)
        else:
            center2d = parse_bbox_center(el.attrib.get("BoundingBox"))

        arrow_head = el.attrib.get("ArrowheadHead") or ("Full" if el.attrib.get("ArrowType") == "FullHead" else None)
        arrow_tail = el.attrib.get("ArrowheadTail")
        arrow_type = el.attrib.get("ArrowType")
        is_arrow = bool(arrow_type or arrow_head or arrow_tail)

        lines.append(
            {
                "id": el.attrib.get("id"),
                "head": (float(head2d[0]), float(head2d[1])) if head2d else None,
                "tail": (float(tail2d[0]), float(tail2d[1])) if tail2d else None,
                "center": (float(center2d[0]), float(center2d[1])) if center2d else None,
                "isArrow": is_arrow,
                "arrowheadType": el.attrib.get("ArrowheadType") or el.attrib.get("ArrowType"),
                "arrowheadHead": arrow_head,
                "arrowheadTail": arrow_tail,
                "headSize": _parse_scaled_number(el.attrib.get("HeadSize")),
                "arrowheadCenterSize": _parse_scaled_number(el.attrib.get("ArrowheadCenterSize")),
                "arrowheadWidth": _parse_scaled_number(el.attrib.get("ArrowheadWidth")),
                "z": int(el.attrib.get("Z", "0") or 0),
            }
        )
    return lines


def extract_arrows(cdxml_path: str) -> List[dict]:
    return [line for line in extract_line_graphics(cdxml_path) if line.get("isArrow")]


def _normalize_text_block(s: str) -> str:
    if not s:
        return s
    prev = None
    cur = s
    while prev != cur:
        prev = cur
        cur = _SUBSCRIPT_JOIN_RE.sub(r"\1\2", cur)
        cur = _PAREN_NUM_JOIN_RE.sub(r"\1\2", cur)

        def _fix(m: re.Match) -> str:
            ws = m.group(2)
            if "\t" in ws or "\r" in ws:
                if m.group(1).isdigit() and m.group(3).isdigit():
                    return f"{m.group(1)} {m.group(3)}"
                return f"{m.group(1)}{m.group(3)}"
            return f"{m.group(1)} {m.group(3)}"

        cur = _ALNUM_WS_RE.sub(_fix, cur)

    lines = [ln.strip() for ln in cur.splitlines() if not _QUESTION_LINE_RE.match(ln)]
    lines = [ln for ln in lines if ln]
    if not lines:
        return ""

    out = [lines[0]]
    for ln in lines[1:]:
        prev_ln = out[-1]
        if _PAREN_ONLY_LINE_RE.match(ln):
            out[-1] = prev_ln.rstrip() + " " + ln
            continue
        if (len(prev_ln) <= 2) or (len(ln) <= 2):
            out[-1] = prev_ln + ln
            continue
        if prev_ln.endswith(")") and ln[:1].isdigit():
            out[-1] = prev_ln + ln
            continue
        out.append(ln)
    return "\n".join(out)


def _merge_paren_only_lines(s: str) -> str:
    if not s:
        return s
    lines = s.splitlines()
    out: List[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped and _PAREN_ONLY_LINE_RE.match(stripped) and out:
            out[-1] = out[-1].rstrip() + " " + stripped
        else:
            out.append(line)
    return "\n".join(out)


def split_tables_by_entry(
    items: List[TextPt],
    y_tol: float = 2.5,
    x_merge_tol: float = 10.0,
) -> Tuple[List[List[List[str]]], List[Tuple[float, float, float, float]], List[TextPt]]:
    max_row_gap_without_entry = 3
    def norm(s: str) -> str:
        return re.sub(r"\s+", " ", s.strip()).lower()

    def is_entry_header(s: str) -> bool:
        return norm(s) == "entry"

    def is_arabic_int(s: str) -> bool:
        return re.fullmatch(r"\d+", s.strip()) is not None

    def split_vertical_cell_values(s: str) -> List[str]:
        if not s:
            return []
        return [ln.strip() for ln in (s or "").splitlines() if ln.strip()]

    def compose_cell_text(fragments: List[dict]) -> str:
        if not fragments:
            return ""
        ordered = sorted(fragments, key=lambda p: (p["y"], p["x"]))
        lines: List[str] = []
        current: List[str] = []
        current_y: Optional[float] = None
        for frag in ordered:
            if current_y is None or abs(frag["y"] - current_y) <= y_tol:
                current.append(frag["t"])
                current_y = frag["y"] if current_y is None else (current_y + frag["y"]) / 2.0
            else:
                lines.append(" ".join(s.strip() for s in current if s.strip()))
                current = [frag["t"]]
                current_y = frag["y"]
        if current:
            lines.append(" ".join(s.strip() for s in current if s.strip()))
        return _normalize_text_block("\n".join(line for line in lines if line.strip())).replace("\n", "\n")

    pts = [{"idx": i, "t": t, "x": float(x), "y": float(y)} for i, (t, (x, y)) in enumerate(items)]
    pts_sorted = sorted(pts, key=lambda p: p["y"])

    rows = []
    for p in pts_sorted:
        if not rows or abs(p["y"] - rows[-1]["y"]) > y_tol:
            rows.append({"y": p["y"], "pts": [p]})
        else:
            rows[-1]["pts"].append(p)
            rows[-1]["y"] = sum(q["y"] for q in rows[-1]["pts"]) / len(rows[-1]["pts"])
    for r in rows:
        r["pts"].sort(key=lambda p: p["x"])

    pt_to_row = {}
    for rid, r in enumerate(rows):
        for p in r["pts"]:
            pt_to_row[p["idx"]] = rid

    def merge_x(xs: List[float], tol: float) -> List[float]:
        xs = sorted(xs)
        if not xs:
            return []
        clusters = [[xs[0]]]
        for v in xs[1:]:
            if abs(v - clusters[-1][-1]) <= tol:
                clusters[-1].append(v)
            else:
                clusters.append([v])
        return [sum(c) / len(c) for c in clusters]

    used = set()
    tables: List[List[List[str]]] = []
    bboxes: List[Tuple[float, float, float, float]] = []
    entry_pts = [p for p in pts if is_entry_header(p["t"])]
    entry_pts.sort(key=lambda p: (p["y"], p["x"]))

    for anchor in entry_pts:
        if anchor["idx"] in used:
            continue
        r0 = pt_to_row[anchor["idx"]]
        header_row = [p for p in rows[r0]["pts"] if p["idx"] not in used]
        entry_items = [p for p in header_row if is_entry_header(p["t"])]
        if not entry_items:
            continue
        entry_item = min(entry_items, key=lambda p: abs(p["x"] - anchor["x"]))
        x0 = entry_item["x"]
        header_x_raw = [p["x"] for p in header_row if p["x"] >= x0 - 1e-6]
        col_x = merge_x(header_x_raw, x_merge_tol)
        col_x.sort()
        if len(col_x) < 2:
            continue

        left_gap = col_x[1] - col_x[0]
        right_gap = col_x[-1] - col_x[-2]
        left_limit = col_x[0] - 0.6 * left_gap
        right_limit = col_x[-1] + 0.6 * right_gap
        bounds = [(col_x[i] + col_x[i + 1]) / 2.0 for i in range(len(col_x) - 1)]

        def _row_entry_num(rpts: List[dict]) -> Optional[int]:
            nums = []
            for p in rpts:
                if is_arabic_int(p["t"]) and p["x"] < bounds[0]:
                    try:
                        nums.append((int(p["t"]), float(p["x"])))
                    except Exception:
                        pass
            if not nums:
                return None
            return min(nums, key=lambda t: abs(t[1] - col_x[0]))[0]

        data_pairs: List[Tuple[int, int]] = []
        miss_streak = 0
        for rid in range(r0 + 1, len(rows)):
            rpts = [p for p in rows[rid]["pts"] if p["idx"] not in used and left_limit <= p["x"] <= right_limit]
            if not rpts:
                continue
            n = _row_entry_num(rpts)
            if n is not None:
                data_pairs.append((rid, n))
                miss_streak = 0
            else:
                miss_streak += 1
                if miss_streak >= max_row_gap_without_entry:
                    break

        if not data_pairs:
            column_blocks: List[List[str]] = [[] for _ in range(len(col_x))]
            block_used = set()
            miss_streak = 0
            for rid in range(r0 + 1, len(rows)):
                rpts = [p for p in rows[rid]["pts"] if p["idx"] not in used and left_limit <= p["x"] <= right_limit]
                if not rpts:
                    continue
                row_has_cell = False
                for p in rpts:
                    ci = bisect_right(bounds, p["x"])
                    if 0 <= ci < len(col_x):
                        column_blocks[ci].append(p["t"])
                        block_used.add(p["idx"])
                        row_has_cell = True
                if row_has_cell:
                    miss_streak = 0
                else:
                    miss_streak += 1
                    if miss_streak >= max_row_gap_without_entry:
                        break

            split_cols: List[List[str]] = []
            for block_list in column_blocks:
                raw_block = "\n".join(part for part in block_list if (part or "").strip())
                split_cols.append(split_vertical_cell_values(raw_block))

            if split_cols and split_cols[0]:
                first_col = split_cols[0]
                if (
                    all(is_arabic_int(v) for v in first_col)
                    and [int(v) for v in first_col] == list(range(int(first_col[0]), int(first_col[0]) + len(first_col)))
                    and all(len(col) in {0, len(first_col)} for col in split_cols[1:])
                ):
                    header_cells = []
                    table_used = set(block_used)
                    for p in header_row:
                        if left_limit <= p["x"] <= right_limit:
                            table_used.add(p["idx"])
                    for ci in range(len(col_x)):
                        frags = [
                            p["t"] for p in header_row
                            if bisect_right(bounds, p["x"]) == ci and left_limit <= p["x"] <= right_limit
                        ]
                        header_cells.append(_normalize_text_block(" ".join(s.strip() for s in frags if s.strip())).replace("\n", " "))
                    if norm(header_cells[0]) == "entry":
                        grid = [header_cells]
                        for row_idx in range(len(first_col)):
                            row_out = []
                            for col in split_cols:
                                val = col[row_idx] if row_idx < len(col) else ""
                                row_out.append(_normalize_text_block(val).replace("\n", " "))
                            grid.append(row_out)
                        xs = [pts[i]["x"] for i in table_used]
                        ys = [pts[i]["y"] for i in table_used]
                        bboxes.append((min(xs), min(ys), max(xs), max(ys)))
                        used |= table_used
                        tables.append(grid)
                        continue
            continue

        data_pairs.sort(key=lambda x: x[0])
        kept = [data_pairs[0]]
        for rid, n in data_pairs[1:]:
            if n == kept[-1][1] + 1:
                kept.append((rid, n))
            else:
                break
        data_row_ids = [rid for rid, _ in kept]
        if not data_row_ids:
            continue

        table_row_ids = [r0] + data_row_ids
        table_used = set()
        row_cells: List[List[List[dict]]] = [
            [[] for _ in range(len(col_x))]
            for _ in range(len(table_row_ids))
        ]
        row_yrefs = [rows[rid]["y"] for rid in table_row_ids]

        for row_pos, rid in enumerate(table_row_ids):
            rpts = [p for p in rows[rid]["pts"] if p["idx"] not in used and left_limit <= p["x"] <= right_limit]
            for p in rpts:
                ci = bisect_right(bounds, p["x"])
                row_cells[row_pos][ci].append(p)
                table_used.add(p["idx"])

        table_ymin = rows[r0]["y"] - y_tol
        table_ymax = rows[data_row_ids[-1]]["y"] + max(18.0, y_tol * 4.0)
        extra_pts = [
            p for p in pts
            if p["idx"] not in used
            and p["idx"] not in table_used
            and left_limit <= p["x"] <= right_limit
            and table_ymin <= p["y"] <= table_ymax
        ]
        for p in extra_pts:
            ci = bisect_right(bounds, p["x"])
            if not (0 <= ci < len(col_x)):
                continue
            if ci == 0:
                continue
            nearest_row = min(
                range(1, len(table_row_ids)),
                key=lambda idx: (abs(p["y"] - row_yrefs[idx]), idx),
            )
            row_cells[nearest_row][ci].append(p)
            table_used.add(p["idx"])

        grid: List[List[str]] = []
        for row_pos in range(len(table_row_ids)):
            row_out = []
            for frag_list in row_cells[row_pos]:
                cell_text = compose_cell_text(frag_list)
                row_out.append(cell_text.replace("\n", "\n") if cell_text else "")
            grid.append(row_out)

        if norm(grid[0][0]) != "entry":
            continue
        xs = [pts[i]["x"] for i in table_used]
        ys = [pts[i]["y"] for i in table_used]
        bboxes.append((min(xs), min(ys), max(xs), max(ys)))
        used |= table_used
        tables.append(grid)

    remaining = [items[i] for i in range(len(items)) if i not in used]
    return tables, bboxes, remaining


def attach_texts_near_molecules(
    mols: Sequence[MolRec],
    remaining: Sequence[TextPt],
    x_factor: float = 0.8,
    inclusive: bool = True,
) -> Tuple[List[MolWithLabelRec], List[TextPt]]:
    rem = list(remaining)
    labels: List[str] = [""] * len(mols)

    def _match_pass(candidate_indices: Sequence[int], available_text_indices: Sequence[int], *, current_x_factor: float, extra_y: float) -> Set[int]:
        assigned = {i: [] for i in candidate_indices}
        used_here: Set[int] = set()
        for j in available_text_indices:
            txt, (x, y) = rem[j]
            best = None
            best_idx = None
            for i in candidate_indices:
                _smi, (cx, cy), (w, h), _molblock_cdxml = mols[i]
                x1 = cx - current_x_factor * w
                x2 = cx + current_x_factor * w
                y1 = cy
                y2 = cy + h + extra_y
                ok = (x1 <= x <= x2) and (y1 <= y <= y2) if inclusive else (x1 < x < x2) and (y1 < y < y2)
                if not ok:
                    continue
                score = (abs(x - cx), max(0.0, y - cy), i)
                if best is None or score < best:
                    best = score
                    best_idx = i
            if best_idx is None:
                continue
            assigned[best_idx].append((y, x, j, txt))
            used_here.add(j)
        for i, hits in assigned.items():
            if not hits:
                continue
            hits.sort(key=lambda z: (z[0], z[1]))
            labels[i] = "\n".join(_normalize_text_block(t) for _, _, _, t in hits)
        return used_here

    unmatched_mol_indices = list(range(len(mols)))
    unused_text_indices = list(range(len(rem)))
    used_pass1 = _match_pass(unmatched_mol_indices, unused_text_indices, current_x_factor=x_factor, extra_y=15)
    unmatched_mol_indices = [i for i in unmatched_mol_indices if not labels[i].strip()]
    unused_text_indices = [j for j in unused_text_indices if j not in used_pass1]

    if unmatched_mol_indices and unused_text_indices:
        used_pass2 = _match_pass(unmatched_mol_indices, unused_text_indices, current_x_factor=1.1, extra_y=30)
        unused_text_indices = [j for j in unused_text_indices if j not in used_pass2]

    new_mols: List[MolWithLabelRec] = []
    for i, (smi, center, wh, molblock_cdxml) in enumerate(mols):
        new_mols.append((smi, center, wh, labels[i], molblock_cdxml))

    new_remaining = [rem[j] for j in unused_text_indices]
    return new_mols, new_remaining


def split_condition_and_note(
    arrows: List[Dict],
    table_bboxes: List[Tuple[float, float, float, float]],
    remaining: List[TextPt],
    x_expand: float = 0.20,
) -> Tuple[str, str]:
    if not table_bboxes:
        cond_hits = sorted(remaining, key=lambda it: (it[1][1], it[1][0]))
        condition = "\n".join(_normalize_text_block(t) for t, _ in cond_hits)
        condition = _merge_paren_only_lines(condition)
        return condition, ""

    table_ymin = min(bb[1] for bb in table_bboxes)
    xranges: List[Tuple[float, float]] = []
    for a in arrows:
        head = a.get("head")
        tail = a.get("tail")
        if head is None or tail is None:
            continue
        x1, _ = head
        x2, _ = tail
        xmin, xmax = (x1, x2) if x1 <= x2 else (x2, x1)
        pad = x_expand * (xmax - xmin)
        xranges.append((xmin - pad, xmax + pad))

    def in_any_xrange(x: float) -> bool:
        for lo, hi in xranges:
            if lo <= x <= hi:
                return True
        return False

    cond = []
    note = []
    for txt, (x, y) in remaining:
        if xranges and (y < table_ymin) and in_any_xrange(x):
            cond.append((y, x, txt))
        else:
            note.append((y, x, txt))

    cond.sort(key=lambda t: (t[0], t[1]))
    note.sort(key=lambda t: (t[0], t[1]))
    condition = "\n".join(_normalize_text_block(t[2]) for t in cond)
    condition = _merge_paren_only_lines(condition)
    note_str = "\n".join(_normalize_text_block(t[2]) for t in note)
    return condition, note_str
