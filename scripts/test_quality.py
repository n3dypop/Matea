"""Ensure quality gates cannot hide missing programs or poor dimensions."""
import copy
import unittest
from check_quality import DIMENSIONS, assess


class QualityGateTests(unittest.TestCase):
    def fixture(self):
        return {"units": [{"id": "unit", "files": ["a.rs"], "evidence": ["proof.json"], "remaining": ["finite scope"],
                           "criteria": {d: {"score": 75, "reason": "bounded independent evidence"} for d in DIMENSIONS}}]}

    def test_low_dimension_is_not_hidden_by_other_high_scores(self):
        plan = self.fixture()
        self.assertTrue(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["memory_safety"]["score"] = 25
        result = assess(plan, {"a.rs"})
        self.assertFalse(result["all_high"])
        self.assertEqual(len(result["below_high"]), 1)

    def test_new_files_and_missing_dimensions_are_rejected(self):
        plan = self.fixture()
        with self.assertRaises(ValueError): assess(plan, {"a.rs", "new.rs"})
        with self.assertRaises(ValueError): assess(plan, set())
        broken = copy.deepcopy(plan)
        del broken["units"][0]["criteria"]["performance"]
        with self.assertRaises(ValueError): assess(broken, {"a.rs"})

    def test_not_applicable_requires_an_explanation_and_is_distinct_from_unmeasured(self):
        plan = self.fixture()
        plan["units"][0]["criteria"]["reference"] = {"score": None, "reason": "original audit utility; no reference counterpart"}
        self.assertTrue(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["performance"] = {"score": 0, "reason": "unmeasured"}
        self.assertFalse(assess(plan, {"a.rs"})["all_high"])
        plan["units"][0]["criteria"]["reference"]["reason"] = ""
        with self.assertRaises(ValueError): assess(plan, {"a.rs"})


if __name__ == "__main__":
    unittest.main()
