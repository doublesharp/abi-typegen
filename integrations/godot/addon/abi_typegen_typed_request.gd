## Converts a pending request's successful value while retaining its error envelope.
class_name AbiTypegenTypedRequest
extends RefCounted

signal finished(outcome: Dictionary)
signal completed(value)
signal failed(error: Dictionary)
signal cancelled()

var _source
var _converter: Callable
var _done := false
var _keep_alive

## Converter may return a typed value or an {ok,value/error} outcome.
static func wrap(request, converter: Callable) -> AbiTypegenTypedRequest:
    var adapter := AbiTypegenTypedRequest.new()
    adapter._source = request
    adapter._converter = converter
    adapter._keep_alive = adapter
    if typeof(request) != TYPE_OBJECT or not is_instance_valid(request) or not converter.is_valid() or not request.has_signal("finished") or not request.has_method("cancel"):
        adapter.call_deferred("_finish", {"ok": false, "error": {"code": "invalid_typed_request", "message": "request and converter are required"}})
    elif request.has_method("is_done") and request.call("is_done"):
        adapter.call_deferred("_finish", {"ok": false, "error": {"code": "already_completed", "message": "wrap a pending request before completion"}})
    else:
        request.connect("finished", adapter._receive, CONNECT_ONE_SHOT)
        if request is Node:
            request.tree_exiting.connect(adapter._source_exiting, CONNECT_ONE_SHOT)
    return adapter

func _receive(outcome: Dictionary) -> void:
    if _done:
        return
    if not outcome.get("ok", false):
        _finish(outcome)
        return
    var converted = _converter.call(outcome.get("value"))
    if typeof(converted) == TYPE_DICTIONARY and converted.has("ok"):
        _finish(converted)
    else:
        _finish({"ok": true, "value": converted})

func _source_exiting() -> void:
    if not _done:
        _finish({"ok": false, "error": {"code": "cancelled", "message": "request owner was destroyed"}})

func _finish(outcome: Dictionary) -> void:
    if _done:
        return
    _done = true
    # Retain a local reference while listeners may release their own references.
    var retained = _keep_alive
    finished.emit(outcome)
    if outcome.get("ok", false):
        completed.emit(outcome.get("value"))
    else:
        var error: Dictionary = outcome.get("error", {"code": "conversion_failed"})
        failed.emit(error)
        if error.get("code") == "cancelled":
            cancelled.emit()
    _source = null
    _keep_alive = null
    retained = null

## Cancels the underlying pending request; finished is emitted exactly once.
func cancel() -> void:
    if _done:
        return
    if is_instance_valid(_source):
        _source.cancel()
    if not _done:
        _finish({"ok": false, "error": {"code": "cancelled", "message": "request cancelled"}})
