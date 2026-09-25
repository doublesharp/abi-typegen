"""Tests for Unity result validation; no Editor required."""
import importlib.util
from pathlib import Path
import tempfile
import plistlib
import unittest

spec = importlib.util.spec_from_file_location("unity_runner", Path(__file__).with_name("run.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ResultsTests(unittest.TestCase):
    def validate(self, xml):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "results.xml"
            path.write_text(xml)
            runner.validate_results(path, ["RequiredTests"])

    def test_passed_required_suite(self):
        self.validate('<test-run result="Passed"><test-case classname="RequiredTests" result="Passed" /></test-run>')

    def test_player_must_report_codec_success(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "player.log"
            path.write_text("player exited normally")
            with self.assertRaises(RuntimeError):
                runner.validate_player_log(path)
            path.write_text("abi-typegen Unity AOT codec passed")
            runner.validate_player_log(path)

    def test_mac_player_uses_bundle_executable_name(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "Smoke.app"
            (bundle / "Contents").mkdir(parents=True)
            with (bundle / "Contents/Info.plist").open("wb") as stream:
                plistlib.dump({"CFBundleExecutable": "unity"}, stream)
            self.assertEqual(runner.mac_executable(bundle), bundle / "Contents/MacOS/unity")

    def test_rejects_empty_skipped_failed_and_missing_suite(self):
        for xml in [
            '<test-run result="Passed" />',
            '<test-run result="Passed"><test-case classname="RequiredTests" result="Skipped" /></test-run>',
            '<test-run result="Failed"><test-case classname="RequiredTests" result="Failed" /></test-run>',
            '<test-run result="Passed"><test-case classname="Other" result="Passed" /></test-run>',
        ]:
            with self.subTest(xml=xml), self.assertRaises(RuntimeError):
                self.validate(xml)


if __name__ == "__main__":
    unittest.main()
