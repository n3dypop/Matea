"""Regression: optimized Python must not silently disable evidence validation."""
import json
import ast
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class EvidenceChecksTests(unittest.TestCase):
    def test_production_evidence_programs_have_no_removable_assert_statements(self):
        for path in (ROOT / "scripts").glob("*.py"):
            if path.name.startswith("test_"):
                continue
            self.assertFalse(any(isinstance(n, ast.Assert) for n in ast.walk(ast.parse(path.read_text(encoding="utf-8")))), path.name)

    def test_failed_check_is_preserved_in_optimized_python(self):
        result = subprocess.run([sys.executable, "-O", "-c", "from evidence_checks import require; require(False, lambda: 'rejected')"], cwd=ROOT / "scripts", capture_output=True, text=True, timeout=30)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("rejected", result.stderr)

    def test_terminal_fixture_mismatch_is_rejected_in_both_interpreter_modes(self):
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            trace.write_text(json.dumps({"width": 80, "height": 24, "sequences": [""]}), encoding="utf-8")
            for flags in [[], ["-O"]]:
                result = subprocess.run([sys.executable, *flags, str(ROOT / "scripts/check_terminal_trace.py"), str(trace)], capture_output=True, text=True, timeout=30)
                self.assertNotEqual(result.returncode, 0, flags)
                self.assertNotIn("exact states", result.stdout, flags)


if __name__ == "__main__":
    unittest.main()
