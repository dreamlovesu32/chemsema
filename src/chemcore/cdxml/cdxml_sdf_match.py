from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

from rdkit import Chem, DataStructs
from rdkit.Chem import rdFingerprintGenerator, rdMolDescriptors


MolMatchRec = Tuple[str, Tuple[float, float], Tuple[float, float], str, Optional[str]]


def _resolve_sdf_path_for_matching(cdxml_path: str) -> Path:
    return Path(f"{Path(cdxml_path)}.sdf")


def enrich_molecules_with_sdf(
    mol_match: Sequence[MolMatchRec],
    cdxml_path: str,
    verbose: bool = True,
    keep_molblock2d: bool = True,
) -> List[Dict[str, Any]]:
    sdf_path = _resolve_sdf_path_for_matching(cdxml_path)
    if not sdf_path.exists():
        raise FileNotFoundError(str(sdf_path))

    def normalized_mol_for_matching(m: Optional[Chem.Mol]) -> Optional[Chem.Mol]:
        if m is None:
            return None
        try:
            m2 = Chem.Mol(m)
        except Exception:
            return None
        try:
            Chem.SanitizeMol(m2)
        except Exception:
            pass
        try:
            Chem.AssignStereochemistry(m2, cleanIt=True, force=True)
        except Exception:
            pass
        return m2

    def key_nostereo_from_mol(m: Chem.Mol) -> Optional[str]:
        m2 = normalized_mol_for_matching(m)
        if m2 is None:
            return None
        try:
            return Chem.MolToSmiles(m2, isomericSmiles=False)
        except Exception:
            return None

    def _is_placeholder_or_alias_mol(m: Optional[Chem.Mol]) -> bool:
        if m is None:
            return True
        try:
            if m.GetNumAtoms() != 1 or m.GetNumBonds() != 0:
                return False
            atom = m.GetAtomWithIdx(0)
            symbol = (atom.GetSymbol() or "").strip()
            if symbol in {"A", "Q", "*", "R", "X"}:
                return True
            if atom.GetAtomicNum() in {0}:
                return True
        except Exception:
            return False
        return False

    def inchikey_nostereo_from_mol(m: Optional[Chem.Mol]) -> Optional[str]:
        m2 = normalized_mol_for_matching(m)
        if m2 is None:
            return None
        if _is_placeholder_or_alias_mol(m2):
            return None
        try:
            Chem.RemoveStereochemistry(m2)
        except Exception:
            pass
        try:
            ik = Chem.MolToInchiKey(m2)
            return ik or None
        except Exception:
            return None

    def mol_from_cdxml_input(smi: str, molblock: Optional[str]) -> Optional[Chem.Mol]:
        if molblock:
            try:
                m2 = Chem.MolFromMolBlock(molblock, sanitize=False, removeHs=False)
            except Exception:
                m2 = None
            if m2 is not None:
                try:
                    Chem.SanitizeMol(m2)
                except Exception:
                    pass
                return m2
        try:
            return Chem.MolFromSmiles(smi)
        except Exception:
            return None

    fp_gen = rdFingerprintGenerator.GetMorganGenerator(radius=2, fpSize=2048)

    def fp_from_mol(m: Optional[Chem.Mol]):
        m2 = normalized_mol_for_matching(m)
        if m2 is None:
            return None
        try:
            return fp_gen.GetFingerprint(m2)
        except Exception:
            return None

    def structure_signature_from_mol(m: Optional[Chem.Mol]) -> Optional[Tuple[str, int, int, int]]:
        m2 = normalized_mol_for_matching(m)
        if m2 is None:
            return None
        try:
            formula = rdMolDescriptors.CalcMolFormula(m2)
        except Exception:
            formula = ""
        try:
            heavy = int(m2.GetNumHeavyAtoms())
        except Exception:
            heavy = -1
        try:
            carbons = sum(1 for a in m2.GetAtoms() if a.GetAtomicNum() == 6)
        except Exception:
            carbons = -1
        try:
            rings = int(rdMolDescriptors.CalcNumRings(m2))
        except Exception:
            rings = -1
        return (formula, heavy, carbons, rings)

    def fp_similarity(cdxml_row: Dict[str, Any], sdf_row: Dict[str, Any]) -> float:
        fp_c = cdxml_row.get("fp")
        fp_s = sdf_row.get("fp")
        if fp_c is None or fp_s is None:
            return 0.0
        try:
            return float(DataStructs.TanimotoSimilarity(fp_c, fp_s))
        except Exception:
            return 0.0

    def same_connectivity(cdxml_row: Dict[str, Any], sdf_row: Dict[str, Any]) -> bool:
        m1 = normalized_mol_for_matching(cdxml_row.get("mol"))
        m2 = normalized_mol_for_matching(sdf_row.get("mol"))
        if m1 is None or m2 is None:
            return False
        try:
            if m1.GetNumHeavyAtoms() != m2.GetNumHeavyAtoms():
                return False
            return bool(m1.HasSubstructMatch(m2, useChirality=False) and m2.HasSubstructMatch(m1, useChirality=False))
        except Exception:
            return False

    def structures_are_compatible(cdxml_row: Dict[str, Any], sdf_row: Dict[str, Any]) -> bool:
        if (
            bool(cdxml_row.get("inchikey_nostereo"))
            and bool(sdf_row.get("inchikey_nostereo"))
            and cdxml_row.get("inchikey_nostereo") == sdf_row.get("inchikey_nostereo")
        ):
            return True
        if (
            bool(cdxml_row.get("key_nostereo"))
            and bool(sdf_row.get("key_nostereo"))
            and cdxml_row.get("key_nostereo") == sdf_row.get("key_nostereo")
        ):
            return True
        if same_connectivity(cdxml_row, sdf_row):
            return True
        c_sig = cdxml_row.get("signature")
        s_sig = sdf_row.get("signature")
        if not c_sig or not s_sig:
            return True
        c_formula, c_heavy, c_carbons, c_rings = c_sig
        s_formula, s_heavy, s_carbons, s_rings = s_sig
        if c_formula and s_formula and c_formula != s_formula:
            return False
        if c_heavy >= 0 and s_heavy >= 0 and c_heavy != s_heavy:
            return False
        if c_carbons >= 0 and s_carbons >= 0 and c_carbons != s_carbons:
            return False
        if c_rings >= 0 and s_rings >= 0 and c_rings != s_rings:
            return False
        return fp_similarity(cdxml_row, sdf_row) >= 0.985

    def layout_sort_key(row: Dict[str, Any]) -> Tuple[float, float, int]:
        center = row.get("center") or (10**9, 10**9)
        x = float(center[0]) if center is not None else float(10**9)
        y = float(center[1]) if center is not None else float(10**9)
        tie = int(row.get("idx") or row.get("sdf_index") or 0)
        return (y, x, tie)

    def hungarian_min_cost(cost: List[List[float]]) -> List[int]:
        n = len(cost)
        m = len(cost[0]) if n else 0
        u = [0.0] * (n + 1)
        v = [0.0] * (m + 1)
        p = [0] * (m + 1)
        way = [0] * (m + 1)
        for i in range(1, n + 1):
            p[0] = i
            j0 = 0
            minv = [float("inf")] * (m + 1)
            used = [False] * (m + 1)
            while True:
                used[j0] = True
                i0 = p[j0]
                delta = float("inf")
                j1 = 0
                for j in range(1, m + 1):
                    if used[j]:
                        continue
                    cur = cost[i0 - 1][j - 1] - u[i0] - v[j]
                    if cur < minv[j]:
                        minv[j] = cur
                        way[j] = j0
                    if minv[j] < delta:
                        delta = minv[j]
                        j1 = j
                for j in range(m + 1):
                    if used[j]:
                        u[p[j]] += delta
                        v[j] -= delta
                    else:
                        minv[j] -= delta
                j0 = j1
                if p[j0] == 0:
                    break
            while True:
                j1 = way[j0]
                p[j0] = p[j1]
                j0 = j1
                if j0 == 0:
                    break
        assignment = [-1] * n
        for j in range(1, m + 1):
            if p[j] != 0:
                assignment[p[j] - 1] = j - 1
        return assignment

    suppl = Chem.SDMolSupplier(str(sdf_path), sanitize=False, removeHs=False, strictParsing=False)
    sdf_records: List[Dict[str, Any]] = []
    sdf_row_id = 0
    for idx, m in enumerate(suppl):
        if m is None:
            continue
        try:
            frags = Chem.GetMolFrags(m, asMols=True, sanitizeFrags=False)
        except Exception:
            frags = ()
        if not frags:
            frags = (m,)
        for frag_no, fm in enumerate(frags, start=1):
            k = key_nostereo_from_mol(fm)
            if not k:
                continue
            fm_norm = normalized_mol_for_matching(fm) or fm
            smi_iso = None
            try:
                smi_iso = Chem.MolToSmiles(fm_norm, isomericSmiles=True)
            except Exception:
                pass
            if smi_iso is None:
                try:
                    smi_iso = Chem.MolToSmiles(fm, isomericSmiles=True)
                except Exception:
                    smi_iso = None

            molblock = None
            if keep_molblock2d:
                try:
                    molblock = Chem.MolToMolBlock(fm)
                except Exception:
                    molblock = None

            smi_iso_from_molblock = None
            if molblock:
                try:
                    m_mb = Chem.MolFromMolBlock(molblock, sanitize=False, removeHs=False)
                except Exception:
                    m_mb = None
                if m_mb is not None:
                    try:
                        Chem.SanitizeMol(m_mb)
                    except Exception:
                        pass
                    try:
                        smi_iso_from_molblock = Chem.MolToSmiles(m_mb, isomericSmiles=True)
                    except Exception:
                        smi_iso_from_molblock = None

            center = None
            try:
                conf = fm.GetConformer()
                if conf and conf.Is3D() is False:
                    xs = [conf.GetAtomPosition(i).x for i in range(fm.GetNumAtoms())]
                    ys = [conf.GetAtomPosition(i).y for i in range(fm.GetNumAtoms())]
                    if xs and ys:
                        center = (sum(xs) / len(xs), sum(ys) / len(ys))
            except Exception:
                center = None

            sdf_row_id += 1
            sdf_records.append(
                {
                    "sdf_index": sdf_row_id,
                    "sdf_record_index": idx,
                    "sdf_fragment_index": frag_no,
                    "key_nostereo": k,
                    "inchikey_nostereo": inchikey_nostereo_from_mol(fm),
                    "sdf_smiles": smi_iso,
                    "sdf_smiles_from_molblock": smi_iso_from_molblock,
                    "molblock2d": molblock,
                    "center": center,
                    "mol": fm,
                    "fp": fp_from_mol(fm),
                    "signature": structure_signature_from_mol(fm),
                }
            )

    cdxml_records: List[Dict[str, Any]] = []
    for i, (smi_cdxml, (cx, cy), (w, h), label, cdxml_molblock) in enumerate(mol_match):
        cdxml_mol = mol_from_cdxml_input(smi_cdxml, cdxml_molblock)
        cdxml_records.append(
            {
                "idx": i,
                "smiles_from_cdxml": smi_cdxml,
                "center": (float(cx), float(cy)),
                "wh": (float(w), float(h)),
                "label": label,
                "mol": cdxml_mol,
                "key_nostereo": key_nostereo_from_mol(cdxml_mol) if cdxml_mol is not None else None,
                "inchikey_nostereo": inchikey_nostereo_from_mol(cdxml_mol),
                "fp": fp_from_mol(cdxml_mol),
                "signature": structure_signature_from_mol(cdxml_mol),
            }
        )

    if cdxml_records and not sdf_records:
        raise ValueError(f"No readable molecules in SDF for matching: {sdf_path}")
    if len(sdf_records) < len(cdxml_records):
        raise ValueError(
            f"Insufficient SDF molecules for one-to-one matching: "
            f"cdxml={len(cdxml_records)}, sdf={len(sdf_records)}, file={sdf_path}"
        )

    exact_key_to_cdxml: Dict[str, List[int]] = {}
    exact_key_to_sdf: Dict[str, List[int]] = {}
    exact_inchi_to_cdxml: Dict[str, List[int]] = {}
    exact_inchi_to_sdf: Dict[str, List[int]] = {}
    for i, row in enumerate(cdxml_records):
        if row.get("key_nostereo"):
            exact_key_to_cdxml.setdefault(str(row["key_nostereo"]), []).append(i)
        if row.get("inchikey_nostereo"):
            exact_inchi_to_cdxml.setdefault(str(row["inchikey_nostereo"]), []).append(i)
    for j, row in enumerate(sdf_records):
        if row.get("key_nostereo"):
            exact_key_to_sdf.setdefault(str(row["key_nostereo"]), []).append(j)
        if row.get("inchikey_nostereo"):
            exact_inchi_to_sdf.setdefault(str(row["inchikey_nostereo"]), []).append(j)

    assigned_cdxml: Dict[int, int] = {}
    assigned_sdf: Set[int] = set()

    def preassign_unique(left_map: Dict[str, List[int]], right_map: Dict[str, List[int]]) -> None:
        for key, left_idxs in left_map.items():
            right_idxs = right_map.get(key) or []
            if len(left_idxs) == 1 and len(right_idxs) == 1:
                i = left_idxs[0]
                j = right_idxs[0]
                if i in assigned_cdxml or j in assigned_sdf:
                    continue
                assigned_cdxml[i] = j
                assigned_sdf.add(j)

    preassign_unique(exact_inchi_to_cdxml, exact_inchi_to_sdf)
    preassign_unique(exact_key_to_cdxml, exact_key_to_sdf)

    cdxml_layout_rank = {int(row["idx"]): rank for rank, row in enumerate(sorted(cdxml_records, key=layout_sort_key), start=1)}
    sdf_layout_rank = {j: rank for rank, (j, row) in enumerate(sorted(list(enumerate(sdf_records)), key=lambda pair: layout_sort_key(pair[1])), start=1)}

    unresolved_cdxml = [i for i in range(len(cdxml_records)) if i not in assigned_cdxml]
    unresolved_sdf = [j for j in range(len(sdf_records)) if j not in assigned_sdf]

    score_matrix: List[List[float]] = []
    for i in unresolved_cdxml:
        c = cdxml_records[i]
        row_scores: List[float] = []
        for j in unresolved_sdf:
            s = sdf_records[j]
            compatible = structures_are_compatible(c, s)
            exact = 1.0 if (bool(c.get("key_nostereo")) and bool(s.get("key_nostereo")) and c.get("key_nostereo") == s.get("key_nostereo")) else 0.0
            exact_inchi = 1.0 if (bool(c.get("inchikey_nostereo")) and bool(s.get("inchikey_nostereo")) and c.get("inchikey_nostereo") == s.get("inchikey_nostereo")) else 0.0
            graph_equiv = 1.0 if same_connectivity(c, s) else 0.0
            sim = fp_similarity(c, s)
            has_mb = 1.0 if s.get("molblock2d") is not None else 0.0
            layout_gap = abs(cdxml_layout_rank.get(i, 10**6) - sdf_layout_rank.get(j, 10**6))
            layout_bonus = 1.0 / (1.0 + float(layout_gap))
            compat_bonus = 1.0 if compatible else -1.0
            score = exact_inchi * 1_500_000.0 + exact * 1_000_000.0 + graph_equiv * 300_000.0 + compat_bonus * 100_000.0 + sim * 1_000.0 + has_mb * 1.0 + layout_bonus * 0.001
            row_scores.append(score)
        score_matrix.append(row_scores)

    if unresolved_cdxml and unresolved_sdf:
        max_score = max((max(row) for row in score_matrix), default=0.0)
        cost_matrix = [[max_score - score for score in row] for row in score_matrix]
        assignment = hungarian_min_cost(cost_matrix)
        for row_idx, col_idx in enumerate(assignment):
            if col_idx is None or col_idx < 0:
                continue
            assigned_cdxml[unresolved_cdxml[row_idx]] = unresolved_sdf[col_idx]

    unmatched_cdxml = [i for i in range(len(cdxml_records)) if i not in assigned_cdxml]
    if unmatched_cdxml:
        raise ValueError(
            f"Failed to assign all CDXML molecules to SDF molecules "
            f"(unmatched_cdxml_indices={unmatched_cdxml}, file={sdf_path})"
        )

    out: List[Dict[str, Any]] = []
    matched_cnt = 0
    for i, c in enumerate(cdxml_records):
        s = sdf_records[assigned_cdxml[i]]
        compatible = structures_are_compatible(c, s)
        final_smiles = c["smiles_from_cdxml"]
        if compatible:
            final_smiles = s.get("sdf_smiles_from_molblock") or s.get("sdf_smiles") or c["smiles_from_cdxml"]
        matched = compatible and bool(s.get("sdf_smiles_from_molblock") or s.get("sdf_smiles"))
        if matched:
            matched_cnt += 1
        out.append(
            {
                "smiles": final_smiles,
                "smiles_from_cdxml": c["smiles_from_cdxml"],
                "key_nostereo": c.get("key_nostereo"),
                "center": c["center"],
                "wh": c["wh"],
                "label": c["label"],
                "matched": matched,
                "sdf_index": s.get("sdf_index"),
                "sdf_smiles": s.get("sdf_smiles"),
                "molblock2d": s.get("molblock2d") if keep_molblock2d else None,
            }
        )

    if verbose:
        unmatched = [rec["smiles_from_cdxml"] for rec in out if not rec["matched"]]
        if unmatched:
            print(f"[SDF match] Unmatched ({len(unmatched)}):")
            for s in unmatched:
                print(f"  - {s}")
    return out
