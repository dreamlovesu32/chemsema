from __future__ import annotations

from itertools import product
from typing import List, Mapping, Optional, Sequence, Set, Tuple

from rdkit import Chem
from rdkit.Chem import rdCIPLabeler


def bond_dir_from_display(display: str) -> Tuple[Optional[Chem.BondDir], bool]:
    value = (display or "").strip()
    if value == "WedgeBegin":
        return Chem.BondDir.BEGINDASH, False
    if value == "WedgedHashBegin":
        return Chem.BondDir.BEGINWEDGE, False
    if value == "WedgeEnd":
        return Chem.BondDir.BEGINDASH, True
    if value == "WedgedHashEnd":
        return Chem.BondDir.BEGINWEDGE, True
    return None, False


def chiral_tag_from_display(
    display: str,
    bond_ordering: Optional[Sequence[str]] = None,
    bond_id: Optional[str] = None,
    atom_id: Optional[str] = None,
    bond_atoms_by_id: Optional[Mapping[str, Tuple[str, str]]] = None,
) -> Optional[Chem.ChiralType]:
    value = (display or "").strip()
    if value == "WedgeBegin":
        return Chem.ChiralType.CHI_TETRAHEDRAL_CW
    if value == "WedgedHashBegin":
        return Chem.ChiralType.CHI_TETRAHEDRAL_CCW
    if value == "WedgeEnd":
        if bond_ordering and bond_id and atom_id and bond_atoms_by_id and "0" in bond_ordering:
            try:
                zero_idx = bond_ordering.index("0")
                bond_idx = bond_ordering.index(bond_id)
                if bond_idx < zero_idx:
                    return Chem.ChiralType.CHI_TETRAHEDRAL_CW
                if bond_idx > zero_idx:
                    ordered = [item for item in bond_ordering if item and item != "0" and item in bond_atoms_by_id]
                    pre = ordered[:zero_idx]
                    if len(pre) >= 2:
                        first_b, first_e = bond_atoms_by_id[pre[0]]
                        second_b, second_e = bond_atoms_by_id[pre[1]]
                        stereo_b, stereo_e = bond_atoms_by_id[bond_id]
                        if (
                            first_b == atom_id
                            and second_e == atom_id
                            and stereo_e == atom_id
                        ):
                            return Chem.ChiralType.CHI_TETRAHEDRAL_CW
            except ValueError:
                pass
        return Chem.ChiralType.CHI_TETRAHEDRAL_CCW
    if value == "WedgedHashEnd":
        if bond_ordering and bond_id and "0" in bond_ordering:
            try:
                if bond_ordering.index(bond_id) < bond_ordering.index("0"):
                    return Chem.ChiralType.CHI_TETRAHEDRAL_CCW
            except ValueError:
                pass
        return Chem.ChiralType.CHI_TETRAHEDRAL_CW
    return None


def normalize_terminal_substituents_on_hetero(rw: Chem.RWMol) -> None:
    target_atom_idxs: Set[int] = set()
    for atom in rw.GetAtoms():
        if atom.GetAtomicNum() in (1, 6):
            continue
        for nb in atom.GetNeighbors():
            if nb.GetAtomicNum() == 1 or nb.GetDegree() != 1:
                continue
            target_atom_idxs.add(nb.GetIdx())

    if not target_atom_idxs:
        return

    for idx in target_atom_idxs:
        at = rw.GetAtomWithIdx(idx)
        at.SetAtomicNum(1)
        at.SetFormalCharge(0)
        at.SetIsotope(0)
        at.SetNumExplicitHs(0)
        at.SetIsAromatic(False)
        at.SetNoImplicit(True)

    for bd in rw.GetBonds():
        b = bd.GetBeginAtomIdx()
        e = bd.GetEndAtomIdx()
        if b in target_atom_idxs or e in target_atom_idxs:
            bd.SetBondType(Chem.BondType.SINGLE)
            bd.SetIsAromatic(False)


def apply_abs_stereo_targets(
    mol: Chem.Mol,
    targets: Sequence[Tuple[int, str]],
) -> Chem.Mol:
    if not targets:
        return mol

    tags = (
        Chem.ChiralType.CHI_TETRAHEDRAL_CW,
        Chem.ChiralType.CHI_TETRAHEDRAL_CCW,
    )

    def _clear_and_assign(m2: Chem.Mol) -> None:
        for atom in m2.GetAtoms():
            if atom.HasProp("_CIPCode"):
                atom.ClearProp("_CIPCode")
        Chem.AssignStereochemistry(m2, cleanIt=True, force=True)

    for combo in product(tags, repeat=len(targets)):
        m2 = Chem.Mol(mol)
        for (atom_idx, _target), _tag in zip(targets, combo):
            atom = m2.GetAtomWithIdx(atom_idx)
            atom.SetChiralTag(Chem.ChiralType.CHI_UNSPECIFIED)
            if atom.HasProp("_CIPCode"):
                atom.ClearProp("_CIPCode")
        for (atom_idx, _target), tag in zip(targets, combo):
            m2.GetAtomWithIdx(atom_idx).SetChiralTag(tag)
        try:
            _clear_and_assign(m2)
        except Exception:
            continue
        ok = True
        for atom_idx, target in targets:
            atom = m2.GetAtomWithIdx(atom_idx)
            got = atom.GetProp("_CIPCode") if atom.HasProp("_CIPCode") else ""
            if got != target:
                ok = False
                break
        if ok:
            return m2
    return mol


