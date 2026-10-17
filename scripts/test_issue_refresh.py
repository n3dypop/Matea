"""Independent fault/identity fixtures for issue research state preservation."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from fetch_upstream_issues import atomic_json, merge_repository, refresh


def issue(number, **changes):
    return {"number": number, "title": "報告", "html_url": f"https://github.com/owner/project/issues/{number}",
            "updated_at": "old", "labels": [{"name": "bug"}], **changes}


class RefreshTests(unittest.TestCase):
    def test_reviews_survive_metadata_changes_disappearance_and_reopening(self):
        first = merge_repository("owner/project", [[issue(2), issue(1)]], {})
        first["issues"][0].update(disposition="fixed", matea_reproduction="confirmed", evidence=["regression"])
        saved = copy.deepcopy(first)
        second = merge_repository("owner/project", [[issue(2, updated_at="new")]], first)
        self.assertEqual(first, saved)
        self.assertEqual(second["issues"][0]["number"], 2)
        self.assertTrue(second["issues"][0]["upstream_updated_since_previous_snapshot"])
        self.assertEqual(second["no_longer_open"][0]["disposition"], "fixed")
        third = merge_repository("owner/project", [[issue(1)]], second)
        self.assertEqual(third["issues"][0]["evidence"], ["regression"])
        self.assertEqual(third["issues"][0]["matea_reproduction"], "confirmed")
        self.assertNotIn("listing_status", third["issues"][0])
        self.assertEqual(third["no_longer_open"][0]["number"], 2)

    def test_pagination_excludes_prs_but_rejects_duplicate_or_foreign_identity(self):
        result = merge_repository("owner/project", [[{**issue(7), "pull_request": {}}], [issue(3)]], {})
        self.assertEqual(result["open_issue_count"], 1)
        self.assertEqual(result["page_count"], 2)
        for pages in [[[issue(3)], [issue(3)]], [[issue(3, html_url="https://github.com/other/repo/issues/3")]],
                      [[issue(True)]], {}, [None]]:
            with self.subTest(pages=pages), self.assertRaises((AssertionError, TypeError)):
                merge_repository("owner/project", pages, {})

    def test_failed_later_fetch_keeps_input_and_published_evidence(self):
        previous = {"schema": 1, "repositories": []}
        saved = copy.deepcopy(previous)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "issues.json"
            atomic_json(path, previous)
            before = path.read_bytes()
            def fetch(repo):
                if repo == "second":
                    raise TimeoutError("offline fixture")
                return []
            with patch("fetch_upstream_issues.REPOSITORIES", ["first", "second"]):
                with self.assertRaises(TimeoutError):
                    atomic_json(path, refresh(previous, fetch))
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(previous, saved)
            self.assertEqual(list(path.parent.iterdir()), [path])

    def test_failed_replace_keeps_old_file_and_cleans_temporary(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "issues.json"
            atomic_json(path, {"old": "抹茶"})
            before = path.read_bytes()
            with patch("fetch_upstream_issues.os.replace", side_effect=PermissionError("fixture")):
                with self.assertRaises(PermissionError):
                    atomic_json(path, {"new": True})
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(json.loads(before), {"old": "抹茶"})
            self.assertEqual(list(path.parent.iterdir()), [path])


if __name__ == "__main__":
    unittest.main()
