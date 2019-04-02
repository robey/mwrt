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
    - u32: magic = xx xx xx xx
    - u8: version = 0
    - uint: index of "main" function in constant pool
    - u8[...]: constant pool
- code object:
    - u8: local count
    - u8: max stack size
    - u8[...]: bytecode
- each instruction is one byte, followed by optional (varint or zigzag) parameters
- to get short-circuit or/and, use nested if

## bytecodes

- stack vars are S1, S2... (left to right); immediates are N1, N2...
- 0 immediates (8)
    - load slot #S2 from S1 -> S1
    - store S3 into slot #S2 of S1
    - if: execute next only if S1 is true
    - new obj: S1 slots, filling the first S2 from stack -> S1
    - call function S2 with S1 args
    - return with S1 values
    - do nothing
    - break into debugger
- 1 immediate (16)
    - load immediate N1 -> S1
    - load const #N1 addr (as obj) -> S1    (is this necessary?)
    - load const #N1 (as sint value) -> S1
    - load local #N1 -> S1
    - load global #N1 -> S1
    - load slot #N1 from S1 -> S1
    - store S1 into local #N1
    - store S1 into global #N1
    - store S2 into slot #N1 of S1
    - length (in slots) of S1 -> S1
    - unary op #N1 on S1
    - binary op #N1 on S1, S2
    - new obj: N1 slots, filling the first S1 from stack -> S1  (?)
    - call function S1 with N1 args
    - return with N1 values
    - jump to absolute byte #N1
- 2 immediates (2)
    - new obj: N1 slots, filling the first N2 from stack -> S1
    - call native module #N1, function #N2

## unary operations

- 0: not
- 1: negative
- 2: bit-not

## binary operations

- 0: +
- 1: -
- 2: *
- 3: /
- 4: %
- 5: =
- 6: <
- 7: >=
- 8: bit-or
- 9: bit-and
- a: bit-xor
- b: shift-left
- c: shift-right
- d: sign shift-right

## potential native modules

- byte arrays (length + data)
    - new (size, fill byte)
    - length
    - compare
    - concat
    - index of
    - last index of
    - slice
    - b[x] get
    - mutable?
        - b[x] set (mutable get)
        - copy (mutable slice)
        - fill (mutable new)
- strings
- infinite ints
