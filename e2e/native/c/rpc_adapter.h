#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void unhex(const char *hex, uint8_t *out, size_t len) {
    for (size_t i=0;i<len;i++) { unsigned value; assert(sscanf(hex+2*i,"%2x",&value)==1); out[i]=(uint8_t)value; }
}
static atg_result *rpc_transport(void *context,const atg_address *address,const uint8_t *data,size_t len,int write,const atg_call_options *options) {
    (void)context; (void)options;
    printf("%c 0x",write ? 'W' : 'R');
    for (size_t i=0;i<20;i++) printf("%02x",address->bytes[i]);
    printf(" 0x"); for (size_t i=0;i<len;i++) printf("%02x",data[i]); printf("\n"); fflush(stdout);
    char response[16384]; assert(fgets(response,sizeof response,stdin));
    size_t count=strcspn(response,"\r\n"); assert(count>=2 && response[0]=='0' && response[1]=='x' && count%2==0);
    uint8_t decoded[8192]; size_t n=(count-2)/2; unhex(response+2,decoded,n);
    return atg_result_bytes(decoded,n);
}
static atg_address rpc_address(void) {
    const char *text=getenv("ATG_TOKEN_ADDRESS"); assert(text && strlen(text)==42);
    atg_address address; unhex(text+2,address.bytes,20); return address;
}
