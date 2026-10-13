---
title: Who is vfmaddrnd231pd??
time: October 12, 2026
discussion: https://news.ycombinator.com/item?id=50063138
intro: |
    `vfmaddrnd231pd` is a unique instruction from the FMA instruction set. Most instructions in that set have mnemonics like `vfmadd231pd`, but here `rnd` means something very specific. The Intel Software Development Manual documents it as:

    > **VFMADDRND231PD -- Fused Multiply-Add of Packed Double-Precision Floating-Point Values with rounding control**
    > 
    > [...] The immediate byte defines several bit fields that control rounding, DAZ, FTZ, and exception suppression (See Table 5-3).
---

`vfmaddrnd231pd` is a unique instruction from the FMA instruction set. Most instructions in that set have mnemonics like `vfmadd231pd`, but here `rnd` means something very specific. The Intel Software Development Manual documents it as:

> **VFMADDRND231PD -- Fused Multiply-Add of Packed Double-Precision Floating-Point Values with rounding control**
> 
> [...] The immediate byte defines several bit fields that control rounding, DAZ, FTZ, and exception suppression (See Table 5-3).

Most x86 floating-point instructions use the global rounding mode and settings, changing which is pretty slow. But `vfmaddrnd231pd` allows the rounding mode to be controlled on a per-instruction basis, and since FMA can emulate addition, subtraction, and multiplication, this mechanism effectively allows any arithmetic to have controllable rounding without AVX-512 -- making some algorithms that rely on rounding incredibly faster.

Or so it *would*, if this instruction actually existed. GNU assembler doesn't recognize it. The Intel C/C++ Compiler intrinsics `_mm_fmaddround_pd` and `_mm256_fmaddround_pd` that SDM mentions are absent not only from Clang, but from ICC as well. Trying to assemble the instructions by hand from the SDM opcode list reveals another issue:

> `VEX.DDS.128.66.0F3A.W1 B8 /r /ib`

`VEX` is the VEX prefix, but what is `DDS`? This opcode is the only mention of `DDS` in the SDM, and [this SO answer](https://stackoverflow.com/a/37354947/5417677) says that `DDS` is merely used for documentation and was removed from later SDM versions, but the fact that it's still present here is suspicious. And when you actually run the instruction on a CPU that supports the FMA instruction set, like my Haswell, it raises `#UD` ("UnDefined instruction")!

I already found it strange that the SDM only lists `vfmaddrnd231pd` and not other variants like using `pd`/`ps`/`sd`/`ss` or `132`/`213`/`231`, which other FMA instructions have, but this confirmed my suspicions. It was at that point that I downloaded a more recent version of the SDM and noticed that this instruction is absent in the 2026 edition of the SDM, despite being present in the 2024 version.

I must say I'm incredibly confused. This instruction doesn't look like a typo in the documentation, since the docs contain unique tables and pseudocode that isn't similar to any other instruction. My best hypothesis is that Intel wanted to add the `vfmaddrnd*` set, then found an issue with it at the last moment and removed it from the CPU, but didn't quite adjust the SDM correctly. But I have no way to confirm that.

If anyone reading this has a friend at Intel, I'd love to know the answer.

### Update

[seastarer](https://news.ycombinator.com/item?id=50067815) and [039234562456](https://news.ycombinator.com/item?id=50068769) gave me enough pointers to crack this.

Back in 2023, Intel failed to add AVX-512 support to consumer CPUs due to hardware limitations, but still wanted to add some goodies from AVX-512. That's how AVX10 was born -- a new extension that looked roughly like AVX-512, but had conditional support for 512-bit vector lengths.

AVX-512 uses [a strange instruction encoding](https://en.wikipedia.org/wiki/EVEX_prefix) with the `EVEX` prefix. For compactness, the bits responsible for the static rounding mode are reused for the operation width (128, 256, or 512 bits), so only scalar and 512-bit operations can customize rounding.

This was a problem for AVX10, which didn't support 512-bit operations: if it interpreted the same instruction as operating on 256-bit values instead of 512-bit ones, different CPUs would behave inconsistently. So AVX10 had to use a different encoding for its own custom rounding modes.

My theory is that originally, Intel decided that the easiest way to do this was to add an immediate byte that controls the rounding, which is how `vfmaddrnd231pd` and similar instructions for other arithmetic operations were born. Notice that it uses a `VEX` prefix, not the AVX-512-specific `EVEX` prefix -- this would allow CPUs without 512-bit vectors to avoid handling the `EVEX` prefix at all. Since they added an entire byte, they utilized it in full: 2 bits for the rounding mode, 6 bits for everything else they thought might be useful.

It seems like by 2024, Intel decided that this approach won't work. And I get it: it's odd that `vfmaddrnd231pd` was more powerful than AVX-512 instructions, but only supported 256-bit vectors. And if it supported 512-bit vectors, the rounding mode control built into AVX-512 would become obsolete for the most part.

So Intel bit the bullet, dedicated the last unused bit in the `EVEX` prefix to enabling rounding modes for 256-bit vectors, and reused the AVX-512 instruction encoding for AVX10. This also allowed them to reuse the AVX-512 assembly syntax and avoid introducing new mnemonics, at which point the `vfmaddrnd231pd` mnemonic and others become unnecessary. During this process, someone forgot to remove `vfmaddrnd231pd` specifically from the SDM draft.

By 2025, AVX-512 was considered table stakes due to the popularity of Zen 4, and Intel finally decided to make 512-bit support unconditional in AVX10. This made this entire hack unnecessary, so Intel [updated the AVX10 whitepaper](https://cdrdv2.intel.com/v1/dl/getContent/784343):

> Removed references to 256-bit maximum vector register size, enumeration of vector-length support, and **256-bit instructions supporting embedded rounding**.

...and submitted [patches to GCC removing 256-bit rounding support](https://gcc.gnu.org/pipermail/gcc-patches/2025-March/678195.html):

> In this new whitepaper, all the platforms will support 512 bit vector width (previously, E-core is up to 256 bit, leading to hybrid clients and Atom Server 256 bit only). Also, 256 bit rounding is not that useful because we currently have rounding feature directly on E-core now and no need to use 256-bit rounding as somehow a workaround. HW will remove that support.

We can partially confirm this theory by inspecting the patches. One of them is named:

> Revert "AVX10.2 ymm rounding: Support vfmadd{132, 231, 213}p{s, d, h} intrins"

...and removes multiple intrinsics, including `_mm256_fmadd_round_pd`, which is named very similarly to the documented intrinsic for the ghost instruction, `_mm256_fmaddround_pd`.

At a later point, someone at Intel realized that the `vfmaddrnd231pd` was still mentioned in the SDM and removed it.

Summing up: I believe that `vfmaddrnd231pd` is of AVX10 lineage. I think it was added during the prototyping stage of AVX10, then reworked to use a different instruction encoding, until it finally became unnecessary. Market pressures, timing constraints, and weird tech debt affected the design process enough that things had to be reworked on the fly, and at some point someone slipped up and accidentally documented an instruction that has never existed in released hardware.
