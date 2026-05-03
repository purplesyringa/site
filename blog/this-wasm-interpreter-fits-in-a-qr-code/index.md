---
title: This Wasm interpreter fits in a QR code
time: May 3, 2026
intro: |
    2944 bytes. Less than 0.006% of [Wasmtime](https://wasmtime.dev/), smaller than a C "Hello, world!".

    Zero dependencies, no cheating: just a static x86-64 [Linux executable](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format). Scan the QR code above with `zbarimg --raw -Sbinary` or another QR decoder that supports binary data, or [directly download the program](https://github.com/purplesyringa/autumn/raw/refs/heads/master/autumn) from [the GitHub repo](https://github.com/purplesyringa/autumn), and you're good to go.
---

![A QR code containing the interpreter](autumn.png)

2944 bytes. Less than 0.006% of [Wasmtime](https://wasmtime.dev/), smaller than a C "Hello, world!".

Zero dependencies, no cheating: just a static x86-64 [Linux executable](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format). Scan the QR code above with `zbarimg --raw -Sbinary` or another QR decoder that supports binary data, or [directly download the program](https://github.com/purplesyringa/autumn/raw/refs/heads/master/autumn) from [the GitHub repo](https://github.com/purplesyringa/autumn), and you're good to go.

Here's me playing [the Rust guessing game](https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html), [cross-compiled for Wasm](https://github.com/purplesyringa/autumn/raw/master/tests/guessing-game/guessing-game.wasm):

![Terminal recording: I run ./autumn tests/guessing-game/guessing-game.wasm and then try to guess a number from 1 to 100 based on whether my guess is below or above the actual number.](guessing-game.gif)

And here's me running [QuickJS](https://github.com/quickjs-ng/quickjs), [straight from official releases](https://github.com/quickjs-ng/quickjs/releases/tag/v0.14.0):

![Terminal recording: I run ./autumn qjs-wasi.wasm and type simple JavaScript expressions into the REPL, like 5+3 and "hello, " + "world". The JS interpreter is slow at syntax-highlighting the input, but works.](qjs.gif)

The interpreter implements the entire [Lime1](https://github.com/WebAssembly/tool-conventions/blob/main/Lime.md#lime1) Wasm feature set, and a chunk of [WASI](https://wasi.dev/) sufficient for these programs. It's not quite feature-complete, it's slow, and it's insecure -- but it's still a marvelous feat of engineering with reasonable capabilities.

I had insane fun working on it, and now I want to share what made this possible.


### Acknowledgements

Thanks to:

- [Yuki](https://github.com/sylfn) for writing a tool for the project and rubber-duck debugging.
- [Genbox](https://github.com/Genbox) for contributing the idea of using a QR code.


### Sizecoding 101

QR codes can contain up to 2953 bytes. If the project was any simpler, I'd [just use assembly](https://nathanotterness.com/2021/10/tiny_elf_modernized.html) -- but that has a disastrous effect on velocity, which is crucial for an open-ended project that requires constant experimentation and sweeping changes.

Herding `gcc` with `-Os -fno-align-functions -fno-align-jumps -fno-align-labels -fno-align-loops -fno-jump-tables -fno-asynchronous-unwind-tables -fomit-frame-pointer` helps, but not when the simplest C program is already too large. The issue is not the language, but metadata: symbols, ELF sections, dynamic linking. The headers are tiny, but they add up, and at this scale each one has to be abolished.

A linker script can merge almost all sections -- code, globals, constants -- into one, dropping unnecessary alignment from the binary. This makes code writeable and data executable, but security was never a goal in this experiment.

```c
ENTRY(_start)
OUTPUT_FORMAT(elf64-x86-64)

SECTIONS {
    . = 0x400000;
    .text : {
        *(.text .text.* .data .data.* .rodata .rodata.*)
    }
    .bss : {
        *(.bss .bss.*) *(COMMON)
    }
    /DISCARD/ : {
        *(.note.*)
    }
}
```

`strip --strip-section-headers` removes symbol information and section info, retaining only segments -- a crude approximation sufficient for the kernel to run the program.

Linking `libc` is expensive. `libc` does two things: initializes the program and invokes syscalls. In most operating systems, only `libc` knows how to do this, but Linux offers a stable userland ABI. You only need 23 bytes of machine code to wrap `main`:

```c
asm (
    ".globl _start;"
    "_start:"
    "pop %rdi;" // argc
    "mov %rsp, %rsi;" // argv
    "lea 8(%rsi,%rdi,8), %rdx;" // envp
    "call main;"
    "mov $60, %eax;" // exit
    "xor %edi, %edi;"
    "syscall;"
);
```

This is [inline assembly](https://en.cppreference.com/c/language/asm), and you'll need to get used to it, because in sizecoding it's everywhere. Invoking syscalls, like `read`, is as simple as:

```c
long rax = SYS_read;
asm volatile ("syscall" : "+a"(rax) : "D"(fd), "S"(buf), "d"(count) : "rcx", "r11", "memory");
return rax;
```

x86 also has a party trick up its sleeve: legacy [CISC](https://en.wikipedia.org/wiki/Complex_instruction_set_computer) instructions. `memcpy`, `memset`, `strlen`, and all that jazz can each be implemented with a single 2-byte [`rep` instruction](https://www.felixcloutier.com/x86/rep:repe:repz:repne:repnz).

These features make x86/Linux the best combination for this project -- x86-64, more specifically, since Wasm needs 64-bit operations. This is unlike [demos](https://en.wikipedia.org/wiki/Demoscene), which usually target Windows or older architectures because they use a different set of features.

This forms a 200-byte clean slate. Now what?


### Parsing

To execute a Linux ELF program, you only need to parse headers, map segments to memory, and jump to the entry point. Dynamic linking is only slightly harder. Wasm is not like that -- it has a multitude of different sections describing various moving parts:

- Type section -- the dictionary of function signatures.
- Import section -- IDs associated with syscalls.
- Function section -- function signatures as indices into the type section.
- Memory section -- amount of memory to allocate.
- Global section -- initial global values.
- Export section -- the index of the `_start` function.
- Start section -- the initializing function.
- Element section -- jump tables.
- Code section -- function bodies.
- Data section -- initial memory contents.

Parsing this takes 800 bytes, or 25% of the quota down the drain before executing code. And that's already after some optimization! So here's the first one of many unorthodox tricks I used.


### Imports

Wasm programs import syscalls as external functions, like `environ_get` or `fd_read`. Simple interpreters store the names as strings, but I store hashes instead -- collisions are not an issue because the WASIp1 interface is closed.

x86 has [instructions for CRC32](https://www.felixcloutier.com/x86/crc32), and for syscall names, hashing the first 8 bytes of the name and its length gives a collision-free hash at the cost of a single instruction.

Hashes can then be resolved to implementations with flat maps. But of which layout?

```c
// Interleaved keys and values?
struct key_value {
    unsigned short key;
    unsigned short value;
};
struct key_value table[20];

// Or separate keys and values?
unsigned short keys[20];
unsigned short values[20];
```

C++ uses the former, but the latter is both faster (if using vectorization) and shorter: `repne scasw` (the x86 generalization of `memchr`) can locate the key in just two bytes.


### Control flow

What's the first instruction in an average Wasm program? `i32.const`? `call`? Nope, it's `block`, and it's worse than you might think.

CPUs implement control flow as conditional jumps to instruction addresses. Wasm begs to differ. Wasm functions are trees with high-level constructs, like `if` and `loop`. Instead of jumping, you can `continue` a `loop` (jumping to its beginning) or `break` from a `block` (jumping to its end). With enough nesting, [any control flow can be simulated](http://troubles.md/why-do-we-need-the-relooper-algorithm-again/).

Jumps look like `br <level>`, where `<level>` specifies how many blocks to exit. Typical interpreters analyze the code before execution to know which instruction each jump targets. Since Wasm instructions are variable-length, this requires two functions per opcode: "parse" and "execute" -- a luxury we can't afford. But there is a workaround:

```c
const char *p; // global code pointer, adjusted by `read_uint` and other parsers

void local_get() {
    unsigned long idx = read_uint();
    // If `parsing_mode` is `true`, we parse the instruction and quit. Otherwise, we execute it
    // after parsing.
    if (parsing_mode) {
        return;
    }
    push(locals[idx]);
}
```

Single-pass execution saves even more code. Instead of a `bool`, I maintain an integer `break_level`. If it's `0`, code is executed normally. Otherwise, instructions are only parsed. It's set by `br` and decremented on each block exit. This mechanism powers `block`, `loop`, `if`, `else`, and `return`, implementing control flow with little size overhead.

Is it slow? Sure, since it constantly reparses code it doesn't execute -- look at how laggy QuickJS is. But the beauty of it is that for once I don't care about performance.


### Registers

You're already familiar with the globals `p` (code pointer) and `break_level` (parsing/execution mode), which have to be accessed by every single instruction handler:

```c
unsigned char *p;
unsigned break_level;
```

Accessing globals stored in memory takes space, but x86-64 has registers to spare:

```c
register unsigned char *p asm ("r12");
register unsigned break_level asm ("r14");
```

Yes, this syntax works for globals, at least on GCC -- you don't need to write all handlers in assembly just to pass data through registers. This saves about 500 bytes.


### Instructions

With Wasm peculiarities out of the way, we can actually execute instructions:

```c
while (*p != 0x0b /* end */) {
    opcode_map[*p++]();
}
```

Let's take a look at memory loads for example. There are loads of them:

- 0x28 `i32.load`
- 0x29 `i64.load`
- 0x2A `f32.load`
- 0x2B `f64.load`
- 0x2C `i32.load8_s`
- 0x2D `i32.load8_u`
- 0x2E `i32.load16_s`
- 0x2F `i32.load16_u`
- 0x30 `i64.load8_s`
- 0x31 `i64.load8_u`
- 0x32 `i64.load16_s`
- 0x33 `i64.load16_u`
- 0x34 `i64.load32_s`
- 0x35 `i64.load32_u`

Individually, each handler needs little code, but it's better to write a single complex function than 14 small ones, since they add up. The multitude of opcodes can be implemented by a 64-bit read followed by variable shifts, with amount depending on bitness and arithmetic/logical shifts depending on sign- or zero-extension:

```c
void load(unsigned char opcode) {
    // ...
    int d = 64 - bitness;
    value <<= d; // shift out unused bits
    if (opcode % 2 == 0) { // signed or pure 32-bit
        value = (long)value >> d; // return the bits to the correct position with sign-extension
        if (opcode < 0x30) { // 32-bit destination
            value &= -1U; // clear top 32 bits
        }
    } else { // unsigned or pure 64-bit
        value >>= d; // return the bits to the correct position with zero-extension
    }
    // ...
}
```

Sign-extension is simple, but the shift amount is hard to calculate from `opcode` without using a LUT. The same situation arises in other operations, and so the cheapest way to do this is to integrate a general-purpose LUT into the opcode map:

```c
void load(unsigned char opcode, unsigned char d) {
    // ...
}

while (*p != 0x0b /* end */) {
    unsigned char opcode = *p++;
    opcode_map[opcode].handler(opcode, opcode_map[opcode].arg);
}
```

You'll see this utilized more soon.


### Funclets

Let's focus on unary operations for now. Their handlers have the same structure:

```c
if (break_level > 0) {
    return;
}
unsigned long value = *stack_head;
// ...
*stack_head = value;
```

Reusing the boilerplate with `switch (opcode)` is suboptimal. A hand-written jump table with the branch offset in `arg` is smaller than any pure-C alternative can be:

```c
asm (
    "jmp *%[handler];"
    "unop_handlers:"
    "unop_clz32: lzcnt %k0, %k0; jmp 1f;"
    "unop_clz64: lzcnt %0, %0; jmp 1f;"
    "unop_ctz32: tzcnt %k0, %k0; jmp 1f;"
    "unop_ctz64: tzcnt %0, %0; jmp 1f;"
    // ...
    "1:"
    : "+R"(*stack_head)
    : [handler]"r"(&unop_handlers + arg)
    : "flags"
);
```

This is an optimization a compiler can't do because it's unaware of the external LUT. Here's one it could, but didn't. `jmp` takes two bytes; `ret` takes only one:

```c
asm (
    "call *%[handler];"
    ".pushsection .text.op;"
    "unop_handlers:"
    "unop_clz32: lzcnt %k0, %k0; ret;"
    "unop_clz64: lzcnt %0, %0; ret;"
    "unop_ctz32: tzcnt %k0, %k0; ret;"
    "unop_ctz64: tzcnt %0, %0; ret;"
    // ...
    ".popsection;"
    : "+R"(*stack_head)
    : [handler]"r"(&unop_handlers + arg)
    : "flags"
);
```

This tiny change saves about 30 bytes -- mere crumbs in all other circumstances, but since the final result is just 9 bytes below the limit, even this minor thing matters.


### Prefixes

Assembly paves the way for trickier optimizations. Many unary ops have 32-bit and 64-bit variants, which x86 denotes by the presence of a REX prefix (here `48`):

```x86asm
   0f be c0             movsx  eax,al ; sign-extend 8 bits to 32 bits
48 0f be c0             movsx  rax,al ; sign-extend 8 bits to 64 bits
   0f bf c0             movsx  eax,ax ; sign-extend 16 bits to 32 bits
48 0f bf c0             movsx  rax,ax ; sign-extend 16 bits to 64 bits
```

[*Real* programmers](https://xkcd.com/378/) overlap their branches -- take that, ARM!

```c
    "extend8_64: movsx %b0, %0; ret;"
    "extend8_32 = extend8_64 + 1;"
```

`sqrt` and `float` $\leftrightarrow$ `double` conversion use a different encoding, breaking this trick:

```x86asm
f3 0f 51 c0             sqrtss xmm0,xmm0 ; float sqrt
f2 0f 51 c0             sqrtsd xmm0,xmm0 ; double sqrt
f3 0f 5a c0             cvtss2sd xmm0,xmm0 ; float -> double
f2 0f 5a c0             cvtsd2ss xmm0,xmm0 ; double -> float
```

But the corresponding vectorized operations save the day. x86 never fails to surprise.

```x86asm
   0f 51 c0             sqrtps xmm0,xmm0
66 0f 51 c0             sqrtpd xmm0,xmm0
   0f 5a c0             cvtps2pd xmm0,xmm0
66 0f 5a c0             cvtpd2ps xmm0,xmm0
```


### Code patching

Not all instructions are as simple. Wasm comparisons map nicely to the x86 [`setcc` instructions](https://www.felixcloutier.com/x86/setcc) -- `sete`, `setl`, `setae` -- which encode the operation in the middle byte:

```x86asm
0f 94 c0                sete   al ; a = b
0f 9c c0                setl   al ; a < b, signed
0f 93 c0                setae  al ; a >= b, unsigned
```

Funclets would take four bytes per opcode, but patching a function's x86 code in runtime -- a rudimentary form of JIT -- brings overhead down to a constant:

```c
asm (
    "cmp $0x51, %[opcode];"
    "jb 1f + 1;" // if 32-bit, skip over the REX prefix in `cmp`
    "1: cmp %[b], %[a];" // perform 64-bit comparison
    "mov %[arg], 2f + 1(%%rip);" // patch the setb instruction, taking the byte from `arg`
    "2: setb %[out]" // this is actually arbitrary setcc
    : [out]"=R"(out)
    : [a]"R"(a), [b]"R"(b), [arg]"r"(arg), [opcode]"r"(opcode)
    : "flags"
);
```

[Self-modifying code](https://en.wikipedia.org/wiki/Self-modifying_code) has troublesome effects on performance, but we've burned that bridge a long time ago -- it can't be that bad in comparison to constant reparsing.

Floating-point rounding also benefits from patching, even though the rounding direction can be set in a global flag -- patching code turns out to be cheaper.


### PF

Next up: float-to-int conversions. These supports four destination types: `i32`, `u32`, `i64`, and `u64`. They look similar, but their implementations are completely different:

```rust
fn f64_to_i32(x: f64) -> i32 { x as _ }
fn f64_to_u32(x: f64) -> u32 { x as _ }
fn f64_to_i64(x: f64) -> i64 { x as _ }
fn f64_to_u64(x: f64) -> u64 { x as _ }
```

```x86asm expansible
f64_to_i32:
    xor       eax, eax
    ucomisd   xmm0, xmm0
    maxsd     xmm0, 0xc1e0000000000000
    minsd     xmm0, 0x41dfffffffc00000
    cvttsd2si ecx, xmm0
    cmovnp    eax, ecx
    ret

f64_to_u32:
    xorpd     xmm1, xmm1
    maxsd     xmm1, xmm0
    movsd     xmm0, 0x41efffffffe00000
    minsd     xmm0, xmm1
    cvttsd2si rax, xmm0
    ret

f64_to_i64:
    cvttsd2si rax, xmm0
    ucomisd   xmm0, 0x43dfffffffffffff
    mov       rcx, 0x7fffffffffffffff
    cmovbe    rcx, rax
    xor       eax, eax
    ucomisd   xmm0, xmm0
    cmovnp    rax, rcx
    ret

f64_to_u64:
    cvttsd2si rax, xmm0
    mov       rcx, rax
    sar       rcx, 63
    movapd    xmm1, xmm0
    subsd     xmm1, 0x43e0000000000000
    cvttsd2si rdx, xmm1
    and       rdx, rcx
    or        rdx, rax
    xor       ecx, ecx
    xorpd     xmm1, xmm1
    ucomisd   xmm0, xmm1
    cmovae    rcx, rdx
    ucomisd   xmm0, 0x43efffffffffffff
    mov       rax, -1
    cmovbe    rax, rcx
    ret
```

Parts of Rust's generated code are suboptimal, overall the distinctions are justified:

- x86 returns `0x80...00` on out-of-bounds conversions, Wasm enforces saturation. For signed conversions, this constant is by pure luck the precise lower bound -- but the upper bound still needs to be handled explicitly.

- `f64_to_i32` clamps the input `double` with `min(x, 2^32 - 1)`. `f64_to_i64` can't do that because `2^64 - 1` doesn't fit in `f64` precisely -- and rounding would cause `cast(+inf)` to return `2^64 - 2048` instead of `2^64 - 1`.

- For unsigned conversions, `max(x, 0)` sets the lower bound. This also converts `NaN` to `0`, as mandated by Wasm, since x86 defines `min(NaN, a) = a`. Signed conversions still require explicit `NaN` handling.

- x86 only supports signed conversions. `u32` can be emulated with `i64` because it always fits; large `u64` values need to be handled manually.

Handling this complexity in a single generic function is an exercise in ingenuity.

First, I generate the `double` representation of the upper bound on the fly:

```c
unsigned bitness = 31 + (arg & 1) + 32 * ((arg & 4) != 0);
unsigned long upper_bound = (1023UL + bitness) << 52; // pow2(bitness), actually `double`
```

`min` fails for `f64_to_i64`, so it's not worth doing -- better compare `x < upper_bound` and manually handle the too-high case. x86 reports whether `x` is `NaN` as a side effect of comparison. GCC codegen can't utilize it, so inline assembly has to be used:

```c
asm goto (
    "ucomisd %1, %0;"
    "jp %l2;"
    "jb %l3;"
    :
    : "x"(x), "x"(upper_bound)
    : "flags"
    : maxsd, below_limit
);
```

Note the jump to `maxsd` on `NaN` -- the instruction `x = max(x, 0)` used by unsigned conversion can be abused to convert `NaN` to `0` for signed integers as well.

```c
below_limit:
if (arg & 1) { // unsigned
maxsd:
    asm ("maxsd %1, %0" : "+x"(x) : "x"(0L));
}

// 32-bit or 64-bit signed conversion
asm ("cvttsd2si %1, %0;" : "=r"(out) : "x"(x)); // everything but i32 is converted as i64
if (arg == 0 || arg == 2) {
    asm ("cvttsd2si %1, %k0;" : "=r"(out) : "x"(x)); // i32 is converted as i32
}
```

Built-in signed conversion suffices for everything but `u64` values between `2^63` and `2^64 - 1`. Not to worry: IEEE-754 `double`s in this range can be converted by hand:

```c
if ((arg & 1) && (long)out < 0) { // intended destination u64, and i64 conversion failed
    unsigned long value;
    __builtin_memcpy(&value, &x, 8);
    out = (value << 11) | (1UL << 63); // move mantissa to the right position, set hidden bit
}
```


### FPU

Float-to-int conversions could convert `float` inputs to `double`s to halve code size. Not so easy in the other direction: `(float)n` and `(float)(double)n` can produce different results because both steps in the `i64` $\rightarrow$ `double` $\rightarrow$ `float` chain introduce rounding.

Duplication or code patching is necessary. Or is it?

The x87 FPU uses 80-bit `long double`s, which barely fit all `i64`s without rounding. Who knew FPU could be useful? Typed loads and stores reduce case handling further:

```c
// Load
long double ld;
if (arg == 1) { // load s32 as s32
    asm ("fildl %1" : "=t"(ld) : "m"(*stack_head));
} else { // load other types as s64
    asm ("fildq %1" : "=t"(ld) : "m"(*stack_head));
}

// Store
*stack_head = 0;
if (opcode < 0xb7) { // f32
    float f = ld;
    __builtin_memcpy(stack_head, &f, 4);
} else { // f64
    double d = ld;
    __builtin_memcpy(stack_head, &d, 8);
}
```

Large `u64`s misparsed as `i64`s can be fixed simply by adding `2^64` to the float, a trick I stole from GCC's codegen:

```c
if (arg == 2 && (long)*stack_head < 0) { // u64 and misparsed
    ld += 0x1p64;
}
```


### min/max

I've only applied unconventional, but textbook methods so far. This one is different.

Wasm defines `min` and `max` like this:

- If there's a `NaN` argument, the result is `NaN`.
- `-0` is considered less than `+0`.

x86 defines `min` and `max` like this:

- If there's a `NaN` argument, the result is the second argument.
- `-0` and `+0` are considered equal, with ties resolved to the second argument.

It's a subtle difference, but I want to make a standards-compatible interpreter, not something that works for two apps. Implementing `min` or `max` individually is doable:

```rust
fn wasm_min(x: f64, y: f64) -> f64 {
    if x /* unordered */ y {
        // There must be a NaN -- produce NaN as an output. Addition is cheaper than a constant.
        x + y
    } else if x == y {
        // Either same value, or -0 and +0. For the former, `|` is a no-op. For the latter, the
        // sign bit is set to negative if either argument is negative.
        f64::from_bits(x.to_bits() | y.to_bits())
        // For max: f64::from_bits(x.to_bits() & y.to_bits())
    } else if x < y { // for max: x > y
        x
    } else {
        y
    }
}
```

When merged into one handler, this requires patching two locations:

- The opcode in the signed zero handling, between `or` and `and`.
- The comparison, between `x < y` and `x > y`.

Only one patch can use the opcode LUT, leaving the other to calculate a constant. But there's an alternative. In x86, comparison is a two-step operation:

- `cmp a, b` stores compressed results of all possible comparisons in [flags](https://en.wikipedia.org/wiki/FLAGS_register).
- `jcc target` parses flags and jumps if the right condition is satisfied.

The result of "less" comparison is stored in the carry flag. If we could just invert it between `cmp` and `jcc` if the operation is `max`, we could avoid patching code.

Saving the flag to a register to `xor` it is too wasteful -- we need to operate on flags directly. [cmc](https://www.felixcloutier.com/x86/cmc) inverts the carry flag unconditionally, but skipping it on `min` requires comparing the opcode, which clobbers the carry flag. So we need another way.

`adc a, b` computes `a + b + carry` and, among others, sets the zero flag if the result is `0`. If we could set `arg = -1` for `min` and `arg = 0` for `max`, then `adc arg, 0` would set ZF on `min` + set carry (`x < y`) and on `max` + no carry (`x >= y`) -- the combination we need.

But `arg` is not free: it needs to be `0xeb` for `min` and `0xdb` for `max` due to patching between `or` and `and`, and these values are too far for ZF to work. However, the legacy parity flag, set if the number of `1` bits in the bottom byte of the result is even, remains receptive to small changes:

- `0xeb + 5 + 0` has 4 set bits (even).
- `0xeb + 5 + 1` has 5 set bits (odd).
- `0xdb + 5 + 0` has 3 set bits (odd).
- `0xdb + 5 + 1` has 4 set bits (even).

```x86asm
; valid for min
jc output_x

; valid for max
jnc output_x

; valid for both
adc arg, 5
jnp output_x
```

This is the first time I've used PF, and only [the second time](https://codegolf.stackexchange.com/a/246679) I've seen it in codegolf.


### Syscalls

Calculations without side effects are not terribly interesting. Let's talk about syscalls. `fd_read` is the most interesting one, since its implementation has to convert data between arrays of native and WASI structures:

```c
struct wasi_iovec {
    unsigned buf;
    unsigned buf_len;
} *src;

struct iovec {
    void *iov_base;
    size_t iov_len;
} *dst;
```

Here's a converter written by a noob, requiring four manual memory accesses:

```c
for (unsigned i = 0; i < count; i++) {
    dst->iov_base = memory + src->buf;
    dst->iov_len = src->buf_len;
    src++, dst++;
}
```

Here's one written by a pro, using x86's ancient stream reading/writing instructions:

```c
for (unsigned i = 0; i < count; i++) {
    asm volatile (
        "lodsl;" // src->buf
        "add %[memory], %%rax;"
        "stosq;" // dst->iov_base
        "lodsl;" // src->buf_len
        "stosq" // dst->iov_len
        : "+S"(src), "+D"(dst)
        : [memory]"i"(memory)
        : "rax", "memory"
    );
}
```

`lodsl` reads 4 bytes and increments the input pointer. `stosq` stores 8 bytes and increments the output pointer. This saves 16 bytes -- huge for a simple change.


### Compression

Are these thorough optimizations and hand-written assembly enough? Nope: at this point, the interpreter exceeds 4500 bytes. But I have an ace up my sleeve.

Demosceners are used to packing their demos. Programs compressed with tools like [Crinkler](https://github.com/runestubbe/Crinkler) don't just use external decoders like `gzip` -- that would be cheating, and compression ratio suffers anyway. Instead, it packs a fine-tuned [entropy decoder](https://en.wikipedia.org/wiki/Entropy_coding) written in assembly. It's a little slow to decode, even slower to encode, and requires plenty of memory, but it gets the job done.

But Crinkler targets 32-bit Windows, while I'm using 64-bit Linux. And I can't change this: wasting space on 64-bit arithmetic sucks as much as importing syscalls from NTDLL. I needed a new Crinkler, and the lazy bum I am, I poked [Yuki](https://github.com/sylfn) to do it for me.

The details are a little too technical, so I won't cover them in the entirety -- if you're interested, [this post](https://code4k.blogspot.com/2010/12/crinkler-secrets-4k-intro-executable.html) covers the Crinkler design, which is very similar to ours.

In a nutshell, we're compressing concatenated text and data with [arithmetic coding](https://en.wikipedia.org/wiki/Arithmetic_coding), loading it with a small bootstrapping decoder and jumping to it. The probabilities are computed with linear [context mixing](https://en.wikipedia.org/wiki/Context_mixing) according to static models that are brute-forced by the encoder. We operate on bit granularity, and predictors use a shared 8 MiB hash table with CRC32 as the hash function. We also perform [the E8 transform](https://en.wikipedia.org/wiki/BCJ_(algorithm)).

Notable differences from Crinkler are:

- The initial interval size is $2$, not $2^{31}$ -- this simplifies the decoder.
- We increment `c0` and `c1` and halve the wrong counter, instead of incrementing the correct one and (saturate-)halving the wrong one.
- We don't separate the file into code and data sections, since there's little data.
- Instead of explicitly storing weights, we allow models to repeat.

These choices are not necessarily the best for every situation, but we found they worked well here. There's likely a way to save 30-ish bytes, but there's little more we could fit in that space, so we decided to call it quits.

Overall, this reduces code size by about 40%. This shows how redundant x86 code is, but it's also close enough that we had to tweak a few parameters to get it to fit.


### Wrapping up

The actual story was much less linear than described here, but this post is already quite long, and I fear I'll bore people if I discuss more minutiae. If you're interested in more detail, I wrote down [a development log](https://github.com/purplesyringa/autumn/blob/master/log.md).

This project was a lot of fun to me ~~(insomnia excluded)~~, and it was a nice change of pace compared to my usual performance-related work. It was cool to see old techniques find their place in modern computing, like the parity flag, the x87 FPU, self-modifying code, and writing jump tables by hand.

The interpreter is [released on GitHub](https://github.com/purplesyringa/autumn) in all of its glory. It's called `autumn` because the punny name `winter` was already taken, and autumn is kinda like autistic winter. Supported features pass tests from the Wasm spec, though traps and validation aren't implemented, so e.g. memory accesses can go out of bounds, type checks are absent, and parsing can crash. If it wasn't clear by now, don't use this in prod.

I hope this was fun to read. Maybe it'll motivate you to try size coding as well!
