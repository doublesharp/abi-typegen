class_name AbiTypegenTestUtil
extends RefCounted

var failures: Array[String] = []

func check(condition: bool, message: String) -> void:
    if condition:
        return
    failures.append(message)
    printerr("FAIL: ", message)

func equal(actual, expected, message: String) -> void:
    if actual == expected:
        return
    failures.append("%s: expected %s, got %s" % [message, str(expected), str(actual)])
    printerr("FAIL: ", failures[-1])

func ok(outcome: Dictionary, message: String) -> bool:
    if outcome.get("ok", false):
        return true
    check(false, "%s: %s" % [message, str(outcome.get("error", {}))])
    return false
