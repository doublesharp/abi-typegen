import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


RUNNER = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("unreal_runner", RUNNER)
unreal_runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(unreal_runner)


def report(states=None):
    states = states or {}
    tests = [
        {
            "fullTestPath": name,
            "testDisplayName": name.rsplit(".", 1)[-1],
            "state": states.get(name, "Success"),
            "warnings": 0,
            "errors": 0,
        }
        for name in unreal_runner.EXPECTED_TESTS
    ]
    return {
        "succeeded": len(tests),
        "succeededWithWarnings": 0,
        "failed": 0,
        "notRun": 0,
        "inProcess": 0,
        "tests": tests,
    }


class AutomationReportTests(unittest.TestCase):
    def test_engine_exit_argument_matches_automation_completion_text(self):
        command = unreal_runner.editor_test_command(Path("UnrealEditor-Cmd"), "arm64", Path("report"))
        self.assertIn("-TestExit=Automation Test Queue Empty", command)

    def test_all_expected_tests_must_pass(self):
        self.assertEqual(unreal_runner.report_issues(report()), [])

    def test_missing_and_duplicate_tests_fail(self):
        missing = report()
        missing["tests"].pop()
        self.assertTrue(any("missing expected" in issue for issue in unreal_runner.report_issues(missing)))

        duplicate = report()
        duplicate["tests"].append(duplicate["tests"][0].copy())
        self.assertTrue(any("duplicate test" in issue for issue in unreal_runner.report_issues(duplicate)))

    def test_failed_skipped_unrun_or_in_process_tests_fail(self):
        for state in ("Fail", "Skipped", "NotRun", "InProcess"):
            with self.subTest(state=state):
                fixture = report({unreal_runner.EXPECTED_TESTS[0]: state})
                self.assertTrue(unreal_runner.report_issues(fixture))

    def test_failure_counters_fail_even_if_individual_statuses_look_green(self):
        fixture = report()
        fixture["failed"] = 1
        self.assertTrue(any("Failed" in issue for issue in unreal_runner.report_issues(fixture)))
        fixture = report()
        fixture["notRun"] = 1
        self.assertTrue(any("NotRun" in issue for issue in unreal_runner.report_issues(fixture)))
        fixture = report()
        del fixture["inProcess"]
        self.assertTrue(any("InProcess" in issue for issue in unreal_runner.report_issues(fixture)))

    def test_loads_the_engine_utf8_bom(self):
        fixture = report()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "index.json"
            path.write_bytes(b"\xef\xbb\xbf" + json.dumps(fixture).encode())
            self.assertEqual(unreal_runner.report_issues(unreal_runner.load_report(path)), [])

    def test_per_test_errors_and_warnings_are_checked(self):
        fixture = report()
        fixture["tests"][0]["warnings"] = 1
        self.assertTrue(any("warnings" in issue for issue in unreal_runner.report_issues(fixture)))
        fixture = report()
        fixture["tests"][0]["errors"] = 1
        self.assertTrue(any("errors" in issue for issue in unreal_runner.report_issues(fixture)))

    def test_rejects_unexpected_test_names_in_suite(self):
        fixture = report()
        fixture["tests"].append({"fullTestPath": "AbiTypegen.Unreal.Codec.Unexpected", "state": "Success"})
        self.assertTrue(any("unexpected test" in issue for issue in unreal_runner.report_issues(fixture)))


if __name__ == "__main__":
    unittest.main()
