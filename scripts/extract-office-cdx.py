"""Extract embedded CDX streams from an OOXML package, entirely offline.

Preserves every embedding and records every relationship, including non-CDX
objects. Requires olefile. No Office application or clipboard is used.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import posixpath
import re
import struct
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

import olefile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def emf_cdx_payloads(data):
    """Reassemble ChemDraw CDIF data in EMF+ comment records.

    Each comment has its own CDIF prefix and EMF+ record header, which must
    not be mistaken for part of the embedded CDX byte stream.
    """
    if len(data) < 44 or data[40:44] != b' EMF':
        return []
    offset, chunks, payloads = 0, [], []
    while offset + 8 <= len(data):
        kind, size = struct.unpack_from('<II', data, offset)
        if size < 8 or offset + size > len(data):
            raise ValueError('Invalid EMF record length')
        if kind == 70 and size >= 16 and data[offset+12:offset+16] == b'EMF+':
            inner = offset + 16
            while inner + 12 <= offset + size:
                tag, flags, length, count = struct.unpack_from('<HHII', data, inner)
                if length < 12 or count > length-12 or inner+length > offset+size:
                    raise ValueError('Invalid EMF+ record length')
                body = data[inner+12:inner+12+count]
                if tag == 0x4003 and body.startswith(b'CDIF\0'):
                    part = body[5:]
                    if part.startswith(b'VjCD0100') and chunks:
                        payloads.append(b''.join(chunks))
                        chunks = []
                    chunks.append(part)
                    if flags & 2:
                        payloads.append(b''.join(chunks))
                        chunks = []
                inner += length
        offset += size
    if chunks:
        payloads.append(b''.join(chunks))
    return payloads


def validate_cdx(data):
    if not data.startswith(b'VjCD0100\x04\x03\x02\x01' + bytes(10)):
        raise ValueError('Invalid CDX header')
    offset, depth = 22, 0
    while offset+2 <= len(data):
        tag = struct.unpack_from('<H', data, offset)[0]
        offset += 2
        if tag == 0:
            depth -= 1
            if depth == 0:
                return
            if depth < 0:
                break
        elif tag >= 0x8000:
            depth += 1
            offset += 4
        else:
            if depth == 0 or offset+2 > len(data):
                break
            length = struct.unpack_from('<H', data, offset)[0]
            offset += 2
            if length == 0xffff:
                if offset+4 > len(data):
                    break
                length = struct.unpack_from('<I', data, offset)[0]
                offset += 4
            offset += length
        if offset > len(data):
            break
    raise ValueError('Truncated or unbalanced CDX object records')


def extract(source: Path, output: Path):
    output.mkdir(parents=True, exist_ok=True)
    records, references = [], []
    with zipfile.ZipFile(source) as package:
        for part in package.namelist():
            if not part.endswith('.rels'):
                continue
            folder, name = posixpath.split(part)
            owner = posixpath.join(posixpath.dirname(folder), name[:-5])
            for rel in ET.fromstring(package.read(part)):
                if rel.get('TargetMode') == 'External':
                    continue
                target = rel.get('Target', '')
                target = (target.lstrip('/') if target.startswith('/') else
                          posixpath.normpath(posixpath.join(posixpath.dirname(owner), target)))
                references.append(dict(owner=owner, relationshipId=rel.get('Id'),
                                       type=rel.get('Type'), target=target))
        names = sorted((n for n in package.namelist() if '/embeddings/' in n and not n.endswith('/')),
                       key=lambda s: re.sub(r'\d+', lambda m: m[0].zfill(10), s))
        for index, name in enumerate(names, 1):
            data = package.read(name)
            folder = output / f'object-{index:04d}'
            folder.mkdir(exist_ok=True)
            (folder / Path(name).name).write_bytes(data)
            record = dict(index=index, embedding=name, sha256=digest(data),
                          references=[r for r in references if r['target'] == name],
                          streams=[], cdx=[], status='non-cdx')
            streams = [('raw', data)] if data.startswith(b'VjCD0100') else []
            if olefile.isOleFile(io.BytesIO(data)):
                with olefile.OleFileIO(io.BytesIO(data)) as ole:
                    streams.extend(('/'.join(s), ole.openstream(s).read()) for s in ole.listdir())
            for stream, payload in streams:
                record['streams'].append(dict(name=stream, bytes=len(payload), sha256=digest(payload)))
                candidates = ([payload] if payload.startswith(b'VjCD0100')
                              else emf_cdx_payloads(payload))
                for cdx in candidates:
                    validate_cdx(cdx)
                    path = folder / f'chemical-{len(record["cdx"])+1:02d}.cdx'
                    path.write_bytes(cdx)
                    record['cdx'].append(dict(path=str(path.resolve()), stream=stream,
                        representation='direct' if payload is cdx else 'emf-plus-cdif',
                        bytes=len(cdx), sha256=digest(cdx)))
                    record['status'] = 'extracted'
            records.append(record)
        # Preserve Word object occurrence locations without extracting prose.
        occurrences = []
        for part in package.namelist():
            if not part.endswith('.xml') or '/_rels/' in part:
                continue
            try:
                root = ET.fromstring(package.read(part))
            except ET.ParseError:
                continue
            for ordinal, element in enumerate(root.iter()):
                if element.tag.rsplit('}', 1)[-1] == 'OLEObject':
                    attrs = {k.rsplit('}', 1)[-1]: v for k, v in element.attrib.items()}
                    occurrences.append(dict(part=part, elementOrdinal=ordinal, attributes=attrs,
                        targets=[r['target'] for r in references
                                 if r['owner'] == part and r['relationshipId'] == attrs.get('id')]))
    result = dict(schema='office-cdx-extraction-v1', source=str(source.resolve()),
                  sourceSha256=digest(source.read_bytes()), embeddings=records, occurrences=occurrences,
                  summary=dict(embeddings=len(records), occurrences=len(occurrences),
                               cdxStreams=sum(len(r['cdx']) for r in records),
                               nonCdxEmbeddings=sum(not r['cdx'] for r in records)))
    (output / 'manifest.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    print(json.dumps(extract(args.source, args.output)['summary']))
