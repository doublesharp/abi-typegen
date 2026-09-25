#!/bin/bash
set -euo pipefail
# shellcheck disable=SC1090 # run.sh generates the source at a temporary path.
source "${ATG_SHELL_COLLISION:?collision binding is required}"

[[ $ATG_SHELL_COLLISION_CONSTRUCTOR_SIGNATURE == 'constructor(uint256)' ]]
constructor=$(atg_shell_collision_constructor_args 7)
[[ $constructor == 0x0000000000000000000000000000000000000000000000000000000000000007 ]]
[[ $ATG_SHELL_COLLISION_CONSTRUCTOR2_SIGNATURE == 'constructor(address)' ]]

[[ $ATG_SHELL_COLLISION_FOO_BAR_SIGNATURE == 'fooBar(uint256)' ]]
[[ $ATG_SHELL_COLLISION_FOO_BAR2_SIGNATURE == 'foo_bar(uint256)' ]]
first=$(atg_shell_collision_foo_bar_encode 7)
second=$(atg_shell_collision_foo_bar2_encode 7)
[[ ${first:0:10} == "$ATG_SHELL_COLLISION_FOO_BAR_SELECTOR" ]]
[[ ${second:0:10} == "$ATG_SHELL_COLLISION_FOO_BAR2_SELECTOR" ]]
[[ $first != "$second" ]]

note=$(atg_shell_collision_note_encode '--help')
[[ ${note:0:10} == "$ATG_SHELL_COLLISION_NOTE_SELECTOR" ]]
[[ $note == *2d2d68656c70* ]]

nested=$(atg_shell_collision_nested_encode '[[1,2],[3]]')
[[ $nested == "$(cast calldata 'nested(uint256[][])' '[[1,2],[3]]')" ]]
tuples=$(atg_shell_collision_tuple_array_encode '[(7,true)]')
[[ $tuples == "$(cast calldata 'tupleArray((uint256,bool)[])' '[(7,true)]')" ]]

[[ $ATG_SHELL_COLLISION_CHANGED_EVENT_TOPIC != "$ATG_SHELL_COLLISION_CHANGED_EVENT2_TOPIC" ]]
[[ $ATG_SHELL_COLLISION_DENIED_ERROR_SIGNATURE == 'Denied(uint256)' ]]
[[ $ATG_SHELL_COLLISION_DENIED_ERROR2_SIGNATURE == 'Denied(address)' ]]

echo 'shell collision and nested ABI bindings passed'
