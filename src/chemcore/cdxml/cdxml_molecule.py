from __future__ import annotations

from copy import deepcopy
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple
import xml.etree.ElementTree as ET

from rdkit import Chem
from rdkit.Geometry import Point3D

from .cdxml_stereo import (
    apply_2d_stereo_annotations,
    apply_abs_stereo_targets,
    apply_display_tetra_targets,
    apply_double_bond_stereo_from_coords,
    bond_dir_from_display,
    chiral_tag_from_display,
    molblock_with_2d_stereo,
    normalize_terminal_substituents_on_hetero,
)


PLACEHOLDER_TYPES: Set[str] = {"Fragment", "Nickname", "Unspecified"}
ECP_TYPE = "ExternalConnectionPoint"
ABBREV_TEMPLATE_SMILES: Dict[str, str] = {
    "Me": "*C",
    "Et": "*CC",
    "i-Pr": "*C(C)C",
    "iPr": "*C(C)C",
    "t-Bu": "*C(C)(C)C",
    "tBu": "*C(C)(C)C",
    "Ph": "*c1ccccc1",
    "Bn": "*Cc1ccccc1",
    "CF3": "*C(F)(F)F",
    "N3": "*N=[N+]=[N-]",
}


def _is_tag(el: ET.Element, name: str) -> bool:
    return el.tag.endswith(name)


def _parse_xy(p: Optional[str]) -> Optional[Tuple[float, float]]:
    if not p:
        return None
    s = p.strip().split()
    if len(s) < 2:
        return None
    try:
        return float(s[0]), float(s[1])
    except Exception:
        return None


