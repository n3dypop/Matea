"""The cross-OS wrapper must preserve established numeric acceptance contracts."""
import unittest
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
from unittest.mock import patch
from check_reference_paths import check_trajectories, check_timing_mode


class TrajectoryChecks(unittest.TestCase):
    def test_gradient_timing_groups_require_all_separate_probe_cases(self):
        for label,count,group in [(label,count,group) for label,count in [("upstream",6),("matea",12)] for group in ["gradients","progress"]]+[("upstream",14,"viewport-geometry"),("matea",18,"viewport-geometry")]:
            values={f"{group}/{i}":{"case":f"{group}/{i}","semantics":"finite","output":"colors","first_return_value":1} for i in range(count)}
            raw="\n".join(json.dumps({**v,"samples_ns":[1]*31,"iterations_per_sample":1,"allocations":0,"allocated_bytes":0,
                "calibration":{"target_ns":20_000_000,"start_n":1,"max_n":2,"batches":[{"iterations":1,"elapsed_ns":20_000_000,"confirmations_ns":[20_000_000]*2}]}}) for v in values.values()).encode()
            with tempfile.TemporaryDirectory() as directory,patch("check_reference_paths.subprocess.check_output",return_value=raw):
                destination=Path(directory)
                check_timing_mode(destination,Path("fixture"),{},[(["--"+group],values)],label=label,group=group)
                report=json.loads((destination/(label+"-"+{"gradients":"gradient","progress":"progress","viewport-geometry":"viewport-geometry"}[group]+"-timing-mode-checks.json")).read_text())
                self.assertEqual(len(report['checked_cases']),count)
                self.assertEqual(report['group'],group)
                self.assertEqual(report['invalid_configurations_rejected'],[])
    def test_timing_mode_uses_confirmed_target_and_retains_all_91_case_checks(self):
        cohorts=[([],84),(["--virtual-cursor"],7)]
        expected=[]
        for flags,count in cohorts:
            expected.append((flags,{f"case/{len(flags)}/{i}":{"case":f"case/{len(flags)}/{i}","semantics":"finite", "output":"value","first_return_value":1}
                                    for i in range(count)}))
        def output(command,env,timeout):
            self.assertEqual((env["MATEA_PERF_MIN_SAMPLE_NS"],timeout),("20000000",300))
            self.assertNotIn("MATEA_PROBE_OUTPUT_ONLY",env)
            values=expected[int(len(command)>1)][1]
            return "\n".join(json.dumps({**v,"samples_ns":[1]*31,"iterations_per_sample":1,"allocations":0,"allocated_bytes":0,
                "calibration":{"target_ns":20_000_000,"start_n":1,"max_n":2,"batches":[{"iterations":1,"elapsed_ns":20_000_000,"confirmations_ns":[20_000_000]*2}]}}) for v in values.values()).encode()
        with tempfile.TemporaryDirectory() as directory,patch("check_reference_paths.subprocess.check_output",side_effect=output),patch("check_reference_paths.subprocess.run",return_value=SimpleNamespace(returncode=2,stdout=b"")) as run:
            target=Path(directory)
            check_timing_mode(target,Path("fixture"),{"MATEA_PROBE_OUTPUT_ONLY":"1"},expected)
            report=json.loads((target/"timing-mode-checks.json").read_text())
            self.assertEqual(len(report["checked_cases"]),91)
            self.assertTrue(all(c["positive_samples"]==31 for c in report["checked_cases"]))
            check_timing_mode(target,Path("fixture"),{},expected,label="matea")
            rust=json.loads((target/"rust-timing-mode-checks.json").read_text())
            self.assertEqual(rust["probe"],"matea")
            self.assertEqual(rust["checked_cases"],report["checked_cases"])
            self.assertEqual(len(rust["invalid_configurations_rejected"]),5)
            self.assertEqual(run.call_count,5)
    def test_spring_input_identity_and_existing_absolute_bound(self):
        fixture="1.000001,12,1,0\n"
        result=check_trajectories("1.000001,12,1.0000005,0.0000005\n",fixture,True)
        self.assertEqual(result["rows"],1)
        self.assertGreater(result["maximum_absolute_error"],0)
        for actual in ["1.000002,12,1,0", "1.000001,12,1.000002,0", "1.000001,12,1,0.000001", "1.000001,12,1,inf", "1.000001,12,1,nan", "", fixture+fixture]:
            with self.subTest(actual=actual),self.assertRaises(AssertionError):
                check_trajectories(actual,fixture,True)

    def test_projectiles_keep_relative_bound_and_exact_inputs(self):
        fixture="100,2,0,0,0,1,2,3,4,5,6\n"
        result=check_trajectories(fixture,fixture,False)
        self.assertEqual(result["maximum_absolute_error"],0)
        for actual in [fixture.replace("100,2", "100,3"),fixture.replace(",6", ",6.00000001"),"100,2,0,0,0,1,2,3,4,5"]:
            with self.subTest(actual=actual),self.assertRaises(AssertionError):
                check_trajectories(actual,fixture,False)


if __name__=="__main__":
    unittest.main()
