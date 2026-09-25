import importlib.util
import unittest
from pathlib import Path


RUNNER = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("godot_runner", RUNNER)
godot_runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(godot_runner)


class RunnerOutputTests(unittest.TestCase):
    def test_requires_suite_marker(self):
        self.assertTrue(godot_runner.suite_passed("PASS codec\n", "codec"))
        self.assertFalse(godot_runner.suite_passed("Godot exited successfully\n", "codec"))
        self.assertFalse(godot_runner.suite_passed("PASS packaging\n", "codec"))

    def test_rejects_failure_diagnostics_even_with_pass_marker(self):
        self.assertTrue(godot_runner.has_extension_load_failure("ERROR: Failed to load GDExtension"))
        self.assertTrue(godot_runner.has_extension_load_failure("Cannot load GDExtension library"))
        self.assertTrue(godot_runner.has_extension_load_failure("ERROR: No GDExtension library found for current OS and architecture"))
        self.assertTrue(godot_runner.has_extension_load_failure("ERROR: GDExtension dynamic library not found"))
        self.assertTrue(godot_runner.has_failure_diagnostic("PASS codec\nSCRIPT ERROR: parse failed\n"))
        self.assertTrue(godot_runner.has_failure_diagnostic("PASS codec\nERROR: extension unavailable\n"))
        self.assertFalse(godot_runner.has_extension_load_failure("PASS codec\n"))

    def test_rejects_test_failures(self):
        self.assertFalse(godot_runner.suite_passed("PASS codec\nFAIL codec: 1 failure(s)\n", "codec"))
        self.assertFalse(godot_runner.suite_passed("PASS codec with caveat\n", "codec"))


if __name__ == "__main__":
    unittest.main()
