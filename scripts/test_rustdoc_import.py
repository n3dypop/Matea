"""Regress the Windows checkout/import newline mismatch observed in CI."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from import_rustdoc import canonical_fact_bytes


class CanonicalImportTests(unittest.TestCase):
    def test_json_hash_is_identical_for_lf_crlf_and_utf8_payloads(self):
        value = {"name": "抹茶", "source_hash": "unchanged-producer-hash", "items": [1, 2]}
        text = json.dumps(value, indent=2, ensure_ascii=False) + "\n"
        with tempfile.TemporaryDirectory() as directory:
            lf, crlf = Path(directory) / "lf.json", Path(directory) / "crlf.json"
            lf.write_bytes(text.encode("utf-8"))
            crlf.write_bytes(text.replace("\n", "\r\n").encode("utf-8"))
            canonical = canonical_fact_bytes(crlf)
            self.assertEqual(canonical, canonical_fact_bytes(lf))
            self.assertNotIn(b"\r\n", canonical)
            self.assertEqual(json.loads(canonical), value)
            stored = Path(directory) / "stored.json"
            stored.write_bytes(canonical)
            self.assertEqual(hashlib.sha256(stored.read_bytes()).digest(), hashlib.sha256(canonical_fact_bytes(lf)).digest())


if __name__ == "__main__":
    unittest.main()
