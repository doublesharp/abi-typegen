extends SceneTree

const CodecTests = preload("res://tests/codec_tests.gd")
const AnvilTests = preload("res://tests/anvil_tests.gd")
const PackagingTests = preload("res://tests/packaging_tests.gd")

func _initialize() -> void:
    call_deferred("_run")

func _selected_suite() -> String:
    for arg in OS.get_cmdline_user_args():
        if arg.begins_with("--suite="):
            return arg.substr(8)
    return "all"

func _print_failures(label: String, failures: Array[String]) -> int:
    if failures.is_empty():
        print("PASS ", label)
        return 0
    printerr("FAIL ", label, ": ", failures.size(), " failure(s)")
    for failure in failures:
        printerr("  - ", failure)
    return failures.size()

func _run() -> void:
    var suite := _selected_suite()
    var failure_count := 0

    if suite in ["all", "codec"]:
        failure_count += _print_failures("codec", CodecTests.new().run())

    if suite in ["all", "packaging"]:
        failure_count += _print_failures("packaging", PackagingTests.new().run())

    if suite in ["all", "anvil"]:
        var anvil_failures: Array[String] = await AnvilTests.new().run(self)
        failure_count += _print_failures("anvil", anvil_failures)

    if not (suite in ["all", "codec", "packaging", "anvil"]):
        printerr("Unknown suite: ", suite)
        quit(2)
        return

    quit(0 if failure_count == 0 else 1)
