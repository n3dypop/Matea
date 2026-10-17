"""Regress the Windows checkout/import newline mismatch observed in CI."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from import_rustdoc import canonical_fact_bytes, import_reports, plan_import


class CanonicalImportTests(unittest.TestCase):
    def fixture(self, directory, stem="fixture", **changes):
        directory.mkdir(parents=True, exist_ok=True)
        report = {"schema": 1, "reference": "fixture", "sha": "pin", "package": "example",
                  "profile": "default", "target": "test-target", "extractor_sha256": "extractor",
                  "status": "compiled_profile_only_not_full_scope", "exit_code": 0,
                  "rustdoc_exit_code": 0, "dependency_tree_exit_code": 0}
        graph = {**{k: report[k] for k in ("reference", "sha", "package", "profile", "target", "extractor_sha256")},
                 "items": [{"profile_item_key": "one"}], "external_exports": []}
        report.update(changes)
        (directory / (stem + "-run.json")).write_text(json.dumps(report), encoding="utf-8")
        (directory / (stem + "-exports.json")).write_text(json.dumps(graph), encoding="utf-8")

    def test_invalid_later_profile_does_not_change_existing_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, destination = root / "source", root / "published"
            self.fixture(source, "a")
            import_reports([source], destination, {"fixture": "pin"})
            before = {p.name: p.read_bytes() for p in destination.iterdir()}
            self.fixture(source, "z", exit_code=1)
            with self.assertRaises(AssertionError):
                import_reports([source], destination, {"fixture": "pin"})
            self.assertEqual(before, {p.name: p.read_bytes() for p in destination.iterdir()})

    def test_conflicting_same_filename_and_profile_identity_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            a, b, destination = root / "a", root / "b", root / "out"
            self.fixture(a)
            self.fixture(b, target="other-target")
            with self.assertRaises(AssertionError):
                plan_import([a, b], destination, {"fixture": "pin"})
            self.assertFalse(destination.exists())
            self.fixture(b)
            graph = b / "fixture-exports.json"
            value = json.loads(graph.read_text())
            value["items"].append({"profile_item_key": "two"})
            graph.write_text(json.dumps(value))
            with self.assertRaisesRegex(AssertionError, "conflicting input file"):
                plan_import([a, b], destination, {"fixture": "pin"})

    def test_duplicate_keys_missing_empty_and_documentation_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, destination = root / "in", root / "out"
            for items in [[{"profile_item_key": "same"}] * 2, [{"profile_item_key": "one", "docs": "private body"}]]:
                self.fixture(source)
                graph = source / "fixture-exports.json"
                value = json.loads(graph.read_text()); value["items"] = items
                graph.write_text(json.dumps(value))
                with self.assertRaises(AssertionError):
                    plan_import([source], destination, {"fixture": "pin"})
            with self.assertRaises(AssertionError):
                plan_import([root / "missing"], destination, {"fixture": "pin"})
            empty = root / "empty"; empty.mkdir()
            with self.assertRaises(AssertionError):
                plan_import([empty], destination, {"fixture": "pin"})

    def test_successful_import_is_canonical_idempotent_and_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root / "in")
            first = import_reports([root / "in"], root / "out", {"fixture": "pin"})
            expected = {p.name: p.read_bytes() for p in (root / "out").iterdir()}
            second = import_reports([root / "in"], root / "out", {"fixture": "pin"})
            self.assertEqual(first, second)
            self.assertEqual(expected, {p.name: p.read_bytes() for p in (root / "out").iterdir()})
            self.assertEqual(first["snapshots"][0]["local_items"], 1)
            self.assertFalse(first["target_feature_closure_reviewed"])

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
