"""Regression tests for persistent disposable-storage configuration."""

import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "scratch", Path(__file__).resolve().parents[1] / ".cargo/setup-scratch.py"
)
scratch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scratch)


class ScratchTests(unittest.TestCase):
    """Exercise real directory migration and generated settings."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.repo = self.base / "checkout"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.storage = self.base / "custom $storage's #1 🛠"

    def configure(self, storage=None, volume=None):
        return scratch.configure(self.repo, storage, volume)

    def test_custom_root_persists_and_reruns_without_arguments(self):
        self.configure(self.storage)
        self.configure()
        self.assertEqual((self.repo / "target").resolve(), self.storage / "target")
        import tomllib

        cargo = tomllib.loads((self.repo / ".cargo/config.toml").read_text())
        self.assertEqual(cargo["build"]["target-dir"], str(self.storage / "target"))
        self.assertEqual(cargo["env"]["TMPDIR"]["value"], str(self.storage / "tmp"))
        pnpm = (self.repo / "npm/pnpm-workspace.yaml").read_text()
        self.assertIn(str(self.storage / "npm/node_modules"), pnpm)
        result = subprocess.check_output(
            ["make", "--no-print-directory", "-f", "-", "print"],
            cwd=self.repo,
            # Exercise recursive Make's inherited directory logging, as in CI.
            env={**os.environ, "MAKEFLAGS": "w", "MAKELEVEL": "1"},
            input="include .cargo/scratch.local.mk\nprint:\n\t@printf '%s' \"$$TMPDIR\"\n",
            text=True,
        )
        self.assertEqual(result, str(self.storage / "tmp"))

    def test_switching_roots_preserves_existing_corpus(self):
        self.configure(self.storage)
        corpus = self.repo / "fuzz/corpus/sample"
        corpus.write_bytes(b"valuable discovered input\x00")
        alternate = self.base / "second disk"
        self.configure(alternate)
        self.assertEqual(corpus.read_bytes(), b"valuable discovered input\x00")
        self.assertEqual(corpus.resolve(), alternate / "fuzz/corpus/sample")
        self.configure()

    def test_tracked_paths_are_rejected_before_mutations(self):
        (self.repo / "target").mkdir()
        (self.repo / "target/keep").write_text("source")
        subprocess.run(["git", "add", "target/keep"], cwd=self.repo, check=True)
        with self.assertRaisesRegex(ValueError, "tracked"):
            self.configure(self.storage)
        self.assertFalse(self.storage.exists())
        self.assertEqual((self.repo / "target/keep").read_text(), "source")

    def test_existing_configuration_is_not_overwritten(self):
        (self.repo / ".cargo").mkdir()
        config = self.repo / ".cargo/config.toml"
        config.write_text('[alias]\nhello = "check"\n')
        with self.assertRaisesRegex(ValueError, "configuration"):
            self.configure(self.storage)
        self.assertIn("[alias]", config.read_text())
        self.assertFalse(self.storage.exists())

    def test_conflicting_destination_preserves_both_copies(self):
        self.configure(self.storage)
        alternate = self.base / "other"
        (alternate / "target").mkdir(parents=True)
        (alternate / "target/keep").write_text("other")
        with self.assertRaisesRegex(ValueError, "existing"):
            self.configure(alternate)
        self.assertEqual((self.repo / "target").resolve(), self.storage / "target")
        self.assertEqual((alternate / "target/keep").read_text(), "other")

    def test_destination_file_is_rejected_before_any_links_are_created(self):
        self.storage.mkdir()
        (self.storage / "coverage").write_text("keep")
        with self.assertRaisesRegex(ValueError, "existing"):
            self.configure(self.storage)
        self.assertFalse((self.repo / "target").is_symlink())
        self.assertEqual((self.storage / "coverage").read_text(), "keep")

    def test_required_mount_must_be_mounted(self):
        with patch.object(os.path, "ismount", return_value=False):
            with self.assertRaisesRegex(ValueError, "mounted"):
                self.configure(self.storage, self.base)
        self.assertFalse(self.storage.exists())

    def test_storage_inside_checkout_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "outside"):
            self.configure(self.repo / "disposable")

    def test_saved_mount_is_checked_on_subsequent_runs(self):
        with patch.object(os.path, "ismount", return_value=True):
            self.configure(self.storage, self.base)
        with patch.object(os.path, "ismount", return_value=False):
            with self.assertRaisesRegex(ValueError, "mounted"):
                self.configure()

    def test_overlapping_roots_are_rejected_without_copying_corpus_into_itself(self):
        self.configure(self.storage)
        with self.assertRaisesRegex(ValueError, "overlap"):
            self.configure(self.storage / "fuzz/corpus/nested")
        self.assertEqual((self.repo / "target").resolve(), self.storage / "target")

    def test_shell_environment_preserves_special_characters(self):
        self.configure(self.storage)
        actual = subprocess.check_output(
            ["sh", "-c", '. ./.cargo/scratch.local.env; printf "%s" "$TMPDIR"'],
            cwd=self.repo,
            text=True,
        )
        self.assertEqual(actual, str(self.storage / "tmp"))

    def test_source_parent_symlink_cannot_modify_external_directories(self):
        external = self.base / "external"
        (external / "node_modules").mkdir(parents=True)
        (external / "node_modules/keep").write_text("keep")
        (self.repo / "npm").symlink_to(external, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.configure(self.storage)
        self.assertEqual((external / "node_modules/keep").read_text(), "keep")
        self.assertFalse((external / "node_modules").is_symlink())
        self.assertFalse(self.storage.exists())

    def test_configuration_parent_symlink_is_rejected(self):
        external = self.base / "external"
        external.mkdir()
        (self.repo / ".cargo").symlink_to(external, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.configure(self.storage)
        self.assertEqual(list(external.iterdir()), [])

    def test_settings_symlink_is_not_overwritten(self):
        external = self.base / "settings.json"
        external.write_text("{}")
        (self.repo / ".cargo").mkdir()
        (self.repo / scratch.SETTINGS).symlink_to(external)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.configure(self.storage)
        self.assertEqual(external.read_text(), "{}")

    def test_cache_and_temporary_symlinks_cannot_escape_storage(self):
        external = self.base / "external"
        external.mkdir()
        self.storage.mkdir()
        for relative in ("tmp", "cache", "logs", "sccache.sock"):
            link = self.storage / relative
            link.symlink_to(external, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "symlink|escapes"):
                self.configure(self.storage)
            link.unlink()
        self.assertEqual(list(external.iterdir()), [])
        self.assertFalse((self.repo / "target").exists())

    def test_disable_restores_defaults_and_keeps_saved_data(self):
        self.configure(self.storage)
        (self.repo / "target/keep").write_text("build output")
        (self.repo / "fuzz/corpus/keep").write_text("discovered input")
        (self.repo / ".doublcov/keep").write_text("coverage history")
        scratch.disable(self.repo)
        for relative in (*scratch.generated_files(self.storage), scratch.SETTINGS):
            self.assertFalse((self.repo / relative).exists())
        self.assertFalse((self.repo / "target").is_symlink())
        self.assertEqual((self.storage / "target/keep").read_text(), "build output")
        self.assertEqual(
            (self.repo / "fuzz/corpus/keep").read_text(), "discovered input"
        )
        self.assertEqual((self.repo / ".doublcov/keep").read_text(), "coverage history")
        self.assertEqual(
            (self.storage / "fuzz/corpus/keep").read_text(), "discovered input"
        )
        scratch.disable(self.repo)  # Already disabled is a no-op.

    def test_disable_refuses_unknown_links_before_removing_any_configuration(self):
        self.configure(self.storage)
        link = self.repo / "coverage"
        link.unlink()
        link.symlink_to(self.base / "unknown", target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            scratch.disable(self.repo)
        self.assertTrue((self.repo / scratch.SETTINGS).exists())
        self.assertTrue((self.repo / "target").is_symlink())

    def test_unconfigured_make_test_does_not_require_python_or_scratch(self):
        shutil.copy(Path(__file__).resolve().parents[1] / "Makefile", self.repo)
        output = subprocess.check_output(
            ["make", "--no-print-directory", "-n", "test"],
            cwd=self.repo,
            env={**os.environ, "MAKEFLAGS": "w", "MAKELEVEL": "1"},
            text=True,
        )
        self.assertEqual(output.strip(), "cargo test --all")
        self.assertFalse((self.repo / scratch.SETTINGS).exists())

    def test_unconfigured_wrapper_runs_with_normal_environment(self):
        source = Path(__file__).resolve().parents[1] / ".cargo/setup-scratch.py"
        (self.repo / ".cargo").mkdir()
        shutil.copy(source, self.repo / ".cargo/setup-scratch.py")
        output = subprocess.check_output(
            [
                sys.executable,
                str(self.repo / ".cargo/setup-scratch.py"),
                "--run",
                sys.executable,
                "-c",
                "import os; print(os.environ['TMPDIR'])",
            ],
            cwd=self.repo,
            env={**os.environ, "TMPDIR": str(self.base)},
            text=True,
        )
        self.assertEqual(output.strip(), str(self.base))
        self.assertFalse((self.repo / scratch.SETTINGS).exists())

    def test_malformed_saved_settings_fail_before_mutation(self):
        (self.repo / ".cargo").mkdir()
        for settings in (
            [],
            {},
            {"root": 1},
            {"root": "relative"},
            {"root": str(self.storage), "volume": 7},
        ):
            (self.repo / scratch.SETTINGS).write_text(json.dumps(settings))
            with self.assertRaises(ValueError):
                self.configure()
            self.assertFalse(self.storage.exists())

    def test_disable_preserves_modified_generated_configuration(self):
        self.configure(self.storage)
        config = self.repo / ".cargo/config.toml"
        config.write_text(config.read_text() + "# user changes\n")
        with self.assertRaisesRegex(ValueError, "modified"):
            scratch.disable(self.repo)
        self.assertTrue((self.repo / "target").is_symlink())
        self.assertTrue(config.read_text().endswith("# user changes\n"))

    def test_disabled_storage_can_be_enabled_with_a_fresh_root(self):
        self.configure(self.storage)
        (self.repo / "fuzz/corpus/keep").write_text("keep")
        scratch.disable(self.repo)
        fresh = self.base / "fresh"
        self.configure(fresh)
        self.assertEqual(
            (self.repo / "fuzz/corpus/keep").resolve(), fresh / "fuzz/corpus/keep"
        )
        self.assertEqual((self.repo / "fuzz/corpus/keep").read_text(), "keep")

    @unittest.skipUnless(
        shutil.which("cargo"), "Cargo required for default-path integration check"
    )
    def test_cargo_uses_default_paths_before_setup_and_after_disable(self):
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname="storage-test"\nversion="0.1.0"\nedition="2021"\n'
        )
        (self.repo / "src").mkdir()
        (self.repo / "src/lib.rs").write_text("")
        env = {**os.environ, "RUSTC_WRAPPER": ""}
        env.pop("CARGO_TARGET_DIR", None)

        def target():
            data = subprocess.check_output(
                ["cargo", "metadata", "--offline", "--no-deps", "--format-version=1"],
                cwd=self.repo,
                env=env,
                text=True,
            )
            return Path(json.loads(data)["target_directory"])

        self.assertEqual(target(), self.repo / "target")
        self.configure(self.storage)
        self.assertEqual(target(), self.storage / "target")
        scratch.disable(self.repo)
        self.assertEqual(target(), self.repo / "target")


if __name__ == "__main__":
    unittest.main()
