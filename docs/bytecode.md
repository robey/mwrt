bytecode notes

what's an MVP?
  - some global state (bools & ints)
  - events: on gpio change, on attr change (from user)
  - code: basic math (+ - * / %), if-then-else, set/get global state, local state
  - available functions:
      - get gpio N
      - set gpio N to X
      - get attr N
      - set attr N to X
  - all i32

a word can be:
  - i32
  - object or byte-string (pointer to heap)
  - function (offset into string pool)

instructions: (15)
  - load const (8, 16, 32?)
  - load from constant pool N
  - load local N
  - load global N
  - store local N
  - store global N
  - unary operation (not negative bitnot)
  - binary operation (+ - * / % == < > bitor bitand bitxor lshift rshift arth-rshift)
      - to get short-circuit or/and, use nested if
  - if: run next instruction only if A is non-zero
  - jump N
  - call N (count)
  - return N (count)
  - get slot N
  - put slot N
  - allocate object with N slots, fill X from stack


N as varint/zint

string pool.


## constant pool

- any byte array
- a function is a byte array
- need to be able to store serialized "objects" too (a "class" would be a serialized list of functions, which are cpool offsets)

## file format

- magic (u32)
- version
- "main" function index (zint)
- string pool


# bytecode strawman

- each instruction is one byte, followed by optional parameters
- each parameter is in "zint" format:
    - if the high bit is set, another byte will follow
    - each byte has 7 new bits
    - first byte is the lowest 7 bits
- parameter 1 is X, parameter 2 is Y
- if the argument stack is used, they are A B C... in order, always "consumed"

- objects may have up to 64 slots

- code block is:
    - max stack size
    - # of locals
    - code bytes
- calling convention is:
    - place N args on stack
    - "call (const pool #) N"
    - caller receives stack with N args + N (count)
    - "return N"
    - callee has N results pushed + N (count)

## basic set

- e0: load const8 (sign extended) -> A
- e1: load const16 (sign extended) -> A
- e2: load const32 -> A
- e3: load pool #X -> A
- e4: load local #X -> A
- e5: load global #X -> A
- e6: load slot #X from A -> A
- e7: load slot #B from A -> A
- e8: store A into local #X
- e9: store A into global #X
- ea: store B into slot #X of A
- eb: store C into slot #B of A
- ec: unary operation #X on A
- ed: binary operation #X on A, B
- ee: call function A with X args preceding it on stack
- ef: return with X values from stack
- f0: if: execute next only if A is true
- f1: jump to absolute byte #X
- f2: allocate an object with X slots, filling the first Y from the stack -> A
- f3: allocate an object with A slots, filling the first B from the preceding stack -> A
- f4: call native module #X, function #Y

- fe: do nothing
- ff: break into debugger

## basic set, take 2

- break up into how many follow-on immediate args there are
- all immediates are zigzag, some can probably be limited to 0-63 (positive, one byte) for now
- need < 32 bytecodes for this

- 0 immediates (8)
    - load slot #B from A -> A
    - store C into slot #B of A
    - if: execute next only i A is true
    - new obj: A slots, filling the first B from stack -> A
    - call function B with A args
    - return with A values
    - do nothing
    - break into debugger
- 1 immediate (16)
    - load immediate X -> A
    - load const #X addr (as obj) -> A
    - load const #X (as 32-bit value) -> A
    - load local #X -> A
    - load global #X -> A
    - load slot #X from A -> A
    - store A into local #X
    - store A into global #X
    - store B into slot #X of A
    - length (in slots) of A -> A
    - unary op #X on A
    - binary op #X on A, B
    - new obj: X slots, filling the first A from stack -> A  (?)
    - call function A with X args
    - return with X values
    - jump to absolute byte #X
- 2 immediates (2)
    - new obj: X slots, filling the first Y from stack -> A
    - call native module #X, function #Y

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

## short codes

- 00 - 0f: load local 0 - 15
- 10 - 1f: load global 0 - 15
- 20 - 2f: store local 0 - 15
- 30 - 3f: load slot 0 - 15
- 40 - 4f: store slot 0 - 15
- 50 - 57: unary operations 0 - 7
- 58 - 5f: store local 0 - 7 but also leave it on the stack
- 60 - 6f: binary operations 0 - 15
- 70 - 77: call function with 0 - 7 args
- 78 - 7f: return with 0 - 7 values
- 80 - a3: allocate object: 1/1, 2/1, 2/2, 3/1 ... 8/8 (36 combos)

 (load-local 2) (load-local 2) (get-slot 0) (get-slot 4) (call 1) -- load x, load x, get x's type, get the type's 4th field, call it with 1 param (x)

02 02 30 34 71

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
