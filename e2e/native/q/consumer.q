/ Run in generated output directory after loading Events.q.
.atgEvents.loadBridge[`:./abi_typegen_q];
assert:{if[not x;'"assertion failed"]};
word:32#0xff;
address:20#0x11;
padded:(12#0x00),address;
yes:(31#0x00),0x01;
no:32#0x00;
topics:32#0xaa;
data:word,padded,yes,no,word,padded;
v:.atgEvents.e0selectDecode[topics;data];
assert[word~v 0];
assert[address~v 1];
assert[10b~v 2];
assert[(word;address)~v 3];
assert[topics~v 4];
metadata:.atgEvents.metadataColumns!(word;address;word;topics;topics;word;0b);
row:.atgEvents.e0selectRow[metadata;topics;data];
assert[word~row`f0chainId];
assert[0b~row`removed];
table:.atgEvents.e0selectTable upsert row;
assert[1=count table];
assert[()~.atgEvents.e1selectDecode[0x;0x]];
/ Runtime-owned diagnostic text remains intact after the result is released.
do[100;assert["unknown event signature"~@[.atgEvents.nativeDecode;("[]";"NoSuchEvent()";0x;0x);{x}]]];
/ Metadata names are strings even when the ABI name has one character.
assert[10h=type .atgEvents.e2SingleNames[0]];
assert[(enlist "a")~.atgEvents.e2SingleNames[0]];
/ A short ABI body must fail through the native runtime.
assert[`failed~@[.atgEvents.e0selectDecode[topics;];0x;{`failed}]];
/ C boundary rejects character vectors passed as topics.
assert[`failed~@[.atgEvents.e0selectDecode[;data];"bad";{`failed}]];
-1 "q event decoder checks passed";
exit 0;
