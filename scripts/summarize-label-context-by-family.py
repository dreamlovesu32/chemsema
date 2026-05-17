from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def sign3(value: float) -> str:
    if value < -1e-6:
        return "neg"
    if value > 1e-6:
        return "pos"
    return "zero"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize same-shell label residuals by family and ChemDraw/imported context."
    )
    parser.add_argument("payload_json")
    parser.add_argument("component_summary_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    payload = json.loads(Path(args.payload_json).read_text(encoding="utf-8"))
    doc = json.loads(payload["chemcoreDocumentJson"])
    frag = doc["resources"]["mol_cdxml_merged"]["data"]
    nodes = {node["id"]: node for node in frag["nodes"]}

    component_summary = json.loads(Path(args.component_summary_json).read_text(encoding="utf-8"))["componentSummary"]

    output_rows = []
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

    for comp in component_summary:
        rows = comp["labelRows"]
        if not rows:
            continue
        xs = []
        ys = []
        for row in rows:
            box = nodes[row["nodeId"]]["label"]["box"]
            xs.extend([box[0], box[2]])
            ys.extend([box[1], box[3]])
        cx = (min(xs) + max(xs)) / 2.0
        cy = (min(ys) + max(ys)) / 2.0

        for row in rows:
            node = nodes[row["nodeId"]]
            label = node["label"]
            box = label["box"]
            lx = (box[0] + box[2]) / 2.0
            ly = (box[1] + box[3]) / 2.0
            cdxml_meta = label.get("meta", {}).get("import", {}).get("cdxml", {})
            family = f"{row['text']}|{row.get('fill')}"
            context_key = {
                "component": comp["name"],
                "family": family,
                "cdxmlLabelAlignment": cdxml_meta.get("labelAlignment"),
                "cdxmlLabelJustification": cdxml_meta.get("labelJustification"),
                "nodeType": node.get("meta", {}).get("import", {}).get("cdxml", {}).get("nodeType"),
                "sideX": sign3(lx - cx),
                "sideY": sign3(ly - cy),
            }
            key = json.dumps(context_key, sort_keys=True, ensure_ascii=False)
            g = grouped[key]
            g["count"] += 1
            g["sumResidual"] += row["residualCount"]
            dw, dh = row.get("deltaDims", [0, 0])
            dx, dy = row.get("deltaTopLeft", [0, 0])
            g["sumDw"] += dw
            g["sumDh"] += dh
            g["sumDx"] += dx
            g["sumDy"] += dy
            merged_row = dict(row)
            merged_row["component"] = comp["name"]
            merged_row["cdxmlLabelAlignment"] = cdxml_meta.get("labelAlignment")
            merged_row["cdxmlLabelJustification"] = cdxml_meta.get("labelJustification")
            merged_row["nodeType"] = node.get("meta", {}).get("import", {}).get("cdxml", {}).get("nodeType")
            merged_row["sideX"] = sign3(lx - cx)
            merged_row["sideY"] = sign3(ly - cy)
            g["rows"].append(merged_row)
            output_rows.append(merged_row)

    groups = []
    for key, g in grouped.items():
        context = json.loads(key)
        count = g["count"]
        groups.append(
            {
                **context,
                "count": count,
                "sumResidual": g["sumResidual"],
                "avgResidual": g["sumResidual"] / count,
                "avgDw": g["sumDw"] / count,
                "avgDh": g["sumDh"] / count,
                "avgDx": g["sumDx"] / count,
                "avgDy": g["sumDy"] / count,
                "rows": g["rows"],
            }
        )
    groups.sort(key=lambda item: item["sumResidual"], reverse=True)

    output = {"rows": output_rows, "groups": groups}
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
