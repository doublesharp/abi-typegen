defmodule AbiTypegenElixir.GeneratedTest do
  use ExUnit.Case, async: true
  import Bitwise

  @owner "0x0000000000000000000000000000000000000001"

  test "generated Token encodes the canonical balanceOf selector" do
    assert %Ethers.TxData{data: <<0x70, 0xA0, 0x82, 0x31, _::binary-size(32)>>} =
             Token.balance_of(@owner)
  end

  test "uint256 input preserves its full range and rejects overflow" do
    max_uint256 = (1 <<< 256) - 1

    assert %Ethers.TxData{
             data: <<0xA9, 0x05, 0x9C, 0xBB, _address::binary-size(32), word::binary-size(32)>>
           } = Token.transfer(@owner, max_uint256)

    assert :binary.decode_unsigned(word) == max_uint256
    assert_raise ArgumentError, fn -> Token.transfer(@owner, max_uint256 + 1) end
  end

  test "malformed address is rejected by the SDK-backed generated function" do
    assert_raise ArgumentError, fn -> Token.balance_of("0x1234") end
  end

  test "tuple overloads select distinct canonical signatures" do
    assert <<0x0C, 0xE7, 0xA8, 0xBD, _::binary>> = TupleCases.deposit({@owner}).data
    assert <<0xD1, 0xE9, 0x2C, 0x11, _::binary>> = TupleCases.deposit({1}).data
  end

  test "nested dynamic and fixed arrays match an independent ABI encoder" do
    assert cast_calldata!("setMatrix(uint256[][])", "[[1,2],[3]]") ==
             TupleCases.set_matrix([[1, 2], [3]]).data

    assert cast_calldata!("setRows(bool[2][])", "[[true,false],[false,true]]") ==
             TupleCases.set_rows([[true, false], [false, true]]).data
  end

  test "SDK-normalized function names preserve each original selector" do
    assert <<0x5E, 0x25, 0xF0, 0xC6, _::binary>> = NamingCases.foo_bar_2(7).data
    refute NamingCases.foo_bar(false).data == NamingCases.foo_bar_2(7).data
  end

  test "SDK underscore normalization keeps internal and leading underscores distinct" do
    assert <<0xA3, 0xB3, 0x44, 0x10, _::binary>> = UnderscoreCases.foo__bar(1).data
    assert <<0x00, 0xF9, 0x9E, 0x27, _::binary>> = UnderscoreCases.foo_bar(false).data
    assert <<0x41, 0x5D, 0x98, 0xA4, _::binary>> = UnderscoreCases._foo(2).data
    assert <<0x2F, 0xBE, 0xBD, 0x38, _::binary>> = UnderscoreCases.foo(3).data
  end

  test "generated event filters and custom errors retain their ABI fields" do
    assert %Ethers.EventFilter{} = Token.EventFilters.transfer(@owner, nil)
    assert %Ethers.EventFilter{} = Token.EventFilters.approval(nil, @owner)

    selector = Token.Errors.InsufficientBalance.function_selector()
    encoded = ABI.encode(selector, [<<1::160>>, 1, 2])

    assert {:ok, %Token.Errors.InsufficientBalance{} = error} =
             Token.Errors.find_and_decode(encoded)

    assert %{account: <<1::160>>, available: 1, required: 2} = Map.from_struct(error)
    assert {:error, :undefined_error} = Token.Errors.find_and_decode(<<0, 0, 0, 0>>)
  end

  defp cast_calldata!(signature, argument) do
    {output, 0} = System.cmd("cast", ["calldata", signature, argument], stderr_to_stdout: true)
    output |> String.trim() |> String.replace_prefix("0x", "") |> Base.decode16!(case: :mixed)
  end
end
