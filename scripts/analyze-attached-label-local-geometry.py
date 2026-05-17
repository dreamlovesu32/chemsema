from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path


def angle_bucket(deg: float) -> str:
    if -45.0 <= deg < 45.0:
        return "east"
    if 45.0 <= deg < 135.0:
        return "south"
    if deg >= 135.0 or deg < -135.0:
        return "west"
    return "north"


def primary_neighbor_direction(node_id: str, nodes: dict, bonds: list[dict]) -> tuple[str | None, float | None]:
    pos = nodes[node_id]["position"]
    best = None
    for bond in bonds:
        if bond["begin"] == node_id:
            other = nodes[bond["end"]]["position"]
        elif bond["end"] == node_id:
            other = nodes[bond["begin"]]["position"]
        else:
            continue
        dx = other[0] - pos[0]
        dy = other[1] - pos[1]
        dist2 = dx * dx + dy * dy
        if best is None or dist2 > best[0]:
            best = (dist2, dx, dy)
    if best is None:
        return None, None
    _, dx, dy = best
    deg = math.degrees(math.atan2(dy, dx))
    return angle_bucket(deg), deg


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Analyze same-shell attached-group label residuals against local geometry/context."
    )
    parser.add_argument("payload_json")
    parser.add_argument("label_context_json")
    parser.add_argument("geometry_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    payload = json.loads(Path(args.payload_json).read_text(encoding="utf-8"))
    doc = json.loads(payload["chemcoreDocumentJson"])
    mol_obj = next(obj for obj in doc["objects"] if obj["id"] == "obj_cdxml_merged_molecule")
    tx, ty = mol_obj["transform"]["translate"]
    frag = doc["resources"]["mol_cdxml_merged"]["data"]
    nodes = {node["id"]: node for node in frag["nodes"]}
    bonds = frag["bonds"]
    geometry = json.loads(Path(args.geometry_json).read_text(encoding="utf-8"))
    component_boxes = {
        comp["roleGuess"]: comp["worldBox"] for comp in geometry["moleculeComponents"]["components"]
    }
    rows = json.loads(Path(args.label_context_json).read_text(encoding="utf-8"))["rows"]

    detailed_rows = []
    grouped = defaultdict(
        lambda: {
            "count": 0,
            "sumResidual": 0,
            "sumDw": 0,
            "sumDh": 0,
            "sumDx": 0,
            "sumDy": 0,
            "rows": [],
        }
    )

    for row in rows:
        if row.get("layout") != "attached-group":
            continue
        component_name = row["component"]
        component_box = component_boxes[component_name]
        label_box = row["worldBox"]
        node = nodes[row["nodeId"]]
        node_world = [node["position"][0] + tx, node["position"][1] + ty]
        label_center = [(label_box[0] + label_box[2]) / 2.0, (label_box[1] + label_box[3]) / 2.0]
        rel_x = (label_center[0] - component_box[0]) / (component_box[2] - component_box[0])
        rel_y = (label_center[1] - component_box[1]) / (component_box[3] - component_box[1])
        neighbor_bucket, neighbor_angle = primary_neighbor_direction(row["nodeId"], nodes, bonds)
        overhang = {
            "left": component_box[0] - label_box[0],
            "right": label_box[2] - component_box[2],
            "top": component_box[1] - label_box[1],
            "bottom": label_box[3] - component_box[3],
        }
        label_offset = [label_center[0] - node_world[0], label_center[1] - node_world[1]]

        enriched = {
            **row,
            "componentWorldBox": component_box,
            "labelCenterWorld": label_center,
            "nodeWorld": node_world,
            "componentRelCenter": [rel_x, rel_y],
            "componentHalfX": "left" if rel_x < 0.5 else "right",
            "componentHalfY": "top" if rel_y < 0.5 else "bottom",
            "componentQuadrant": f"{'L' if rel_x < 0.5 else 'R'}{'T' if rel_y < 0.5 else 'B'}",
            "overhangToComponent": overhang,
            "labelOffsetFromNode": label_offset,
            "primaryNeighborBucket": neighbor_bucket,
            "primaryNeighborAngle": neighbor_angle,
        }
        detailed_rows.append(enriched)

        key = json.dumps(
            {
                "component": component_name,
                "text": row["text"],
                "fill": row["fill"],
                "nodeType": row.get("nodeType"),
                "cdxmlLabelJustification": row.get("cdxmlLabelJustification"),
                "componentQuadrant": enriched["componentQuadrant"],
                "primaryNeighborBucket": neighbor_bucket,
            },
            ensure_ascii=False,
            sort_keys=True,
        )
        group = grouped[key]
        group["count"] += 1
        group["sumResidual"] += row["residualCount"]
        dw, dh = row.get("deltaDims", [0, 0])
        dx, dy = row.get("deltaTopLeft", [0, 0])
        group["sumDw"] += dw
        group["sumDh"] += dh
        group["sumDx"] += dx
        group["sumDy"] += dy
        group["rows"].append(enriched)

    groups = []
    for key, group in grouped.items():
        base = json.loads(key)
        count = group["count"]
        groups.append(
            {
                **base,
                "count": count,
                "sumResidual": group["sumResidual"],
                "avgResidual": group["sumResidual"] / count,
                "avgDw": group["sumDw"] / count,
                "avgDh": group["sumDh"] / count,
                "avgDx": group["sumDx"] / count,
                "avgDy": group["sumDy"] / count,
                "rows": group["rows"],
            }
        )
    groups.sort(key=lambda item: item["sumResidual"], reverse=True)

    output = {"rows": detailed_rows, "groups": groups}
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
