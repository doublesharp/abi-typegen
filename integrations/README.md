# Engine integrations

The engine adapters connect generated abi-typegen bindings to each engine's
runtime, request lifecycle, and application-owned transaction submission.

| Engine | Adapter                                                                         | Usage guide                                                        |
| ------ | ------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Unity  | [`com.doublesharp.abi-typegen.unity`](unity/com.doublesharp.abi-typegen.unity/) | [Unity adapter](unity/com.doublesharp.abi-typegen.unity/README.md) |
| Unreal | [`AbiTypegen` plugin](unreal/Plugins/AbiTypegen/)                               | [Unreal bindings](../docs/native-bindings.md#unreal-experimental)  |
| Godot  | [GDExtension](godot/extension/)                                                 | [Godot bindings](../docs/native-bindings.md#godot-experimental)    |
