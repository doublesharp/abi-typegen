#include "atg_Token.hpp"
#include <algorithm>
#include <cassert>
#include <iostream>
#include <type_traits>

using TransferResult = atg_Token::Decoded<atg_Token::atg_transfer_returns>;
static_assert(std::is_move_constructible_v<TransferResult>);
static_assert(std::is_move_assignable_v<TransferResult>);
static_assert(!std::is_copy_constructible_v<TransferResult>);
static_assert(!std::is_copy_assignable_v<TransferResult>);

int main() {
    atg_Token::atg_transfer_params args{};
    args.atg_amount.bytes[0] = 0x80;
    auto bytes = atg_Token::atg_transfer_encode(args);
    assert(bytes.size() == 68 && bytes[36] == 0x80);
    std::vector<uint8_t> result(32,0); result[31]=1;
    auto decoded = atg_Token::atg_transfer_decode(result);
    assert(decoded->atg_ok);
    auto moved = std::move(decoded); assert(moved->atg_ok);
    assert(moved.value().atg_ok);
    auto reassigned = atg_Token::atg_transfer_decode(result);
    reassigned = std::move(moved);
    assert(reassigned->atg_ok);
    bool rejected=false;
    try { result[31]=2; (void)atg_Token::atg_transfer_decode(result); } catch (const std::runtime_error&) { rejected=true; }
    assert(rejected);
    rejected=false;
    try { atg_Token::Client client({}, nullptr, nullptr); } catch (const std::invalid_argument&) { rejected=true; }
    assert(rejected);
    rejected=false;
    try { (void)atg_Token::checked(nullptr); } catch (const std::runtime_error&) { rejected=true; }
    assert(rejected);
    auto failure = atg_result_failure("transport failed");
    rejected=false;
    try { (void)atg_Token::checked(failure); } catch (const std::runtime_error& error) { rejected=std::string(error.what())=="transport failed"; }
    assert(rejected);
    auto empty = atg_Token::checked(atg_result_bytes(nullptr, 0));
    assert(atg_Token::copy_bytes(empty).empty());

    // Returned arrays borrow runtime storage and must remain readable after an owner move.
    std::vector<uint8_t> nested(224, 0);
    nested[31]=32; nested[63]=2; nested[95]=64; nested[127]=128;
    nested[159]=1; nested[160]=0x80;
    auto grid = atg_Token::atg_echo_decode(nested);
    std::fill(nested.begin(), nested.end(), 0xff);
    nested.clear();
    auto retained = std::move(grid);
    assert(retained->atg_values.len==2);
    assert(retained->atg_values.data[0].len==1);
    assert(retained->atg_values.data[0].data[0].bytes[0]==0x80);
    assert(retained->atg_values.data[1].len==0);
    std::cout << "C++ bindings: RAII ownership, large integers, decode errors passed\n";
}
