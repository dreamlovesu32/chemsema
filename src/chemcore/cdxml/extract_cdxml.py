from __future__ import annotations
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Sequence, Any
import re
from dataclasses import dataclass, field

from .cdxml_layout import (
    attach_texts_near_molecules as _layout_attach_texts_near_molecules,
    extract_arrows as _layout_extract_arrows,
    extract_texts_non_substituent as _layout_extract_texts_non_substituent,
    split_condition_and_note as _layout_split_condition_and_note,
    split_tables_by_entry as _layout_split_tables_by_entry,
)
from .cdxml_molecule import extract_molecules_smiles_and_centers as _molecule_extract_molecules_smiles_and_centers
from .cdxml_sdf_match import enrich_molecules_with_sdf as _sdf_enrich_molecules_with_sdf


def extract_molecules_smiles_and_centers(
    cdxml_path: str,
) -> List[Tuple[str, Tuple[float, float], Tuple[float, float], Optional[str]]]:
    return _molecule_extract_molecules_smiles_and_centers(cdxml_path)


def extract_texts_non_substituent(
    cdxml_path: str,
) -> List[Tuple[str, Tuple[float, float]]]:
    return _layout_extract_texts_non_substituent(cdxml_path)


def extract_arrows(cdxml_path: str) -> List[dict]:
    return _layout_extract_arrows(cdxml_path)


TextPt = Tuple[str, Tuple[float, float]]  # (text, (x, y))


_SUBSCRIPT_JOIN_RE = re.compile(r"\b([A-Z][A-Za-z]+)\s*\n\s*([0-9]+)")
_ALNUM_WS_RE = re.compile(r"([A-Za-z0-9])([\t\r ]+)([A-Za-z0-9])")
_PAREN_NUM_JOIN_RE = re.compile(r"(\))\s*\n\s*([0-9]+)")
_PAREN_ONLY_LINE_RE = re.compile(r"^[\s]*[\(（].*[\)）][\s]*$")
_QUESTION_LINE_RE = re.compile(r"^[\s]*\?[\s]*$")


def _normalize_text_block(s: str) -> str:
    """
    Normalize text blocks where subscript digits are split onto a new line.
    Example: "PF\\n6" -> "PF6", "CH\\n3" -> "CH3"
    """
    if not s:
        return s
    prev = None
    cur = s
    while prev != cur:
        prev = cur
        cur = _SUBSCRIPT_JOIN_RE.sub(r"\1\2", cur)
        cur = _PAREN_NUM_JOIN_RE.sub(r"\1\2", cur)
        def _fix(m: re.Match) -> str:
            ws = m.group(2)
            if "\t" in ws or "\r" in ws:
                if m.group(1).isdigit() and m.group(3).isdigit():
                    return f"{m.group(1)} {m.group(3)}"
                return f"{m.group(1)}{m.group(3)}"
            return f"{m.group(1)} {m.group(3)}"
        cur = _ALNUM_WS_RE.sub(_fix, cur)

    # drop standalone "?" lines injected by CDXML
    lines = [ln.strip() for ln in cur.splitlines() if not _QUESTION_LINE_RE.match(ln)]
    lines = [ln for ln in lines if ln]
    if not lines:
        return ""

    # merge internal line breaks within a single text block
    out = [lines[0]]
    for ln in lines[1:]:
        prev_ln = out[-1]
        if _PAREN_ONLY_LINE_RE.match(ln):
            out[-1] = prev_ln.rstrip() + " " + ln
            continue
        if (len(prev_ln) <= 2) or (len(ln) <= 2):
            out[-1] = prev_ln + ln
            continue
        if prev_ln.endswith(")") and ln[:1].isdigit():
            out[-1] = prev_ln + ln
            continue
        out.append(ln)
    return "\n".join(out)


def _merge_paren_only_lines(s: str) -> str:
    if not s:
        return s
    lines = s.splitlines()
    out: List[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped and _PAREN_ONLY_LINE_RE.match(stripped) and out:
            out[-1] = out[-1].rstrip() + " " + stripped
        else:
            out.append(line)
    return "\n".join(out)


def split_tables_by_entry(
    items: List[TextPt],
    y_tol: float = 2.5,
    x_merge_tol: float = 10.0,
) -> Tuple[
    List[List[List[str]]], List[Tuple[float, float, float, float]], List[TextPt]
]:
    return _layout_split_tables_by_entry(items, y_tol=y_tol, x_merge_tol=x_merge_tol)


MolRec = Tuple[
    str,
    Tuple[float, float],
    Tuple[float, float],
    Optional[str],
]  # (smi, (cx,cy), (w,h), cdxml_molblock)


def attach_texts_near_molecules(
    mols: Sequence[MolRec],
    remaining: Sequence[TextPt],
    x_factor: float = 0.8,
    inclusive: bool = True,
) -> Tuple[
    List[Tuple[str, Tuple[float, float], Tuple[float, float], str, Optional[str]]],
    List[TextPt],
]:
    return _layout_attach_texts_near_molecules(
        mols,
        remaining,
        x_factor=x_factor,
        inclusive=inclusive,
    )


def split_condition_and_note(
    arrows: List[Dict],
    table_bboxes: List[
        Tuple[float, float, float, float]
    ],  # list of (xmin,ymin,xmax,ymax)
    remaining: List[TextPt],
    x_expand: float = 0.20,
) -> Tuple[str, str]:
    return _layout_split_condition_and_note(
        arrows,
        table_bboxes,
        remaining,
        x_expand=x_expand,
    )


MolMatchRec = Tuple[
    str, Tuple[float, float], Tuple[float, float], str, Optional[str]
]  # (smi,(cx,cy),(w,h),label,cdxml_molblock)


def _resolve_sdf_path_for_matching(cdxml_path: str) -> Path:
    return Path(f"{Path(cdxml_path)}.sdf")


def enrich_molecules_with_sdf(
    mol_match: Sequence[MolMatchRec],
    cdxml_path: str,
    verbose: bool = True,
    keep_molblock2d: bool = True,
) -> List[Dict[str, Any]]:
    return _sdf_enrich_molecules_with_sdf(
        mol_match,
        cdxml_path,
        verbose=verbose,
        keep_molblock2d=keep_molblock2d,
    )


@dataclass
class ExtractResult:
    molecules: List[Dict[str, Any]]
    form: List[List[List[str]]]
    arrows: List[Dict[str, Any]]
    condition: str
    note: str
    texts: List[Tuple[str, Tuple[float, float]]] = field(default_factory=list)


def extract_cdxml(cdxml_path: str) -> ExtractResult:
    cdxml_path = str(Path(cdxml_path))

    mol_infos = extract_molecules_smiles_and_centers(cdxml_path)
    texts = _layout_extract_texts_non_substituent(cdxml_path)
    form, bboxes, remain = _layout_split_tables_by_entry(texts)
    mol_match, remain2 = _layout_attach_texts_near_molecules(mol_infos, remain)
    arrows = _layout_extract_arrows(cdxml_path)
    condition, note = _layout_split_condition_and_note(arrows, bboxes, remain2)
    out = _sdf_enrich_molecules_with_sdf(mol_match, cdxml_path)

    return ExtractResult(
        molecules=out,
        form=form,
        arrows=arrows,
        condition=condition,
        note=note,
        texts=remain2,
    )
    
# if __name__=="__main__":
#     extract_cdxml("/home/jiajun/chemrecords/backend/importers/tmp/oleObject3")
