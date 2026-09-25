#include "atg_Token.hpp"
#include <cassert>
#include <iostream>
int main() {
    atg_Token::atg_transfer_params args{};
    args.atg_amount.bytes[0] = 0x80;
    auto bytes = atg_Token::atg_transfer_encode(args);
    assert(bytes.size() == 68 && bytes[36] == 0x80);
    std::vector<uint8_t> result(32,0); result[31]=1;
    auto decoded = atg_Token::atg_transfer_decode(result);
    assert(decoded->atg_ok);
    auto moved = std::move(decoded); assert(moved->atg_ok);
    bool rejected=false;
    try { result[31]=2; (void)atg_Token::atg_transfer_decode(result); } catch (const std::runtime_error&) { rejected=true; }
    assert(rejected);
    std::cout << "C++ bindings: RAII ownership, large integers, decode errors passed\n";
}
