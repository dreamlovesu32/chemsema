from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def point_in_box(point: tuple[float, float], box: list[int]) -> bool:
    x, y = point
    x1, y1, x2, y2 = box
    return x1 <= x <= x2 and y1 <= y <= y2


def center(box: list[int]) -> tuple[float, float]:
    return ((box[0] + box[2]) / 2.0, (box[1] + box[3]) / 2.0)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize same-shell label residuals by molecule component and label family."
    )
    parser.add_argument("label_attribution_json")
    parser.add_argument("label_box_compare_json")
    parser.add_argument("molecule_partition_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    label_attr = json.loads(Path(args.label_attribution_json).read_text(encoding="utf-8"))["labels"]
    label_box = {
        row["nodeId"]: row
        for row in json.loads(Path(args.label_box_compare_json).read_text(encoding="utf-8"))["rows"]
    }
    components = json.loads(Path(args.molecule_partition_json).read_text(encoding="utf-8"))["componentBreakdown"]

    component_rows: dict[str, list[dict]] = defaultdict(list)
    unmatched: list[dict] = []
    for row in label_attr:
        c = center(row["pixelBox"])
        assigned = None
        for comp in components:
            if point_in_box(c, comp["pixelBox"]):
                assigned = comp["name"]
                break
        merged = dict(row)
        if row["nodeId"] in label_box:
            merged.update(
                {
                    "deltaDims": label_box[row["nodeId"]]["deltaDims"],
                    "deltaTopLeft": label_box[row["nodeId"]]["deltaTopLeft"],
                    "oursDims": label_box[row["nodeId"]]["oursDims"],
                    "refDims": label_box[row["nodeId"]]["refDims"],
                }
            )
        if assigned is None:
            unmatched.append(merged)
        else:
            component_rows[assigned].append(merged)

    component_summary = []
    for comp in components:
        name = comp["name"]
        rows = component_rows.get(name, [])
        by_key = defaultdict(
            lambda: {
                "count": 0,
                "sumResidual": 0,
                "sumDw": 0,
                "sumDh": 0,
                "sumDx": 0,
                "sumDy": 0,
            }
        )
        for row in rows:
            key = f"{row['text']}|{row.get('fill')}"
            g = by_key[key]
            g["count"] += 1
            g["sumResidual"] += row["residualCount"]
            dw, dh = row.get("deltaDims", [0, 0])
            dx, dy = row.get("deltaTopLeft", [0, 0])
            g["sumDw"] += dw
            g["sumDh"] += dh
            g["sumDx"] += dx
            g["sumDy"] += dy

        families = []
        for key, g in by_key.items():
            count = g["count"]
            families.append(
                {
                    "key": key,
                    "count": count,
                    "sumResidual": g["sumResidual"],
                    "avgResidual": g["sumResidual"] / count,
                    "avgDw": g["sumDw"] / count,
                    "avgDh": g["sumDh"] / count,
                    "avgDx": g["sumDx"] / count,
                    "avgDy": g["sumDy"] / count,
                }
            )
        families.sort(key=lambda item: item["sumResidual"], reverse=True)
        component_summary.append(
            {
                "name": name,
                "componentResidualCount": comp["residualCount"],
                "labelResidualCount": comp["labelResidualCount"],
                "nonLabelResidualCount": comp["nonLabelResidualCount"],
                "labelRows": rows,
                "families": families,
            }
        )

    component_summary.sort(key=lambda item: item["componentResidualCount"], reverse=True)
    output = {
        "componentSummary": component_summary,
        "unmatchedCount": len(unmatched),
        "unmatched": unmatched,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
