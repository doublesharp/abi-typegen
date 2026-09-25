#include "atg_Token.h"
#include "rpc_adapter.h"
int main(void) {
    atg_Token_client client = {rpc_address(),NULL,rpc_transport};
    atg_Token_atg_balanceOf_params query = {0}; query.atg_field.bytes[19]=42;
    atg_Token_atg_balanceOf_returns balance = {0};
    atg_result *r=atg_Token_atg_balanceOf(&client,&query,NULL,&balance);
    assert(!atg_result_error(r)); uint8_t previous=balance.atg_field.bytes[31]; atg_result_free(r);
    atg_Token_atg_mint_params args={0}; args.atg_to=query.atg_field; args.atg_amount.bytes[31]=7;
    r=atg_Token_atg_mint(&client,&args,NULL); assert(!atg_result_error(r)); assert(atg_result_len(r)==32); atg_result_free(r);
    r=atg_Token_atg_balanceOf(&client,&query,NULL,&balance);
    assert(!atg_result_error(r)); assert(balance.atg_field.bytes[31]==previous+7); atg_result_free(r);
    puts("OK C signed transaction, receipt and typed read");
}
