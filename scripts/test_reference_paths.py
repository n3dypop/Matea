"""The cross-OS wrapper must preserve established numeric acceptance contracts."""
import unittest
from check_reference_paths import check_trajectories


class TrajectoryChecks(unittest.TestCase):
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
