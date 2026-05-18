from __future__ import annotations

import argparse
import json
import re
import shutil
import zipfile
from io import BytesIO
from pathlib import Path
from typing import Any
import xml.etree.ElementTree as ET

import olefile
from pycdxml.cdxml_converter.chemdraw_io import read_cdx, write_cdxml_file


PREFERRED_STREAMS = ["CONTENTS", "OlePres000", "OlePres001", "Package"]
NS = {
    "p": "http://schemas.openxmlformats.org/presentationml/2006/main",
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
}


def natural_sort_key(path_like: str) -> list[object]:
    parts = re.split(r"(\d+)", path_like or "")
    out: list[object] = []
    for part in parts:
        if part.isdigit():
            out.append(int(part))
        else:
            out.append(part.lower())
    return out


def list_streams(ole: olefile.OleFileIO) -> list[str]:
    return ["/".join(parts) for parts in ole.listdir()]


def read_best_stream(ole: olefile.OleFileIO) -> tuple[str | None, bytes | None]:
    streams = list_streams(ole)
    for name in PREFERRED_STREAMS:
        if name in streams:
            with ole.openstream(name) as fp:
                return name, fp.read()
    candidates: list[tuple[str, bytes]] = []
    for stream in streams:
        try:
            with ole.openstream(stream) as fp:
                data = fp.read()
        except Exception:
            continue
        if len(data) >= 4 and data[:4] == b"VjCD":
            candidates.append((stream, data))
    if candidates:
        candidates.sort(key=lambda item: len(item[1]), reverse=True)
        return candidates[0]
    return None, None


def rel_target(base: str, target: str) -> str:
    if target.startswith("/"):
        return target.lstrip("/")
    base_parts = base.split("/")[:-1]
    for part in target.split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            if base_parts:
                base_parts.pop()
            continue
        base_parts.append(part)
    return "/".join(base_parts)


def extract_preview_rel(ole_obj: ET.Element, slide_rels: dict[str, dict[str, str]]) -> tuple[str | None, str | None]:
    pic = ole_obj.find("p:pic", NS)
    if pic is None:
        return None, None
    blip = pic.find(".//a:blip", NS)
    if blip is None:
        return None, None
    rid = blip.attrib.get(f"{{{NS['r']}}}embed")
    if not rid:
        return None, None
    rel = slide_rels.get(rid)
    if not rel:
        return rid, None
    return rid, rel_target("ppt/slides/slide.xml", rel["Target"])


def extract_entries_from_pptx(pptx_path: Path, out_dir: Path) -> dict[str, Any]:
    out_dir.mkdir(parents=True, exist_ok=True)
    media_dir = out_dir / "media"
    cdx_dir = out_dir / "cdx"
    cdxml_dir = out_dir / "cdxml"
    media_dir.mkdir(exist_ok=True)
    cdx_dir.mkdir(exist_ok=True)
    cdxml_dir.mkdir(exist_ok=True)

    manifest: dict[str, Any] = {
        "pptx": str(pptx_path),
        "entries": [],
    }

    media_cache: dict[str, str] = {}
    dedup_entries: dict[tuple[str, str | None, str], dict[str, Any]] = {}

    with zipfile.ZipFile(pptx_path) as z:
        slide_paths = sorted(
            (
                name
                for name in z.namelist()
                if name.startswith("ppt/slides/slide")
                and name.endswith(".xml")
                and "/_rels/" not in name
            ),
            key=natural_sort_key,
        )

        for slide_path in slide_paths:
            root = ET.fromstring(z.read(slide_path))
            rels_path = slide_path.replace("ppt/slides/", "ppt/slides/_rels/") + ".rels"
            slide_rels: dict[str, dict[str, str]] = {}
            if rels_path in z.namelist():
                rel_root = ET.fromstring(z.read(rels_path))
                for rel in rel_root:
                    rid = rel.attrib.get("Id")
                    if rid:
                        slide_rels[rid] = rel.attrib

            for ole in root.findall(".//p:oleObj", NS):
                embed = ole.find("p:embed", NS)
                if embed is None:
                    continue
                rid = ole.attrib.get(f"{{{NS['r']}}}id")
                rel = slide_rels.get(rid or "")
                if not rel:
                    continue
                target = rel_target(slide_path, rel["Target"])
                if target not in z.namelist():
                    continue

                stream_name = None
                cdx_bytes = None
                with olefile.OleFileIO(BytesIO(z.read(target))) as ole_bin:
                    stream_name, cdx_bytes = read_best_stream(ole_bin)
                if not cdx_bytes:
                    continue

                stem = f"{pptx_path.stem}.{Path(slide_path).stem}.{Path(target).stem}"
                cdx_path = cdx_dir / f"{stem}.cdx"
                cdxml_path = cdxml_dir / f"{stem}.cdxml"
                cdx_path.write_bytes(cdx_bytes)

                try:
                    doc = read_cdx(str(cdx_path))
                    write_cdxml_file(doc, str(cdxml_path))
                    cdxml_rel = str(cdxml_path.relative_to(out_dir))
                except Exception as exc:
                    cdxml_rel = None
                    (cdxml_dir / f"{stem}.error.txt").write_text(repr(exc), encoding="utf-8")

                preview_rid, preview_target = extract_preview_rel(ole, slide_rels)
                preview_rel = None
                if preview_target and preview_target in z.namelist():
                    if preview_target not in media_cache:
                        dst = media_dir / Path(preview_target).name
                        dst.write_bytes(z.read(preview_target))
                        media_cache[preview_target] = str(dst.relative_to(out_dir))
                    preview_rel = media_cache[preview_target]

                entry = {
                    "slide": slide_path,
                    "oleRid": rid,
                    "progId": ole.attrib.get("progId"),
                    "imgW": int(ole.attrib.get("imgW", "0")),
                    "imgH": int(ole.attrib.get("imgH", "0")),
                    "binTarget": target,
                    "oleStream": stream_name,
                    "cdx": str(cdx_path.relative_to(out_dir)),
                    "cdxml": cdxml_rel,
                    "previewRid": preview_rid,
                    "preview": preview_rel,
                }
                key = (slide_path, rid, target)
                existing = dedup_entries.get(key)
                if existing is None:
                    dedup_entries[key] = entry
                elif existing.get("preview") is None and entry.get("preview") is not None:
                    dedup_entries[key] = entry

    manifest["entries"] = sorted(
        dedup_entries.values(),
        key=lambda item: (
            natural_sort_key(item["slide"]),
            natural_sort_key(item["binTarget"]),
        ),
    )
    manifest["entryCount"] = len(manifest["entries"])
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract ChemDraw OLE objects from a PPT/PPTX and convert CDX -> CDXML.")
    parser.add_argument("pptx", help="Path to a .ppt or .pptx file")
    parser.add_argument("--out", required=True, help="Output directory")
    args = parser.parse_args()

    pptx_path = Path(args.pptx).resolve()
    out_dir = Path(args.out).resolve()
    manifest = extract_entries_from_pptx(pptx_path, out_dir)
    (out_dir / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"outDir": str(out_dir), "entryCount": manifest["entryCount"]}, ensure_ascii=False))


if __name__ == "__main__":
    main()
