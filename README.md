# micro-wibble runtime

A minimal garbage-collected bytecode interpreter meant for embedded systems.

Micro-wibble is not written for any particular language. It's a target VM for your language.

In micro-wibble, every value is a machine word: a 32-bit or 64-bit int, depending on your system. A value can be treated as an int for math, comparison, and bit operations, or as a reference to a heap-allocated object. Objects are arrays of up to 64 words. The runtime does no type-checking -- it leaves that up to the compiler.

The runtime is "sandboxed": References are bounds-checked to make sure they're within the heap; object field references are bounds-checked to make sure they're within the object; memory and CPU use can be constrained.

Memory use is prioritized over code speed, because many SoC have half the memory of an Apple ][, but over 100x the processing power.

... more info soon ...
