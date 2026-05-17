from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def summarize(rows: list[dict], key_fn) -> list[dict]:
    groups: dict[str, dict] = defaultdict(lambda: {"count": 0, "residual": 0})
    for row in rows:
        key = key_fn(row)
        groups[key]["count"] += 1
        groups[key]["residual"] += int(row.get("residualCount", 0))
    result = []
    for key, value in groups.items():
        result.append(
            {
                "key": key,
                "count": value["count"],
                "residual": value["residual"],
                "avgResidual": value["residual"] / value["count"] if value["count"] else 0.0,
            }
        )
    result.sort(key=lambda item: item["residual"], reverse=True)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize label residual attribution by label families.")
    parser.add_argument("input_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    obj = json.loads(Path(args.input_json).read_text(encoding="utf-8"))
    rows = obj["labels"]
    summary = {
        "topByResidual": rows[:20],
        "byText": summarize(rows, lambda row: row.get("text", "")),
        "byFill": summarize(rows, lambda row: row.get("fill", "")),
        "byLayout": summarize(rows, lambda row: row.get("layout", "")),
        "byAnchor": summarize(rows, lambda row: row.get("anchor", "")),
        "byAttachment": summarize(rows, lambda row: row.get("attachment", "")),
        "byLayoutAnchor": summarize(
            rows, lambda row: f"{row.get('layout', '')}|{row.get('anchor', '')}"
        ),
        "byTextFill": summarize(
            rows, lambda row: f"{row.get('text', '')}|{row.get('fill', '')}"
        ),
        "byTextLayout": summarize(
            rows, lambda row: f"{row.get('text', '')}|{row.get('layout', '')}"
        ),
    }
    Path(args.output_json).write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(summary, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
