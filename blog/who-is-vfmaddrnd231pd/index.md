---
title: Who is vfmaddrnd231pd??
time: October 12, 2026
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