def extract_molecules_smiles_and_centers(
    cdxml_path: str,
) -> List[Tuple[str, Tuple[float, float], Tuple[float, float], Optional[str]]]:
    xml_path = Path(f"{cdxml_path}.cdxml")
    if not (xml_path.exists() and xml_path.is_file()):
        raise FileNotFoundError(str(xml_path))

    root = ET.parse(str(xml_path)).getroot()

    node_type: Dict[str, str] = {}
    atomic_num: Dict[str, int] = {}
    xy: Dict[str, Optional[Tuple[float, float]]] = {}
    atom_abs_stereo: Dict[str, str] = {}
    atom_charge: Dict[str, int] = {}
    atom_geometry: Dict[str, str] = {}
    atom_bond_ordering: Dict[str, List[str]] = {}
    placeholder_label: Dict[str, str] = {}

    for el in root.iter():
        if _is_tag(el, "n") and "id" in el.attrib:
            nid = el.attrib["id"]
            node_type[nid] = el.attrib.get("NodeType", "")
            if "Element" in el.attrib:
                try:
                    atomic_num[nid] = int(el.attrib["Element"])
                except Exception:
                    atomic_num[nid] = 6
            else:
                atomic_num[nid] = 6
            xy[nid] = _parse_xy(el.attrib.get("p"))
            atom_abs_stereo[nid] = (el.attrib.get("AS", "") or "").strip()
            atom_geometry[nid] = (el.attrib.get("Geometry", "") or "").strip()
            atom_bond_ordering[nid] = (el.attrib.get("BondOrdering", "") or "").split()
            try:
                atom_charge[nid] = int(el.attrib.get("Charge", "0") or "0")
            except Exception:
                atom_charge[nid] = 0
            text_tokens = [
                "".join(t.itertext()).strip()
                for t in el.iter()
                if _is_tag(t, "t") and "".join(t.itertext()).strip()
            ]
            if text_tokens:
                placeholder_label[nid] = max(text_tokens, key=len).replace("\n", "").replace("\t", "").strip()

    bonds: List[Dict[str, object]] = []
    bond_display_by_id: Dict[str, str] = {}
    bond_atoms_by_id: Dict[str, Tuple[str, str]] = {}
    for el in root.iter():
        if _is_tag(el, "b"):
            b = el.attrib.get("B")
            e = el.attrib.get("E")
            if not b or not e:
                continue
            try:
                order = int(el.attrib.get("Order", "1"))
            except Exception:
                order = 1
            bonds.append(
                {
                    "id": el.attrib.get("id"),
                    "b": b,
                    "e": e,
                    "order": order,
                    "display": el.attrib.get("Display", ""),
                }
            )
            bond_id = el.attrib.get("id")
            if bond_id:
                bond_display_by_id[bond_id] = el.attrib.get("Display", "")
                bond_atoms_by_id[bond_id] = (b, e)

    placeholder_inline_nodes: Dict[str, Set[str]] = {}
    all_inline_nodes: Set[str] = set()
    for el in root.iter():
        if _is_tag(el, "n") and "id" in el.attrib:
            pid = el.attrib["id"]
            if el.attrib.get("NodeType", "") not in PLACEHOLDER_TYPES:
                continue
            inline_nodes: Set[str] = set()
            for ch in list(el):
                if _is_tag(ch, "fragment"):
                    for sub in ch.iter():
                        if _is_tag(sub, "n") and "id" in sub.attrib:
                            inline_nodes.add(sub.attrib["id"])
            if inline_nodes:
                placeholder_inline_nodes[pid] = inline_nodes
                all_inline_nodes |= inline_nodes

    bonds_unexpanded = deepcopy(bonds)

    def incident_bonds(nid: str) -> List[Dict[str, object]]:
        return [bd for bd in bonds if bd["b"] == nid or bd["e"] == nid]

    def remove_bonds_touching(ids: Set[str]) -> None:
        bonds[:] = [bd for bd in bonds if bd["b"] not in ids and bd["e"] not in ids]

    def get_external_connection_num(nid: str) -> int:
        for el2 in root.iter():
            if _is_tag(el2, "n") and el2.attrib.get("id") == nid:
                v = el2.attrib.get("ExternalConnectionNum")
                if v and v.isdigit():
                    return int(v)
                return 0
        return 0

    def is_real_atom(nid: str) -> bool:
        return node_type.get(nid, "") not in ({ECP_TYPE} | PLACEHOLDER_TYPES)

    def expand_attached_abbreviation(pid: str) -> bool:
        label = (placeholder_label.get(pid) or "").strip()
        template_smiles = ABBREV_TEMPLATE_SMILES.get(label)
        if not template_smiles:
            return False

        ext_bds = []
        ext_nei = []
        for bd in incident_bonds(pid):
            other = bd["e"] if bd["b"] == pid else bd["b"]
            if not is_real_atom(other):
                continue
            ext_bds.append(bd)
            ext_nei.append(other)
        if not ext_bds:
            return False

        template = Chem.MolFromSmiles(template_smiles)
        if template is None:
            return False
        try:
            Chem.Kekulize(template, clearAromaticFlags=True)
        except Exception:
            pass

        dummy_idx = None
        for atom in template.GetAtoms():
            if atom.GetAtomicNum() == 0:
                dummy_idx = atom.GetIdx()
                break
        if dummy_idx is None:
            return False
        dummy = template.GetAtomWithIdx(dummy_idx)
        nbrs = list(dummy.GetNeighbors())
        if len(nbrs) != 1:
            return False
        attach_idx = nbrs[0].GetIdx()

        pid_xy = xy.get(pid) or (0.0, 0.0)
        ref_xy = xy.get(ext_nei[0]) or (pid_xy[0] - 14.4, pid_xy[1])
        dx = pid_xy[0] - ref_xy[0]
        dy = pid_xy[1] - ref_xy[1]
        norm = (dx * dx + dy * dy) ** 0.5
        if norm < 1e-6:
            dx, dy, norm = 1.0, 0.0, 1.0
        ux, uy = dx / norm, dy / norm

        tmpl_to_id: Dict[int, str] = {}
        for atom in template.GetAtoms():
            if atom.GetIdx() == dummy_idx:
                continue
            new_id = f"{pid}__abbr_{atom.GetIdx()}"
            tmpl_to_id[atom.GetIdx()] = new_id
            node_type[new_id] = ""
            atomic_num[new_id] = int(atom.GetAtomicNum())
            atom_charge[new_id] = int(atom.GetFormalCharge())
            atom_abs_stereo[new_id] = ""
            step = abs(atom.GetIdx() - attach_idx)
            xy[new_id] = (pid_xy[0] + ux * 10.0 * step, pid_xy[1] + uy * 10.0 * step)

        for bond in template.GetBonds():
            b_idx = bond.GetBeginAtomIdx()
            e_idx = bond.GetEndAtomIdx()
            if dummy_idx in {b_idx, e_idx}:
                continue
            order = 1
            if bond.GetBondType() == Chem.BondType.DOUBLE:
                order = 2
            elif bond.GetBondType() == Chem.BondType.TRIPLE:
                order = 3
            bonds.append({"id": None, "b": tmpl_to_id[b_idx], "e": tmpl_to_id[e_idx], "order": order, "display": ""})

        attach_atom_id = tmpl_to_id[attach_idx]
        for neighbor, bd in zip(ext_nei, ext_bds):
            bonds.append(
                {
                    "id": bd.get("id"),
                    "b": neighbor,
                    "e": attach_atom_id,
                    "order": int(bd.get("order", 1)),  # type: ignore[arg-type]
                    "display": str(bd.get("display", "")),
                }
            )

        remove_bonds_touching({pid})
        return True

    changed = True
    while changed:
        changed = False
        for pid, inline_nodes in list(placeholder_inline_nodes.items()):
            ecps = [nid for nid in inline_nodes if node_type.get(nid, "") == ECP_TYPE]
            if not ecps:
                continue

            ext_bds = []
            ext_nei = []
            for bd in incident_bonds(pid):
                other = bd["e"] if bd["b"] == pid else bd["b"]
                if other in inline_nodes:
                    continue
                if not is_real_atom(other):
                    continue
                ext_bds.append(bd)
                ext_nei.append(other)

            if not ext_bds:
                continue

            ecp_to_atom: Dict[str, str] = {}
            ecp_num: Dict[str, int] = {}
            for ecp in ecps:
                ecp_num[ecp] = get_external_connection_num(ecp)
                attach = None
                for bd in incident_bonds(ecp):
                    other = bd["e"] if bd["b"] == ecp else bd["b"]
                    if other in inline_nodes and node_type.get(other, "") != ECP_TYPE:
                        attach = other
                        break
                if attach:
                    ecp_to_atom[ecp] = attach

            if not ecp_to_atom:
                continue

            ecp_sorted = sorted(ecp_to_atom.keys(), key=lambda x: (ecp_num.get(x, 0), x))
            pairs = list(zip(ext_nei, ecp_sorted))
            if not pairs:
                continue

            for neighbor, ecp in pairs:
                attach_atom = ecp_to_atom[ecp]
                order = 1
                matched_bd: Optional[Dict[str, object]] = None
                for bd in ext_bds:
                    o = bd["e"] if bd["b"] == pid else bd["b"]
                    if o == neighbor:
                        order = int(bd.get("order", 1))  # type: ignore[arg-type]
                        matched_bd = bd
                        break
                bonds.append(
                    {
                        "id": None if matched_bd is None else matched_bd.get("id"),
                        "b": neighbor,
                        "e": attach_atom,
                        "order": order,
                        "display": "",
                    }
                )

            remove_bonds_touching({pid, *ecps})
            changed = True

        for pid, nt in list(node_type.items()):
            if nt != "Nickname":
                continue
            if pid in placeholder_inline_nodes:
                continue
            if expand_attached_abbreviation(pid):
                changed = True

    adj: Dict[str, Set[str]] = {nid: set() for nid in node_type.keys()}
    for bd in bonds:
        b, e = str(bd["b"]), str(bd["e"])
        if b in adj and e in adj:
            adj[b].add(e)
            adj[e].add(b)

    top_level_atoms = {nid for nid in adj.keys() if is_real_atom(nid) and (nid not in all_inline_nodes)}
    real_atoms = {nid for nid in adj.keys() if is_real_atom(nid)}
    real_adj: Dict[str, Set[str]] = {a: set() for a in real_atoms}
    for a in real_atoms:
        for nb in adj.get(a, ()):
            if nb in real_atoms:
                real_adj[a].add(nb)

    keep: Set[str] = set(top_level_atoms)
    stack = list(top_level_atoms)
    while stack:
        x = stack.pop()
        for nb in real_adj.get(x, ()):
            if nb not in keep:
                keep.add(nb)
                stack.append(nb)

    comps: List[List[str]] = []
    seen: Set[str] = set()
    for a in sorted(keep):
        if a in seen:
            continue
        st = [a]
        seen.add(a)
        comp: List[str] = []
        while st:
            x = st.pop()
            comp.append(x)
            for nb in real_adj.get(x, ()):
                if nb in keep and nb not in seen:
                    seen.add(nb)
                    st.append(nb)
        if len(comp) == 1 and len(real_adj.get(comp[0], ())) == 0:
            continue
        comps.append(comp)

    comps.sort(key=len, reverse=True)

    def _geom_for_component_unexpanded(comp_nodes_expanded: List[str]) -> Tuple[float, float, float, float]:
        comp_set = set(comp_nodes_expanded)
        use_ids: Set[str] = set()
        for nid in comp_nodes_expanded:
            if nid in all_inline_nodes:
                continue
            if xy.get(nid) is not None:
                use_ids.add(nid)
        for bd in bonds_unexpanded:
            b = str(bd["b"])
            e = str(bd["e"])
            bt = node_type.get(b, "")
            et = node_type.get(e, "")
            if bt in PLACEHOLDER_TYPES and e in comp_set and b not in all_inline_nodes and xy.get(b) is not None:
                use_ids.add(b)
            if et in PLACEHOLDER_TYPES and b in comp_set and e not in all_inline_nodes and xy.get(e) is not None:
                use_ids.add(e)
        pts = [xy[nid] for nid in use_ids if xy.get(nid) is not None]
        if not pts:
            return 0.0, 0.0, 0.0, 0.0
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        xmin, xmax = min(xs), max(xs)
        ymin, ymax = min(ys), max(ys)
        cx = sum(xs) / len(xs)
        cy = sum(ys) / len(ys)
        return float(cx), float(cy), float(xmax - xmin), float(ymax - ymin)

    results: List[Tuple[str, Tuple[float, float], Tuple[float, float], Optional[str]]] = []
    for i, comp_nodes in enumerate(comps, start=1):
        comp_set = set(comp_nodes)
        rw = Chem.RWMol()
        idmap: Dict[str, int] = {}

        for nid in comp_nodes:
            anum = int(atomic_num.get(nid, 6))
            atom = Chem.Atom(anum)
            charge = int(atom_charge.get(nid, 0))
            if charge:
                atom.SetFormalCharge(charge)
            idmap[nid] = rw.AddAtom(atom)

        for bd in bonds:
            b, e = str(bd["b"]), str(bd["e"])
            if b in comp_set and e in comp_set:
                ib, ie = idmap[b], idmap[e]
                if rw.GetBondBetweenAtoms(ib, ie) is not None:
                    continue
                o = int(bd.get("order", 1))  # type: ignore[arg-type]
                btype = Chem.BondType.SINGLE
                if o == 2:
                    btype = Chem.BondType.DOUBLE
                elif o == 3:
                    btype = Chem.BondType.TRIPLE
                bond_dir, reverse = bond_dir_from_display(str(bd.get("display", "")))
                add_b, add_e = (ie, ib) if reverse else (ib, ie)
                rw.AddBond(add_b, add_e, btype)
                bond = rw.GetBondBetweenAtoms(ib, ie)
                if bond is not None and bond_dir is not None and btype == Chem.BondType.SINGLE:
                    bond.SetBondDir(bond_dir)

        mol = rw.GetMol()
        conf = Chem.Conformer(mol.GetNumAtoms())
        conf.Set3D(False)
        for nid, aid in idmap.items():
            p = xy.get(nid) or (0.0, 0.0)
            conf.SetAtomPosition(aid, Point3D(float(p[0]), float(p[1]), 0.0))
        mol.AddConformer(conf, assignId=True)

        smi = "SANITIZE_FAIL"
        molblock_cdxml: Optional[str] = None
        export_mol = Chem.Mol(mol)
        stereo_targets = [(idmap[nid], atom_abs_stereo.get(nid)) for nid in comp_nodes if atom_abs_stereo.get(nid) in {"R", "S"} and nid in idmap]
        display_tetra_targets = []
        for nid in comp_nodes:
            if nid not in idmap:
                continue
            if atom_geometry.get(nid) != "Tetrahedral":
                continue
            if atom_abs_stereo.get(nid) in {"R", "S"}:
                continue
            for bond_id in atom_bond_ordering.get(nid) or []:
                if not bond_id or bond_id == "0":
                    continue
                tag = chiral_tag_from_display(
                    bond_display_by_id.get(bond_id, ""),
                    atom_bond_ordering.get(nid) or [],
                    bond_id,
                    nid,
                    bond_atoms_by_id,
                )
                if tag is not None:
                    display_tetra_targets.append((idmap[nid], tag))
                    break
        try:
            Chem.SanitizeMol(export_mol)
            export_mol = apply_2d_stereo_annotations(export_mol)
            export_mol = apply_display_tetra_targets(export_mol, display_tetra_targets)
            export_mol = apply_abs_stereo_targets(export_mol, stereo_targets)
            export_mol = apply_double_bond_stereo_from_coords(export_mol)
            smi = Chem.MolToSmiles(export_mol, isomericSmiles=True)
        except Exception:
            try:
                rw2 = Chem.RWMol(export_mol)
                normalize_terminal_substituents_on_hetero(rw2)
                mol2 = rw2.GetMol()
                Chem.SanitizeMol(mol2)
                mol2 = apply_2d_stereo_annotations(mol2)
                mol2 = apply_display_tetra_targets(mol2, display_tetra_targets)
                mol2 = apply_abs_stereo_targets(mol2, stereo_targets)
                mol2 = apply_double_bond_stereo_from_coords(mol2)
                smi = Chem.MolToSmiles(mol2, isomericSmiles=False)
                export_mol = mol2
            except Exception:
                try:
                    smi = Chem.MolToSmiles(mol, isomericSmiles=False)
                except Exception:
                    pass

        try:
            molblock_cdxml = molblock_with_2d_stereo(export_mol)
        except Exception:
            try:
                molblock_cdxml = Chem.MolToMolBlock(mol)
            except Exception:
                molblock_cdxml = None

        mol.SetProp("_Name", str(i))
        mol.SetProp("SMILES", smi)

        cx, cy, w, h = _geom_for_component_unexpanded(comp_nodes)
        results.append((smi, (cx, cy), (w, h), molblock_cdxml))

    return results
