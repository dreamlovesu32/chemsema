import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('extract_office_cdx', Path(__file__).parents[1]/'extract-office-cdx.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def chemical():
    return b'VjCD0100\x04\x03\x02\x01'+bytes(10)+struct.pack('<HIH', 0x8000, 1, 0)+b'\0\0'


def metafile(parts):
    header = bytearray(108)
    struct.pack_into('<II', header, 0, 1, 108)
    header[40:44] = b' EMF'
    content = b'EMF+'
    for index, part in enumerate(parts):
        body = b'CDIF\0'+part
        size = (12+len(body)+3)//4*4
        content += struct.pack('<HHII', 0x4003, 2 if index==len(parts)-1 else 0, size, len(body)) + body + bytes(size-12-len(body))
    return bytes(header)+struct.pack('<III', 70, 12+len(content), len(content))+content


class ExtractionTests(unittest.TestCase):
    def test_reassembles_cdif_across_record_boundaries(self):
        data = chemical()
        for boundary in [1, 8, 22, 27]:
            result = module.emf_cdx_payloads(metafile([data[:boundary], data[boundary:]]))
            self.assertEqual(result, [data])
            module.validate_cdx(result[0])

    def test_rejects_truncated_objects_and_arbitrary_magic_hits(self):
        with self.assertRaises(ValueError):
            module.validate_cdx(chemical()[:27])
        self.assertEqual(module.emf_cdx_payloads(b'not a metafile'+chemical()), [])

    def test_retains_every_embedding_and_relationship(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp)/'input.docx'
            with zipfile.ZipFile(source, 'w') as package:
                package.writestr('word/embeddings/item1.bin', chemical())
                package.writestr('word/embeddings/item2.bin', b'non-chemical')
                package.writestr('word/_rels/document.xml.rels', '<Relationships><Relationship Id="rId1" Target="embeddings/item1.bin" Type="oleObject"/></Relationships>')
                package.writestr('word/document.xml', '<document xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><o:OLEObject ProgID="ChemDraw.Document.6.0" r:id="rId1"/></document>')
            result = module.extract(source, Path(temp)/'output')
            self.assertEqual(result['summary'], dict(embeddings=2, occurrences=1, cdxStreams=1, nonCdxEmbeddings=1))
            self.assertEqual(result['occurrences'][0]['targets'], ['word/embeddings/item1.bin'])
            self.assertEqual(Path(result['embeddings'][0]['cdx'][0]['path']).read_bytes(), chemical())


if __name__ == '__main__':
    unittest.main()
