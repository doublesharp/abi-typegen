#include "atg_Token.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

static atg_result *transport(void *context, const atg_address *address, const uint8_t *data, size_t len, int write, const atg_call_options *options) {
    (void)context; (void)address; (void)options;
    assert(len >= 4);
    if (!memcmp(data, atg_Token_atg_echo_selector,4)) return atg_result_bytes(data+4,len-4);
    uint8_t response[32] = {0};
    response[31] = write ? 7 : 42;
    return atg_result_bytes(response,32);
}
int main(void) {
    atg_Token_atg_transfer_params args = {0};
    args.atg_to.bytes[19] = 1; args.atg_amount.bytes[31] = 42;
    atg_result *r = atg_Token_atg_transfer_encode(&args);
    assert(!atg_result_error(r)); assert(atg_result_len(r) == 68);
    const uint8_t selector[4] = {0xa9,0x05,0x9c,0xbb};
    assert(!memcmp(atg_result_data(r),selector,4)); assert(atg_result_data(r)[67] == 42);
    atg_result_free(r);
    atg_Token_client client = {{ {0} },NULL,transport};
    atg_Token_atg_balanceOf_params read_args = {0};
    atg_Token_atg_balanceOf_returns output = {0};
    r = atg_Token_atg_balanceOf(&client,&read_args,NULL,&output);
    assert(!atg_result_error(r)); assert(output.atg_amount.bytes[31] == 42); atg_result_free(r);
    r = atg_Token_atg_transfer(&client,&args,NULL);
    assert(!atg_result_error(r)); assert(atg_result_data(r)[31] == 7); atg_result_free(r);
    uint8_t bad[32] = {0}; bad[31] = 2;
    atg_Token_atg_transfer_returns result = {0};
    r = atg_Token_atg_transfer_decode(bad,sizeof bad,&result); assert(atg_result_error(r)); atg_result_free(r);
    atg_word value = {{0}}; value.bytes[31] = 1;
    atg_call_options options = {0}; options.value = &value;
    r = atg_Token_atg_transfer(&client,&args,&options);
    assert(atg_result_error(r)); atg_result_free(r);
    atg_Token_atg_echo_params echo = {0};
    echo.atg_values.len = 2;
    echo.atg_values.data = calloc(2,sizeof(*echo.atg_values.data));
    assert(echo.atg_values.data);
    echo.atg_values.data[0].len = 1;
    echo.atg_values.data[0].data = calloc(1,sizeof(*echo.atg_values.data[0].data));
    assert(echo.atg_values.data[0].data);
    echo.atg_values.data[0].data[0].bytes[0] = 0x80;
    atg_Token_atg_echo_returns echoed = {0};
    r = atg_Token_atg_echo(&client,&echo,NULL,&echoed);
    assert(!atg_result_error(r)); assert(echoed.atg_values.len == 2);
    assert(echoed.atg_values.data[0].data[0].bytes[0] == 0x80);
    assert(echoed.atg_values.data[1].len == 0);
    free(echo.atg_values.data[0].data); free(echo.atg_values.data); atg_result_free(r);
    uint8_t topics[3][32] = {{0}}; memcpy(topics[0],atg_Token_atg_Transfer_event_topic,32);
    topics[1][31] = 1; topics[2][31] = 2;
    uint8_t event_data[32] = {0}; event_data[31] = 7;
    atg_Token_atg_Transfer_event_fields event = {0};
    r = atg_Token_atg_Transfer_event_decode(&topics[0][0],3,event_data,32,&event);
    assert(!atg_result_error(r)); assert(event.atg_from.bytes[19] == 1); assert(event.atg_amount.bytes[31] == 7); atg_result_free(r);
    atg_log_filter filter;
    assert(atg_Token_atg_Transfer_event_filter(client.address,NULL,NULL,&filter));
    assert(filter.topic_count == 3 && filter.has_topic[0] && !filter.has_topic[1]);
    uint8_t revert[36] = {0}; memcpy(revert,atg_Token_atg_Denied_error_selector,4); revert[35] = 3;
    atg_Token_atg_Denied_error_fields error = {0};
    r = atg_Token_atg_Denied_error_decode(revert,sizeof revert,&error);
    assert(!atg_result_error(r)); assert(error.atg_account.bytes[19] == 3); atg_result_free(r);
    puts("C bindings: typed reads/writes, calldata, invalid result rejection passed");
}
