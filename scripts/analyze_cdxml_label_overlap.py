from __future__ import annotations

import sys
from pathlib import Path

from chemcore.cdxml.cdxml_fragment_display import extract_display_fragments


def overlap_area(a: list[float], b: list[float]) -> float:
    x1 = max(float(a[0]), float(b[0]))
    y1 = max(float(a[1]), float(b[1]))
    x2 = min(float(a[2]), float(b[2]))
    y2 = min(float(a[3]), float(b[3]))
    if x2 <= x1 or y2 <= y1:
        return 0.0
    return (x2 - x1) * (y2 - y1)


def analyze(cdxml_base: str) -> None:
    fragments = extract_display_fragments(cdxml_base, table_bboxes=[])
    for fragment in fragments:
        labels = []
        for node in fragment.get("nodes", []):
            label = node.get("label") or {}
            bbox = label.get("bbox")
            text = label.get("text")
            if bbox and text:
                labels.append((node["id"], text, bbox))

        hits = []
        for i in range(len(labels)):
            for j in range(i + 1, len(labels)):
                left = labels[i]
                right = labels[j]
                area = overlap_area(left[2], right[2])
                if area > 0:
                    hits.append((area, left, right))

        if not hits:
            continue

        print(f"fragment {fragment.get('id')}:")
        for area, left, right in sorted(hits, key=lambda item: item[0], reverse=True):
            print(
                f"  overlap={area:.2f} "
                f"{left[0]}:{left[1]} {left[2]} "
                f"<-> {right[0]}:{right[1]} {right[2]}"
            )


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: analyze_cdxml_label_overlap.py /path/to/oleObject4", file=sys.stderr)
        return 2
    analyze(str(Path(argv[1])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
