from __future__ import annotations

import argparse
import itertools
import json
from pathlib import Path


DEFAULT_CATEGORICAL_FIELDS = [
    "text",
    "fill",
    "componentQuadrant",
    "cdxmlLabelJustification",
    "primaryNeighborBucket",
]

DEFAULT_NUMERIC_FIELDS = [
    "gapRight",
    "topPagePhase",
    "xPagePhase",
    "baselineTopPxPhase",
]

ACTION_TO_FIELD = {
    "-2": "globalDelta_top-2",
    "-1": "globalDelta_top-1",
    "+1": "globalDelta_top1",
    "+2": "globalDelta_top2",
}


def thresholds(values: list[float]) -> list[float]:
    vals = sorted(set(values))
    mids = [(a + b) * 0.5 for a, b in zip(vals, vals[1:])]
    return sorted(set(vals + mids))


def load_rows(top_json: Path, phase_json: Path, action: str) -> list[dict]:
    top_rows = {row["nodeId"]: row for row in json.loads(top_json.read_text(encoding="utf-8"))}
    phase_rows = json.loads(phase_json.read_text(encoding="utf-8"))["rows"]
    delta_field = ACTION_TO_FIELD[action]
    rows: list[dict] = []
    for phase_row in phase_rows:
        node_id = phase_row["nodeId"]
        if node_id not in top_rows:
            continue
        merged = dict(phase_row)
        merged["globalDelta"] = float(top_rows[node_id].get(delta_field) or 0.0)
        rows.append(merged)
    return rows


def build_conditions(rows: list[dict], categorical_fields: list[str], numeric_fields: list[str]):
    conditions: list[tuple[str, object]] = []
    for field in categorical_fields:
        values = sorted({row.get(field) for row in rows if row.get(field) is not None})
        for value in values:
            conditions.append(
                (
                    f"{field}=={value}",
                    lambda row, field=field, value=value: row.get(field) == value,
                )
            )
    for field in numeric_fields:
        values = [float(row[field]) for row in rows if row.get(field) is not None]
        for threshold in thresholds(values):
            conditions.append(
                (
                    f"{field}<={threshold:.6f}",
                    lambda row, field=field, threshold=threshold: row.get(field) is not None
                    and float(row[field]) <= threshold + 1e-9,
                )
            )
            conditions.append(
                (
                    f"{field}>={threshold:.6f}",
                    lambda row, field=field, threshold=threshold: row.get(field) is not None
                    and float(row[field]) >= threshold - 1e-9,
                )
            )
    return conditions


def score_subset(subset: list[dict]) -> dict[str, float]:
    total = float(sum(float(row["globalDelta"]) for row in subset))
    neg = float(sum(-min(0.0, float(row["globalDelta"])) for row in subset))
    pos = float(sum(max(0.0, float(row["globalDelta"])) for row in subset))
    return {"total": total, "neg": neg, "pos": pos}


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Search simple geometric predicates for attached-label top-nudge families."
    )
    parser.add_argument("top_json")
    parser.add_argument("phase_json")
    parser.add_argument("--action", required=True, choices=sorted(ACTION_TO_FIELD))
    parser.add_argument("--output", required=True)
    parser.add_argument("--max-combo", type=int, default=3)
    parser.add_argument("--categorical-fields", default=",".join(DEFAULT_CATEGORICAL_FIELDS))
    parser.add_argument("--numeric-fields", default=",".join(DEFAULT_NUMERIC_FIELDS))
    parser.add_argument("--top", type=int, default=30)
    args = parser.parse_args()

    top_json = Path(args.top_json)
    phase_json = Path(args.phase_json)
    output_json = Path(args.output)

    categorical_fields = [field for field in args.categorical_fields.split(",") if field]
    numeric_fields = [field for field in args.numeric_fields.split(",") if field]

    rows = load_rows(top_json, phase_json, args.action)
    conditions = build_conditions(rows, categorical_fields, numeric_fields)

    seen_subsets: set[tuple[str, ...]] = set()
    evaluated: list[dict] = []
    for combo_size in range(1, args.max_combo + 1):
        for combo in itertools.combinations(range(len(conditions)), combo_size):
            subset = [row for row in rows if all(conditions[index][1](row) for index in combo)]
            if not subset:
                continue
            node_ids = tuple(sorted(str(row["nodeId"]) for row in subset))
            if node_ids in seen_subsets:
                continue
            seen_subsets.add(node_ids)
            score = score_subset(subset)
            evaluated.append(
                {
                    "conditions": [conditions[index][0] for index in combo],
                    "count": len(subset),
                    "nodeIds": list(node_ids),
                    **score,
                }
            )

    top_total = sorted(
        evaluated,
        key=lambda item: (item["total"], -item["neg"], item["pos"], item["count"]),
        reverse=True,
    )[: args.top]

    safe = [item for item in evaluated if item["neg"] < 1e-12 and item["total"] > 0]
    safe.sort(key=lambda item: (item["total"], -item["count"]), reverse=True)

    best_safe_total = safe[0]["total"] if safe else 0.0
    best_safe_min_count = sorted(
        [item for item in safe if abs(item["total"] - best_safe_total) < 1e-12],
        key=lambda item: (item["count"], len(item["conditions"])),
    )[: args.top]

    output = {
        "top_json": str(top_json.resolve()),
        "phase_json": str(phase_json.resolve()),
        "action": args.action,
        "categorical_fields": categorical_fields,
        "numeric_fields": numeric_fields,
        "max_combo": args.max_combo,
        "top_total": top_total,
        "best_safe_min_count": best_safe_min_count,
        "safe_top": safe[: args.top],
    }
    output_json.write_text(json.dumps(output, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
