from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


SPECIAL_FILES = {
    "f4_32333": {
        "m1": "attached-ynode2-posdy1-m1-fg3-label-iou.json",
        "m2": "attached-ynode2-familyA-m2-fg3-label-iou.json",
    },
    "f4_32335": {
        "m1": "attached-ynode2-posdy1-m1-fg3-label-iou.json",
        "m2": "attached-ynode2-familyA-m2-fg3-label-iou.json",
    },
    "f2_34461": {
        "m1": "node34461-m1-fg3-label-iou.json",
        "m2": "node34461-m2-fg3-label-iou.json",
    },
}


@dataclass(frozen=True)
class Row:
    node_id: str
    text: str
    fill: str | None
    quadrant: str | None
    justification: str | None
    neighbor: str | None
    phase: float
    m1: float
    m2: float


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def label_iou_map(path: Path) -> dict[str, float]:
    return {
        row["nodeId"]: float(row["iou"])
        for row in load_json(path)["rows"]
    }


def candidate_paths(root: Path, node_id: str, shift_tag: str) -> list[Path]:
    paths = [
        root / f"{node_id}-y{shift_tag}-fg3-label-iou.json",
        root / f"{node_id}-{shift_tag}-fg3-label-iou.json",
    ]
    if node_id in SPECIAL_FILES and shift_tag in SPECIAL_FILES[node_id]:
        paths.append(root / SPECIAL_FILES[node_id][shift_tag])
    return paths


def load_rows(phase_json: Path, baseline_json: Path, variants_root: Path) -> list[Row]:
    phase_rows = load_json(phase_json)["rows"]
    baseline = label_iou_map(baseline_json)
    rows: list[Row] = []
    for row in phase_rows:
        if not row.get("component") or not row.get("text"):
            continue
        node_id = row["nodeId"]
        if node_id not in baseline:
            continue
        deltas: dict[str, float] = {}
        for shift_tag in ("m1", "m2"):
            matched = None
            for path in candidate_paths(variants_root, node_id, shift_tag):
                if not path.exists():
                    continue
                data = label_iou_map(path)
                if node_id in data:
                    matched = data[node_id] - baseline[node_id]
                    break
            if matched is None:
                raise FileNotFoundError(
                    f"missing {shift_tag} variant label IoU for {node_id}"
                )
            deltas[shift_tag] = matched
        rows.append(
            Row(
                node_id=node_id,
                text=row.get("text", ""),
                fill=row.get("fill"),
                quadrant=row.get("componentQuadrant"),
                justification=row.get("cdxmlLabelJustification"),
                neighbor=row.get("primaryNeighborBucket"),
                phase=float(row["topPagePhase"]),
                m1=deltas["m1"],
                m2=deltas["m2"],
            )
        )
    return rows


def delta(row: Row, action: str) -> float:
    if action == "m1":
        return row.m1
    if action == "m2":
        return row.m2
    return 0.0


def threshold_values(phases: list[float]) -> list[float]:
    uniq = sorted(set(phases))
    return [(a + b) * 0.5 for a, b in zip(uniq, uniq[1:])]


def predicate_catalog(rows: list[Row]) -> list[tuple[str, callable]]:
    preds: list[tuple[str, callable]] = []
    thresholds = threshold_values([row.phase for row in rows])
    for t in thresholds:
        preds.append((f"phase<{t:.6f}", lambda row, t=t: row.phase < t))
    fields = [
        ("text", "text"),
        ("fill", "fill"),
        ("quadrant", "quadrant"),
        ("justification", "justification"),
        ("neighbor", "neighbor"),
    ]
    for field_name, attr in fields:
        values = sorted({getattr(row, attr) for row in rows})
        for value in values:
            preds.append(
                (
                    f"{field_name}=={value}",
                    lambda row, attr=attr, value=value: getattr(row, attr) == value,
                )
            )
    for field_name, attr in (("fill", "fill"), ("text", "text"), ("quadrant", "quadrant"), ("neighbor", "neighbor")):
        values = sorted({getattr(row, attr) for row in rows})
        for t in thresholds:
            for value in values:
                preds.append(
                    (
                        f"phase<{t:.6f}&{field_name}=={value}",
                        lambda row, t=t, attr=attr, value=value: row.phase < t
                        and getattr(row, attr) == value,
                    )
                )
    return preds


