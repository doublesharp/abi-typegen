#!/usr/bin/env python3
"""Run Unity compatibility checks and reject skipped or missing results."""
import argparse
import os
import plistlib
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

PROJECT = Path(__file__).resolve().parent
ROOT = PROJECT.parents[2]
PREFIX = "AbiTypegen.Unity.E2E.Tests."


def validate_results(path, required_suites):
    tree = ET.parse(path).getroot()
    cases = list(tree.iter("test-case"))
    if tree.get("result") != "Passed" or not cases:
        raise RuntimeError(f"Unity tests failed or produced no cases: {path}")
    if any(case.get("result") != "Passed" for case in cases):
        raise RuntimeError(f"Unity tests contain failed or skipped cases: {path}")
    classes = {case.get("classname") for case in cases}
    missing = set(required_suites) - classes
    if missing:
        raise RuntimeError(f"Unity did not run required suites: {sorted(missing)}")
    print(f"Unity results: {len(cases)} passed ({path})")


def run_tests(editor, mode):
    if mode == "playmode":
        for name in ("ATG_RPC_URL", "ATG_TOKEN_ADDRESS", "ATG_PRIVATE_KEY", "ATG_CHAIN_ID"):
            if not os.environ.get(name):
                raise RuntimeError(f"{name} is required; run PlayMode through e2e/native/anvil.py")
    results = PROJECT / "TestResults"
    results.mkdir(exist_ok=True)
    xml = results / f"{mode}.xml"
    xml.unlink(missing_ok=True)
    subprocess.run([
        editor, "-batchmode", "-nographics", "-projectPath", str(PROJECT),
        "-runTests", "-testPlatform", "EditMode" if mode == "editmode" else "PlayMode",
        "-testResults", str(xml), "-logFile", str(results / f"{mode}.log"),
    ], check=True, timeout=900)
    suites = ["GeneratedBindingsEditModeTests"] if mode == "editmode" else [
        "AnvilPlayModeTests", "LifecyclePlayModeTests"
    ]
    validate_results(xml, [PREFIX + suite for suite in suites])


def mac_executable(bundle):
    with (bundle / "Contents/Info.plist").open("rb") as stream:
        name = plistlib.load(stream)["CFBundleExecutable"]
    if not isinstance(name, str) or not name or name in (".", "..") or Path(name).name != name:
        raise RuntimeError("Invalid player executable name in app bundle")
    return bundle / "Contents/MacOS" / name


def validate_player_log(path):
    if "abi-typegen Unity AOT codec passed" not in path.read_text(errors="replace"):
        raise RuntimeError(f"Player exited without confirming codec assertions: {path}")


def run_player(editor, backend):
    if sys.platform.startswith("linux"):
        target = "Linux64"
        variant = "Mono" if backend == "mono" else "IL2CPP"
        executable = Path("AbiTypegenUnitySmoke.x86_64")
    elif sys.platform == "darwin":
        target = "Mac"
        variant = "MacMono" if backend == "mono" else "MacIL2CPP"
        executable = Path("AbiTypegenUnitySmoke.app/Contents/MacOS/AbiTypegenUnitySmoke")
    else:
        raise RuntimeError("Standalone qualification is configured for Linux and macOS")
    folder = PROJECT / "Build" / variant
    folder.mkdir(parents=True, exist_ok=True)
    player = folder / executable
    if sys.platform == "darwin" and (folder / "AbiTypegenUnitySmoke.app/Contents/Info.plist").is_file():
        player = mac_executable(folder / "AbiTypegenUnitySmoke.app")
    player.unlink(missing_ok=True)
    subprocess.run([
        editor, "-batchmode", "-nographics", "-quit", "-projectPath", str(PROJECT),
        "-executeMethod", "AbiTypegen.Unity.E2E.Editor.StandaloneBuild.Build" + target +
        ("Mono" if backend == "mono" else "Il2Cpp"),
        "-logFile", str(folder / "build.log"),
    ], check=True, timeout=1800)
    if sys.platform == "darwin":
        player = mac_executable(folder / "AbiTypegenUnitySmoke.app")
    player_log = folder / "player.log"
    player_log.unlink(missing_ok=True)
    subprocess.run([str(player), "-batchmode", "-nographics", "-logFile", str(player_log)],
                   check=True, timeout=120)
    validate_player_log(player_log)
    print(f"Unity {variant} player codec assertions passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["editmode", "playmode", "mono", "il2cpp", "all"])
    parser.add_argument("--editor", default=os.environ.get("UNITY_EDITOR"))
    args = parser.parse_args()
    if not args.editor or not Path(args.editor).is_file():
        parser.error("set UNITY_EDITOR to an installed, activated Unity 6000.6.3f1 executable")
    editor = str(Path(args.editor).resolve())
    if args.mode == "all":
        run_tests(editor, "editmode")
        subprocess.run([sys.executable, str(ROOT / "e2e/native/anvil.py"), sys.executable,
                        str(Path(__file__).resolve()), "playmode", "--editor", editor],
                       cwd=ROOT, check=True, timeout=360)
        run_player(editor, "mono")
        run_player(editor, "il2cpp")
    elif args.mode in ("editmode", "playmode"):
        run_tests(editor, args.mode)
    else:
        run_player(editor, args.mode)


if __name__ == "__main__":
    main()
