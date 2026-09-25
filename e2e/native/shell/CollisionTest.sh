#!/bin/bash
set -euo pipefail
# shellcheck disable=SC1090 # run.sh generates the source at a temporary path.
source "${ATG_SHELL_COLLISION:?collision binding is required}"

assert_equal() {
    if [[ $1 != "$2" ]]; then
        printf 'expected %s, got %s\n' "$2" "$1" >&2
        exit 1
    fi
}

assert_different() {
    if [[ $1 == "$2" ]]; then
        printf 'expected different values, got %s\n' "$1" >&2
        exit 1
    fi
}

assert_equal "$ATG_SHELL_COLLISION_CONSTRUCTOR_SIGNATURE" 'constructor(uint256)'
constructor=$(atg_shell_collision_constructor_args 7)
assert_equal "$constructor" 0x0000000000000000000000000000000000000000000000000000000000000007
assert_equal "$ATG_SHELL_COLLISION_CONSTRUCTOR2_SIGNATURE" 'constructor(address)'

assert_equal "$ATG_SHELL_COLLISION_FOO_BAR_SIGNATURE" 'fooBar(uint256)'
assert_equal "$ATG_SHELL_COLLISION_FOO_BAR2_SIGNATURE" 'foo_bar(uint256)'
first=$(atg_shell_collision_foo_bar_encode 7)
second=$(atg_shell_collision_foo_bar2_encode 7)
assert_equal "${first:0:10}" "$ATG_SHELL_COLLISION_FOO_BAR_SELECTOR"
assert_equal "${second:0:10}" "$ATG_SHELL_COLLISION_FOO_BAR2_SELECTOR"
assert_different "$first" "$second"

note=$(atg_shell_collision_note_encode '--help')
assert_equal "${note:0:10}" "$ATG_SHELL_COLLISION_NOTE_SELECTOR"
if [[ $note != *2d2d68656c70* ]]; then
    printf 'encoded note is missing --help bytes: %s\n' "$note" >&2
    exit 1
fi

nested=$(atg_shell_collision_nested_encode '[[1,2],[3]]')
assert_equal "$nested" "$(cast calldata 'nested(uint256[][])' '[[1,2],[3]]')"
tuples=$(atg_shell_collision_tuple_array_encode '[(7,true)]')
assert_equal "$tuples" "$(cast calldata 'tupleArray((uint256,bool)[])' '[(7,true)]')"

assert_different "$ATG_SHELL_COLLISION_CHANGED_EVENT_TOPIC" "$ATG_SHELL_COLLISION_CHANGED_EVENT2_TOPIC"
assert_equal "$ATG_SHELL_COLLISION_DENIED_ERROR_SIGNATURE" 'Denied(uint256)'
assert_equal "$ATG_SHELL_COLLISION_DENIED_ERROR2_SIGNATURE" 'Denied(address)'

echo 'shell collision and nested ABI bindings passed'
