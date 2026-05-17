from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


CHEMDRAW_EMF_PAGE_SCALE = 0.26666668


def load_json(path: Path, encoding: str | None = None):
    if encoding is not None:
        return json.loads(path.read_text(encoding=encoding))
    for candidate in ("utf-8", "utf-8-sig", "utf-16"):
        try:
            return json.loads(path.read_text(encoding=candidate))
        except Exception:
            pass
    raise RuntimeError(f"unable to decode json file: {path}")


def frac(value: float) -> float:
    return value % 1.0


def main() -> None:
    ap = argparse.ArgumentParser(
        description=(
            "Compute packaged GDI+ page-space text placement phases for attached labels "
            "and join them with same-shell dy sensitivity metrics."
        )
    )
    ap.add_argument("preview_bounds_json")
    ap.add_argument("attached_text_primitives_json")
    ap.add_argument("attached_geometry_json")
    ap.add_argument("attached_phase_sensitivity_json")
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    preview = load_json(Path(args.preview_bounds_json))
    primitives = load_json(Path(args.attached_text_primitives_json))
    geometry_rows = {
        row["nodeId"]: row
        for row in load_json(Path(args.attached_geometry_json))["rows"]
    }
    sensitivity_rows = {
        row["nodeId"]: row
        for row in load_json(Path(args.attached_phase_sensitivity_json))["rows"]
    }

    draw = preview["drawBoundsLogical"]
    source = preview["sourceBoundsSvgPx"]
    min_x, min_y, max_x, max_y = source
    source_width = max(max_x - min_x, 1.0)
    source_height = max(max_y - min_y, 1.0)
    target_width = max(draw["width"], 1)
    target_height = max(draw["height"], 1)
    width_ratio = target_width / source_width
    height_ratio = target_height / source_height
    scale = min(width_ratio, height_ratio)
    drawn_width = source_width * scale
    drawn_height = source_height * scale
    offset_x = draw["left"] + (target_width - drawn_width) / 2.0
    offset_y = draw["top"] + (target_height - drawn_height) / 2.0

    rows = []
    for primitive in primitives:
        node_id = primitive.get("nodeId")
        if not node_id:
            continue
        geo = geometry_rows.get(node_id, {})
        sens = sensitivity_rows.get(node_id, {})
        x = primitive["x"]
        y = primitive["y"]
        baseline_offset = primitive.get("baselineOffset") or primitive["fontSize"] * 0.86
        font_size = primitive["fontSize"]
        font_px = font_size * scale / CHEMDRAW_EMF_PAGE_SCALE
        baseline_top_px = baseline_offset * scale / CHEMDRAW_EMF_PAGE_SCALE
        baseline_y_page = (offset_y + (y - min_y) * scale) / CHEMDRAW_EMF_PAGE_SCALE
        x_page = (offset_x + (x - min_x) * scale) / CHEMDRAW_EMF_PAGE_SCALE
        top_page = baseline_y_page - baseline_top_px
        rect_height_page = max(font_px * 1.45, 1.0)
        rect_bottom_page = top_page + rect_height_page

        row = {
            "nodeId": node_id,
            "text": geo.get("text") or primitive.get("text") or "",
            "fill": geo.get("fill") or primitive.get("fill"),
            "component": geo.get("component"),
            "componentQuadrant": geo.get("componentQuadrant"),
            "cdxmlLabelJustification": geo.get("cdxmlLabelJustification"),
            "primaryNeighborBucket": geo.get("primaryNeighborBucket"),
            "gapRight": geo.get("gapRight"),
            "centerYPhase": (geo["labelCenterWorld"][1] % 1.0)
            if geo.get("labelCenterWorld")
            else None,
            "boxTopPhase": (geo["worldBox"][1] % 1.0) if geo.get("worldBox") else None,
            "baselineYPage": baseline_y_page,
            "baselineYPagePhase": frac(baseline_y_page),
            "xPage": x_page,
            "xPagePhase": frac(x_page),
            "fontPx": font_px,
            "fontPxPhase": frac(font_px),
            "baselineTopPx": baseline_top_px,
            "baselineTopPxPhase": frac(baseline_top_px),
            "topPage": top_page,
            "topPagePhase": frac(top_page),
            "rectHeightPage": rect_height_page,
            "rectHeightPagePhase": frac(rect_height_page),
            "rectBottomPage": rect_bottom_page,
            "rectBottomPagePhase": frac(rect_bottom_page),
            "baseIou": sens.get("baseIou"),
            "frame_dy1DeltaIou": sens.get("frame_dy1DeltaIou"),
            "frame_dy3DeltaIou": sens.get("frame_dy3DeltaIou"),
        }
        rows.append(row)

    def summarize(key_fields):
        groups = defaultdict(lambda: {"count": 0, "sumDy1": 0.0, "sumDy3": 0.0, "samples": []})
        for row in rows:
            key = tuple(row.get(field) for field in key_fields)
            g = groups[key]
            g["count"] += 1
            g["sumDy1"] += row.get("frame_dy1DeltaIou") or 0.0
            g["sumDy3"] += row.get("frame_dy3DeltaIou") or 0.0
            if len(g["samples"]) < 8:
                g["samples"].append(
                    {
                        "nodeId": row["nodeId"],
                        "text": row["text"],
                        "fill": row["fill"],
                        "topPagePhase": row["topPagePhase"],
                        "baselineYPagePhase": row["baselineYPagePhase"],
                        "rectBottomPagePhase": row["rectBottomPagePhase"],
                        "frame_dy1DeltaIou": row["frame_dy1DeltaIou"],
                        "frame_dy3DeltaIou": row["frame_dy3DeltaIou"],
                    }
                )
        out = []
        for key, g in groups.items():
            item = {
                "key": {field: value for field, value in zip(key_fields, key)},
                "count": g["count"],
                "avgDy1DeltaIou": g["sumDy1"] / g["count"],
                "avgDy3DeltaIou": g["sumDy3"] / g["count"],
                "samples": g["samples"],
            }
            out.append(item)
        out.sort(key=lambda item: (-item["count"], repr(item["key"])))
        return out

    payload = {
        "previewBoundsJson": str(Path(args.preview_bounds_json).resolve()),
        "attachedTextPrimitivesJson": str(Path(args.attached_text_primitives_json).resolve()),
        "attachedGeometryJson": str(Path(args.attached_geometry_json).resolve()),
        "attachedPhaseSensitivityJson": str(Path(args.attached_phase_sensitivity_json).resolve()),
        "transform": {
            "sourceBoundsSvgPx": source,
            "drawBoundsLogical": draw,
            "widthRatio": width_ratio,
            "heightRatio": height_ratio,
            "scale": scale,
            "offsetX": offset_x,
            "offsetY": offset_y,
            "pageScale": CHEMDRAW_EMF_PAGE_SCALE,
        },
        "rows": rows,
        "summaryByTopAndBaselinePhase": summarize(["topPagePhase", "baselineYPagePhase"]),
        "summaryByTopAndRectBottomPhase": summarize(["topPagePhase", "rectBottomPagePhase"]),
        "summaryByNeighborAndTopPhase": summarize(["primaryNeighborBucket", "topPagePhase"]),
    }
    Path(args.output).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
