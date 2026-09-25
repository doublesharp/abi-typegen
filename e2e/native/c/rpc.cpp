#include "atg_Token.hpp"
#include "rpc_adapter.h"
int main() {
    atg_Token::Client client(rpc_address(),nullptr,rpc_transport);
    atg_Token::atg_balanceOf_params query{}; query.atg_field.bytes[19]=43;
    auto before=client.atg_balanceOf(query);
    atg_Token::atg_mint_params args{}; args.atg_to=query.atg_field; args.atg_amount.bytes[31]=9;
    auto hash=client.atg_mint(args); assert(hash.size()==32);
    auto after=client.atg_balanceOf(query); assert(after->atg_field.bytes[31]==before->atg_field.bytes[31]+9);
    puts("OK C++ signed transaction, receipt and typed read");
}