def eval_policy(rows: list[Row], fn) -> tuple[float, list[dict]]:
    total = 0.0
    applied = []
    for row in rows:
        action = fn(row)
        gain = delta(row, action)
        total += gain
        applied.append(
            {
                "nodeId": row.node_id,
                "action": action,
                "gain": gain,
            }
        )
    return total, applied


def best_one_rule(rows: list[Row], preds: list[tuple[str, callable]]) -> list[dict]:
    out = []
    for name, pred in preds:
        for action_true in ("0", "m1", "m2"):
            for action_false in ("0", "m1", "m2"):
                total, applied = eval_policy(
                    rows,
                    lambda row, pred=pred, at=action_true, af=action_false: at
                    if pred(row)
                    else af,
                )
                out.append(
                    {
                        "predicate": name,
                        "trueAction": action_true,
                        "falseAction": action_false,
                        "totalGain": total,
                        "applied": applied,
                    }
                )
    out.sort(key=lambda item: (-item["totalGain"], item["predicate"]))
    return out[:20]


def best_two_rule(rows: list[Row], preds: list[tuple[str, callable]]) -> list[dict]:
    pred_scores = []
    for name, pred in preds:
        best = max(
            eval_policy(
                rows,
                lambda row, pred=pred, at=at, af=af: at if pred(row) else af,
            )[0]
            for at in ("0", "m1", "m2")
            for af in ("0", "m1", "m2")
        )
        pred_scores.append((best, name, pred))
    pred_scores.sort(key=lambda item: -item[0])
    top_preds = pred_scores[:120]

    out = []
    for _, root_name, root_pred in top_preds:
        for split_side in (True, False):
            for _, child_name, child_pred in top_preds:
                for other_action in ("0", "m1", "m2"):
                    for child_true_action in ("0", "m1", "m2"):
                        for child_false_action in ("0", "m1", "m2"):
                            total, applied = eval_policy(
                                rows,
                                lambda row,
                                root_pred=root_pred,
                                split_side=split_side,
                                child_pred=child_pred,
                                other_action=other_action,
                                child_true_action=child_true_action,
                                child_false_action=child_false_action: (
                                    child_true_action
                                    if root_pred(row) == split_side and child_pred(row)
                                    else child_false_action
                                    if root_pred(row) == split_side
                                    else other_action
                                ),
                            )
                            out.append(
                                {
                                    "rootPredicate": root_name,
                                    "splitSide": split_side,
                                    "childPredicate": child_name,
                                    "otherAction": other_action,
                                    "childTrueAction": child_true_action,
                                    "childFalseAction": child_false_action,
                                    "totalGain": total,
                                    "applied": applied,
                                }
                            )
    out.sort(key=lambda item: (-item["totalGain"], item["rootPredicate"]))
    return out[:30]


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Search compact local attached-label replay policies using measured "
            "same-shell label IoU gains for y=-1 and y=-2 variants."
        )
    )
    parser.add_argument("phase_json")
    parser.add_argument("baseline_label_iou_json")
    parser.add_argument("variants_root")
    parser.add_argument("output_json")
    args = parser.parse_args()

    rows = load_rows(
        Path(args.phase_json),
        Path(args.baseline_label_iou_json),
        Path(args.variants_root),
    )
    preds = predicate_catalog(rows)
    payload = {
        "rows": [
            {
                "nodeId": row.node_id,
                "text": row.text,
                "fill": row.fill,
                "quadrant": row.quadrant,
                "justification": row.justification,
                "neighbor": row.neighbor,
                "phase": row.phase,
                "m1Gain": row.m1,
                "m2Gain": row.m2,
            }
            for row in rows
        ],
        "bestOneRule": best_one_rule(rows, preds),
        "bestTwoRule": best_two_rule(rows, preds),
    }
    Path(args.output_json).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
