# Engine integrations

The engine adapters connect generated abi-typegen bindings to each engine's
runtime, request lifecycle, and application-owned transaction submission. Shared
operations provide simulation, gas estimation, confirmed receipts, and HTTP event
polling. Generated types and decoders remain contract-specific; providers, signers,
and application behavior are supplied by the consuming project.

| Engine | Adapter                                                                         | Usage guide                                                        |
| ------ | ------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Unity  | [`com.doublesharp.abi-typegen.unity`](unity/com.doublesharp.abi-typegen.unity/) | [Unity adapter](unity/com.doublesharp.abi-typegen.unity/README.md) |
| Unreal | [`AbiTypegen` plugin](unreal/Plugins/AbiTypegen/)                               | [Unreal adapter](unreal/README.md)                               |
| Godot  | [GDExtension](godot/extension/) and [shared addon](godot/addon/)                 | [Godot operations](godot/addon/README.md)                         |