def apply_2d_stereo_annotations(m2: Chem.Mol) -> Chem.Mol:
    try:
        conf_id = m2.GetConformer().GetId()
    except Exception:
        conf_id = -1
    try:
        Chem.AssignChiralTypesFromBondDirs(m2, confId=conf_id, replaceExistingTags=False)
    except Exception:
        pass
    try:
        Chem.SetBondStereoFromDirections(m2)
    except Exception:
        pass
    try:
        Chem.AssignStereochemistry(m2, cleanIt=True, force=True)
    except Exception:
        pass
    return m2


def apply_double_bond_stereo_from_coords(m2: Chem.Mol) -> Chem.Mol:
    def _choose_stereo_neighbor(mol: Chem.Mol, atom_idx: int, other_idx: int) -> Optional[int]:
        candidates = [
            nb.GetIdx()
            for nb in mol.GetAtomWithIdx(atom_idx).GetNeighbors()
            if nb.GetIdx() != other_idx and nb.GetAtomicNum() != 1
        ]
        if not candidates:
            return None
        best_idx = candidates[0]
        best_rank = (
            mol.GetAtomWithIdx(best_idx)
            .GetPropsAsDict(includePrivate=True, includeComputed=True)
            .get("_CIPRank", -1)
        )
        for cand_idx in candidates[1:]:
            rank = (
                mol.GetAtomWithIdx(cand_idx)
                .GetPropsAsDict(includePrivate=True, includeComputed=True)
                .get("_CIPRank", -1)
            )
            if rank > best_rank:
                best_idx = cand_idx
                best_rank = rank
        return best_idx

    def _side(conf: Chem.Conformer, a_idx: int, b_idx: int, p_idx: int) -> float:
        a_pos = conf.GetAtomPosition(a_idx)
        b_pos = conf.GetAtomPosition(b_idx)
        p_pos = conf.GetAtomPosition(p_idx)
        return (b_pos.x - a_pos.x) * (p_pos.y - a_pos.y) - (b_pos.y - a_pos.y) * (p_pos.x - a_pos.x)

    m3 = Chem.Mol(m2)
    try:
        Chem.AssignStereochemistry(m3, cleanIt=True, force=True)
    except Exception:
        pass
    try:
        rdCIPLabeler.AssignCIPLabels(m3)
    except Exception:
        pass
    try:
        conf = m3.GetConformer()
    except Exception:
        return m2

    changed = False
    for bond in m3.GetBonds():
        if bond.GetBondType() != Chem.BondType.DOUBLE:
            continue
        if bond.GetStereo() not in {Chem.BondStereo.STEREONONE, Chem.BondStereo.STEREOANY}:
            continue
        begin_idx = bond.GetBeginAtomIdx()
        end_idx = bond.GetEndAtomIdx()
        left_idx = _choose_stereo_neighbor(m3, begin_idx, end_idx)
        right_idx = _choose_stereo_neighbor(m3, end_idx, begin_idx)
        if left_idx is None or right_idx is None:
            continue
        left_side = _side(conf, begin_idx, end_idx, left_idx)
        right_side = _side(conf, begin_idx, end_idx, right_idx)
        if abs(left_side) < 1e-6 or abs(right_side) < 1e-6:
            continue
        stereo = Chem.BondStereo.STEREOZ if left_side * right_side > 0 else Chem.BondStereo.STEREOE
        bond.SetStereoAtoms(left_idx, right_idx)
        bond.SetStereo(stereo)
        changed = True

    if not changed:
        return m2
    try:
        Chem.SetDoubleBondNeighborDirections(m3, conf)
    except Exception:
        pass
    try:
        Chem.AssignStereochemistry(m3, cleanIt=True, force=True)
    except Exception:
        pass
    return m3


def apply_display_tetra_targets(
    mol: Chem.Mol,
    targets: Sequence[Tuple[int, Chem.ChiralType]],
) -> Chem.Mol:
    if not targets:
        return mol
    m2 = Chem.Mol(mol)
    changed = False
    for atom_idx, tag in targets:
        atom = m2.GetAtomWithIdx(atom_idx)
        if atom.GetChiralTag() == Chem.ChiralType.CHI_UNSPECIFIED:
            atom.SetChiralTag(tag)
            changed = True
    if changed:
        try:
            Chem.AssignStereochemistry(m2, cleanIt=True, force=True)
        except Exception:
            pass
    return m2


def molblock_with_2d_stereo(m2: Chem.Mol) -> Optional[str]:
    try:
        molblock_mol = Chem.Mol(m2)
        Chem.WedgeMolBonds(molblock_mol, molblock_mol.GetConformer())
        return Chem.MolToMolBlock(molblock_mol)
    except Exception:
        try:
            return Chem.MolToMolBlock(m2)
        except Exception:
            return None
