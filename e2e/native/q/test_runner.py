"""Regression: q can exit zero after a script error; missing completion must fail."""
from pathlib import Path
import tempfile
import unittest
from run_q import run_q


class RunnerTest(unittest.TestCase):
    def test_script_error_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            script = Path(directory) / "failure.q"
            script.write_text("'\"intentional failure\";\n-1 \"UNREACHED\";\nexit 0;\n")
            with self.assertRaises(RuntimeError):
                run_q(script, "UNREACHED")


if __name__ == "__main__":
    unittest.main()
