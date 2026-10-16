"""Regression tests for visibility/alias boundaries in compiler JSON ingestion."""
import unittest
from rustdoc_exports import FORMAT, exports


def item(identifier, name, kind, data, visibility="public", crate_id=0):
    return {"id": identifier, "crate_id": crate_id, "name": name, "span": None,
            "visibility": visibility, "inner": {kind: data}, "deprecation": None}


class ExportsTests(unittest.TestCase):
    def test_standard_blankets_are_distinct_from_local_generic_implementations(self):
        def implementation(trait):
            return {"trait": trait, "items": [], "is_synthetic": False,
                    "blanket_impl": {"generic": "T"}, "is_negative": False, "generics": {}}
        data = {"format_version": FORMAT, "root": 0,
                "paths": {"10": {"crate_id": 1, "path": ["core", "convert", "From"], "kind": "trait"}},
                "index": {
                    "0": item(0, "fixture", "module", {"items": [1]}),
                    "1": item(1, "Value", "struct", {"kind": "unit", "impls": [2, 3]}),
                    "2": item(2, None, "impl", implementation({"path": "From", "id": 10, "args": None})),
                    "3": item(3, None, "impl", implementation({"path": "Local", "id": 4, "args": None})),
                    "4": item(4, "Local", "trait", {"items": []}),
                }}
        data["index"]["3"]["span"] = {"filename": "/external/macro.rs", "begin": [1, 1], "end": [1, 10]}
        graph = exports(data, "/tmp/fixture")
        self.assertEqual(len(graph["trait_implementations"]), 1)
        self.assertEqual(graph["trait_implementations"][0]["trait"]["path"], "Local")
        self.assertEqual(graph["implicit_trait_summary"]["standard_blanket:core::convert::From"], 1)

    def test_public_types_in_private_modules_remain_callable_via_return_values(self):
        data = {"format_version": FORMAT, "root": 0,
                "paths": {"2": {"crate_id": 0, "path": ["fixture", "private", "Inferred"], "kind": "struct"}},
                "index": {
                    "0": item(0, "fixture", "module", {"items": [1]}),
                    "1": item(1, "expose", "function", {"sig": {"output": {"resolved_path": {"path": "Inferred", "id": 2, "args": None}}}}),
                    "2": item(2, "Inferred", "struct", {"kind": "unit", "impls": [3]}),
                    "3": item(3, None, "impl", {"trait": None, "items": [4]}),
                    "4": item(4, "method", "function", {}),
                }}
        graph = exports(data, "/tmp/fixture")
        method = next(i for i in graph["items"] if i["name"] == "method")
        self.assertFalse(method["path_is_nameable"])
        self.assertEqual(method["public_path"], "fixture::private::Inferred::method")

    def test_nested_alias_glob_private_field_and_trait_default(self):
        data = {"format_version": FORMAT, "root": 0, "paths": {}, "index": {
            "0": item(0, "fixture", "module", {"items": [1]}),
            "1": item(1, "nested", "module", {"items": [2, 3, 7]}),
            "2": item(2, None, "use", {"id": 4, "name": "Alias", "source": "private::Visible", "is_glob": False}),
            "3": item(3, None, "use", {"id": 5, "name": "*", "source": "private", "is_glob": True}),
            "4": item(4, "Visible", "struct", {"kind": {"plain": {"fields": [6, 8]}}, "impls": [10]}),
            "5": item(5, "private", "module", {"items": [4, 9]}, "default"),
            "6": item(6, "value", "struct_field", {"primitive": "u8"}),
            "7": item(7, "Contract", "trait", {"items": [11]}),
            "8": item(8, "secret", "struct_field", {"primitive": "u8"}, "default"),
            "9": item(9, "hidden", "function", {}, "default"),
            "10": item(10, None, "impl", {"trait": None, "items": [12, 13]}),
            "11": item(11, "provided", "function", {}, "default"),
            "12": item(12, "method", "function", {}),
            "13": item(13, "private_method", "function", {}, "default"),
        }}
        graph = exports(data, "/tmp/fixture")
        public = {i["public_path"] for i in graph["items"]}
        self.assertIn("fixture::nested::Alias", public)
        self.assertIn("fixture::nested::Alias::method", public)
        self.assertIn("fixture::nested::Contract::provided", public)
        self.assertNotIn("fixture::Alias", public)
        self.assertFalse(any(i["name"] in ["secret", "hidden", "private_method", "private"] for i in graph["items"]))
        self.assertTrue(any(e["relation"] == "glob_member" and e["to"] == "4" for e in graph["edges"]))

    def test_external_alias_and_schema_change_are_never_silently_resolved(self):
        data = {"format_version": FORMAT, "root": 0,
                "paths": {"9": {"crate_id": 1, "path": ["other", "Thing"], "kind": "struct"}},
                "index": {"0": item(0, "fixture", "module", {"items": [1]}),
                          "1": item(1, None, "use", {"id": 9, "name": "Alias", "source": "other::Thing", "is_glob": False})}}
        graph = exports(data, "/tmp/fixture")
        self.assertEqual(graph["external_exports"][0]["public_path"], "fixture::Alias")
        self.assertEqual(graph["status"], "compiled_profile_only_not_full_scope")
        key = next(i["profile_item_key"] for i in graph["items"] if i["kind"] == "use")
        data["paths"]["99"] = data["paths"].pop("9")
        data["index"]["1"]["inner"]["use"]["id"] = 99
        again = exports(data, "/tmp/fixture")
        self.assertEqual(key, next(i["profile_item_key"] for i in again["items"] if i["kind"] == "use"))
        data["format_version"] += 1
        with self.assertRaises(ValueError):
            exports(data, "/tmp/fixture")


if __name__ == "__main__":
    unittest.main()
