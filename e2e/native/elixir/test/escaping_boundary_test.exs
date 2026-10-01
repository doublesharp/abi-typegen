defmodule AbiTypegenElixir.EscapingBoundaryTest do
  use ExUnit.Case, async: true

  test "dollar function aliases retain the original selectors" do
    first = apply(DollarAliases, :"foo$_bar", [true])
    second = apply(DollarAliases, :"foo$_bar_2", [7])

    # Selectors independently computed with cast sig for the original ABI names.
    assert first.data == <<0x78, 0x4A, 0x4C, 0xE8, 1::256>>
    assert second.data == <<0xA8, 0x27, 0xD0, 0x4E, 7::256>>
    refute first.data == second.data
  end

  test "reserved function aliases retain the original selectors" do
    for {name, signature, value} <- [
          {:when, "when(bool)", true},
          {:when_2, "When(uint256)", 7}
        ] do
      assert apply(ReservedAliases, name, [value]).data == ABI.encode(signature, [value])
    end
  end
end
