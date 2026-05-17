from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Summarize current-vs-no-knockout label deltas by molecule component and label family, "
            "optionally merging same-shell residual context."
        )
    )
    parser.add_argument("component_summary_json")
    parser.add_argument("current_vs_noknockout_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    component_summary = json.loads(Path(args.component_summary_json).read_text(encoding="utf-8"))[
        "componentSummary"
    ]
    current_vs_nok = json.loads(Path(args.current_vs_noknockout_json).read_text(encoding="utf-8"))["rows"]

    node_to_component: dict[str, str] = {}
    node_to_residual: dict[str, int] = {}
    node_to_family: dict[str, str] = {}
    for comp in component_summary:
        for row in comp["labelRows"]:
            node_id = row["nodeId"]
            node_to_component[node_id] = comp["name"]
            node_to_residual[node_id] = row["residualCount"]
            node_to_family[node_id] = f"{row['text']}|{row.get('fill')}"

    component_family = defaultdict(
        lambda: {
            "count": 0,
            "sumResidual": 0,
            "sumKnockoutDw": 0,
            "sumKnockoutDh": 0,
            "sumKnockoutDx": 0,
            "sumKnockoutDy": 0,
            "rows": [],
        }
    )
    unmatched: list[dict] = []

    for row in current_vs_nok:
        node_id = row["nodeId"]
        component = node_to_component.get(node_id)
        if component is None:
            unmatched.append(row)
            continue
        family = node_to_family[node_id]
        g = component_family[(component, family)]
        g["count"] += 1
        residual = node_to_residual.get(node_id, row.get("residualCount", 0))
        g["sumResidual"] += residual

        # current_vs_noknockout rows are encoded as:
        # ours = no-knockout, ref = current
        # so knockout contribution = ref - ours = -(delta*)
        dw, dh = row.get("deltaDims", [0, 0])
        dx, dy = row.get("deltaTopLeft", [0, 0])
        g["sumKnockoutDw"] += -dw
        g["sumKnockoutDh"] += -dh
        g["sumKnockoutDx"] += -dx
        g["sumKnockoutDy"] += -dy
        g["rows"].append(
            {
                "nodeId": node_id,
                "residualCount": residual,
                "knockoutContribution": {
                    "dw": -dw,
                    "dh": -dh,
                    "dx": -dx,
                    "dy": -dy,
                },
                "currentDims": row.get("refDims"),
                "noKnockoutDims": row.get("oursDims"),
            }
        )

    grouped_components = []
    component_to_families = defaultdict(list)
    for (component, family), g in component_family.items():
        count = g["count"]
        component_to_families[component].append(
            {
                "key": family,
                "count": count,
                "sumResidual": g["sumResidual"],
                "avgResidual": g["sumResidual"] / count,
                "avgKnockoutDw": g["sumKnockoutDw"] / count,
                "avgKnockoutDh": g["sumKnockoutDh"] / count,
                "avgKnockoutDx": g["sumKnockoutDx"] / count,
                "avgKnockoutDy": g["sumKnockoutDy"] / count,
                "rows": g["rows"],
            }
        )

    summary_lookup = {comp["name"]: comp for comp in component_summary}
    for component, families in component_to_families.items():
        families.sort(key=lambda item: item["sumResidual"], reverse=True)
        comp = summary_lookup[component]
        grouped_components.append(
            {
                "name": component,
                "componentResidualCount": comp["componentResidualCount"],
                "labelResidualCount": comp["labelResidualCount"],
                "nonLabelResidualCount": comp["nonLabelResidualCount"],
                "families": families,
            }
        )

    grouped_components.sort(key=lambda item: item["componentResidualCount"], reverse=True)
    output = {
        "components": grouped_components,
        "unmatchedCount": len(unmatched),
        "unmatched": unmatched,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
