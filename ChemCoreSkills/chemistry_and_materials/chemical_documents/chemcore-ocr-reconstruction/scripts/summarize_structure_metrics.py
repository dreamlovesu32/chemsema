#!/usr/bin/env python3
"""Summarize ChemCore OCR structure metrics under a directory."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def status_of(data: dict[str, Any]) -> str:
    if "structure_status" in data:
        return str(data["structure_status"])
    if data.get("structure_isomorphic") is True:
        return "pass"
    if data.get("ok") is False:
        return "fail"
    return "unknown"


def taxonomy_of(data: dict[str, Any]) -> list[str]:
    for key in ("failure_taxonomy", "failureTaxonomy", "failure_types", "failureTypes"):
        value = data.get(key)
        if isinstance(value, list):
            return [str(item) for item in value]
    diffs = data.get("differences")
    if isinstance(diffs, list):
        tags: list[str] = []
        for diff in diffs:
            if isinstance(diff, dict):
                for key in ("taxonomy", "kind", "type"):
                    if key in diff:
                        tags.append(str(diff[key]))
                        break
            else:
                tags.append(str(diff))
        return tags
    return []


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", help="Directory containing structure-metrics.json files.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    args = parser.parse_args()

    paths = sorted(Path(args.root).rglob("structure-metrics.json"))
    statuses: Counter[str] = Counter()
    taxonomy: Counter[str] = Counter()
    rows: list[dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        status = status_of(data if isinstance(data, dict) else {})
        tags = taxonomy_of(data if isinstance(data, dict) else {})
        statuses[status] += 1
        taxonomy.update(tags or ["unclassified"])
        rows.append({"path": str(path), "status": status, "taxonomy": tags})

    summary = {
        "root": str(Path(args.root).resolve()),
        "count": len(paths),
        "statuses": dict(statuses),
        "taxonomy": dict(taxonomy),
        "items": rows,
    }

    if args.json:
        print(json.dumps(summary, ensure_ascii=False, indent=2))
    else:
        print(f"metrics: {summary['count']}")
        print("statuses:")
        for key, value in statuses.most_common():
            print(f"  {key}: {value}")
        print("taxonomy:")
        for key, value in taxonomy.most_common():
            print(f"  {key}: {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
