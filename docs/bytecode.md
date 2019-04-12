# micro-wibble bytecode format

## goals

- everything is a machine word (i32 or i64)
    - value (int)
    - reference to heap (pointer)
    - index into constant pool (need to be clearly different from a pointer) (maybe always set bit 0?)
- heap objects are just arrays of words ("slots")
    - or byte-accessed, through an extension (strings, byte arrays)
    - objects may have up to 64 slots
- no runtime type-checking, only sandbox-style bounds checking (bad code can crash, but can't corrupt the runtime)
- globals & locals (each are just numbered slots)
- constant pool is a set of byte-arrays, indexed from 0
    - can include "frozen objects" (array of slots)
    - a "class" (type) can itself be a frozen object (compiler would do this, not the runtime)
- code is loaded from the constant pool
    - fixed # of locals (specified in code object)
    - fixed max stack size for operations (also specified in code object)
- calling convention is:
    - compiler must enforce agreement between expected & actual # of args (use a temp object for varargs)
    - callee places N args on stack
    - "call (const pool #) N"
    - callee receives args as first N locals
    - "return N"
    - caller has N results pushed to its stack

## file format

- all ints are encoded as either varint (unsigned) or zigzag (signed)
- format:
    - u32: magic = F0 9F 97 BF
    - u8: version = 0
    - u8: global count
    - uint: index of "main" function in constant pool
    - u8[...]: constant pool
- code object:
    - u8: local count
    - u8: max stack size
    - u8[...]: bytecode
- each instruction is one byte, followed by optional (varint or zigzag) parameters
- to get short-circuit or/and, use nested if
- address of a constant is stored as `(c << 1) | 1`

## bytecodes

- stack vars are S1, S2... (left to right); immediates are N1, N2...
- 0 immediates (9)
    - load slot #S2 from S1 -> S1 `LD [*]`
    - store S3 into slot #S2 of S1 `ST [*]`
    - if: execute next only if S1 is true `IF`
    - * new obj: S1 slots, filling the first S2 from stack -> S1 `NEW`
    - call function S2 with S1 args `CALL`
    - * length (in slots) of S1 -> S1 `SIZE`
    - * return with S1 values `RET`
    - * do nothing `NOP`
    - * break into debugger `BREAK`
- 1 immediate (13)
    - * load immediate N1 -> S1 `LD #n`
    - * load address of const #N1 (as obj) -> S1 `LD %n` --- *maybe not necessary?*
    - * load local #N1 -> S1 `LD @n`
    - * load global #N1 -> S1 `LD $n`
    - * load slot #N1 from S1 -> S1 `LD [#n]`
    - * store S1 into local #N1 `ST @n`
    - * store S1 into global #N1 `ST $n`
    - * store S2 into slot #N1 of S1 `ST [#n]`
    - * unary op #N1 on S1
    - * binary op #N1 on S1, S2
    - ~new obj: N1 slots, filling the first S1 from stack -> S1~
    - call function S1 with N1 args `CALL #n`
    - * return with N1 values `RET #n`
    - jump to absolute byte #N1 `JUMP #n`
- 2 immediates (2)
    - * new obj: N1 slots, filling the first N2 from stack -> S1 `NEW #n, #n`
    - call native module #N1, function #N2 `SYS #n, #n`

## unary operations

- 0: not `NOT`
- 1: negative `NEG`
- 2: bit-not `INV`

## binary operations

- 0: + `ADD`
- 1: - `SUB`
- 2: * `MUL`
- 3: / `DIV`
- 4: % `MOD`
- 5: = `EQ`
- 6: < `LT`
- 7: <= `LE`
- 8: bit-or `OR`
- 9: bit-and `AND`
- a: bit-xor `XOR`
- b: shift-left `LSL`
- c: shift-right `LSR`
- d: sign shift-right `ASR`

## potential native modules

- byte arrays (length + data)
    - new (size, fill byte)
    - length
    - compare
    - ~concat~ (new, copy, copy)
    - index of
    - last index of
    - ~slice~ (new, copy)
    - b[x] get
    - b[x] set
    - copy b[i..j] into c[k]
    - fill b[i..j] with x
- strings
- infinite ints

## extended arrays/strings

- since an object can only have 64 slots
- use indirection: an extended array has a size, capacity, and refs to spans of 64 slots



// macro_rules! fail {
// -            ($code: expr) => {
// -                return Err(frame.to_error($code));
// -            };
// -        }
