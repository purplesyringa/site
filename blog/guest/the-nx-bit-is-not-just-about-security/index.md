---
title: The NX bit is not just about security
time: September 4, 2026
upstream: https://sleirsgoevy.dev
intro: |
    > While I'm taking a short break from low-level programming, here's a story by a friend of mine, [Sonya](https://sleirsgoevy.dev/), about debugging a seemingly impossible bug in ARM code.

    This bug hunting saga started several months ago. While developing a bare-metal hypervisor on ARM64 for [postmarketOS](https://postmarketos.org/), I hit a strange bug: if I enabled the [CTR_EL0](https://support.arm.com/documentation/ddi0601/2026-06/AArch64-Registers/CTR-EL0--Cache-Type-Register?lang=en) intercept (which was the whole purpose of the HV, so could not be skipped), the phone would randomly lock itself up. After a few seconds, the watchdog kicked in and reset the system. At first I thought that the boot got slowed down so much that the system simply did not have enough time to boot, but disabling the watchdog did not help either (fortunately, the phone in question had removable battery, and I didn't have to wait several hours for it to discharge). So, I started digging deeper.
author: sleirsgoevy@gmail.com (Sonya Sireneva)
---

> While I'm taking a short break from low-level programming, here's a story by a friend of mine, [Sonya](https://sleirsgoevy.dev/), about debugging a seemingly impossible bug in ARM code.

This bug hunting saga started several months ago. While developing a bare-metal hypervisor on ARM64 for [postmarketOS](https://postmarketos.org/), I hit a strange bug: if I enabled the [CTR_EL0](https://support.arm.com/documentation/ddi0601/2026-06/AArch64-Registers/CTR-EL0--Cache-Type-Register?lang=en) intercept (which was the whole purpose of the HV, so could not be skipped), the phone would randomly lock itself up. After a few seconds, the watchdog kicked in and reset the system. At first I thought that the boot got slowed down so much that the system simply did not have enough time to boot, but disabling the watchdog did not help either (fortunately, the phone in question had removable battery, and I didn't have to wait several hours for it to discharge). So, I started digging deeper.

## Hypothesis #1: My emulation of MRS is wrong

On Aarch64, so-called Special Function Registers (which `CTR_EL0` is one of) are accessed using `MRS` and `MSR` machine instructions:

```armasm
mrs x3, ctr_el0 // read access
msr ctr_el0, x3 // write access
```

These instructions move data between the specified SFR and the specified general-purpose register (in this case, `X3`), with all other registers remaining unchanged. So the failure must have meant that I was either corrupting some of the registers I had to preserve, or not writing the real output register correctly. Thus the first two things I verified were the exception handler trampoline:

```armasm
trap_from_el1:
sub sp, sp, #256
stp x0, x1, [sp]
stp x2, x3, [sp, #16]
stp x4, x5, [sp, #32]
// ...
stp x28, x29, [sp, #224]
stp x30, xzr, [sp, #240]
mov x0, sp
bl handle_trap_from_el1
mrs x1, elr_el2
add x0, x0, x1
msr elr_el2, x0
ldp x0, x1, [sp]
ldp x2, x3, [sp, #16]
ldp x4, x5, [sp, #32]
// ...
ldp x28, x29, [sp, #224]
ldp x30, xzr, [sp, #240]
add sp, sp, #256
eret
```

And the stack allocation:

```armasm
.section .data.stack
.p2align 12
.long 0
.p2align 12
stack:
```

Both were correct, with no obvious signs of issues. I singlestepped the whole exception handler in QEMU and verified that it was acting exactly as expected.

To make sure things work smoothly on real hardware, I added debug prints before and after the exception handler, and it turned out that, on real hardware, the exception handler was not modifying any registers, even the intended output register.

The culprit turned out to be in this innocent invocation:

```c
msr_accessor_sort(
    msr_accessors,
    ((uintptr_t)msr_accessors_end - (uintptr_t)msr_accessors) / sizeof(struct msr_accessor)
);
```

As you may know, ARM is not Icache/Dcache-coherent, which means that modifications to the data do not automatically propagate to the instruction fetches: either the modified data may not have been committed to RAM yet, or the instruction cache might be holding stale cached data from before the write. And since the buffer contained executable machine instructions, sorting the array and later trying to execute it was not going to work. So I moved the sorting to the build phase and...

The debug prints showed that the handlers worked as intended. But the system still didn't boot.

## Hypothesis #2: out-of-spec hardware

<aside-start-here />

As you all know, x86(-64) hardware is only produced by two vendors -- Intel and AMD, so we can expect very consistent behavior across systems.

:::aside
No, OpenAI, you don't own the em-dash.
:::

On the ARM side the situation is different: while ARM does provide a reference implementation of the architecture, vendors are free to customize it at will, or even roll their own implementations. This means that ARM CPUs tend to have many subtle (and not so subtle) bugs, some of which were probably [considered features](https://github.com/torvalds/linux/blob/4fe89d07dcc2804c8b562f6c7896a45643d34b2f/arch/arm64/kernel/head.S#L520) by their developers. So the next obvious guess was that the CPU was somehow out-of-spec, and was not behaving as it was supposed to.

With that in mind, I added extra handlers to make sure that every exception is printed:

```armasm
vbar_el2:
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
b trap_from_el1
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
b trap_from_el1
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
bl unknown_trap
.p2align 7
```

The `unknown_trap` routine would then print out the `X30` register (`lr` for those of you more familiar with Aarch32), `ELR_EL2`, and other SFRs to determine the cause of the exception. However, none of this was actually firing.

The next best guess was that the kernel was panicking due to some wrong handling, so the next thing I did was using [/proc/last_kmsg](https://android.googlesource.com/kernel/common/+/c672528aec4a1cf6f3df7a6022e6823a20b20f8e) to read the crashed kernel's logs. Unfortunately for me, the last messages I got in the log were:

```
[    3.113676]  (0)[153:init]fs_mgr: Running /system/bin/e2fsck on /dev/block/platform/mtk-msdc.0/11230000.msdc0/by-name/userdata
[    3.125072]  (0)[158:e2fsck]random: e2fsck urandom read with 12 bits of entropy available
```

This meant that the kernel did not crash cleanly. I suspected that the `/dev/urandom` device was responsible for the crash, and patched it out of the kernel to test that hypothesis -- which made the boot go a bit further, but not much. Still a dead end.

At that point I had no idea where exactly things were going wrong, but I knew it had something to do with the intercepts, since disabling the intercepts entirely fixed the issue. So, I filtered the kernel for suspicious instructions using objdump:

```
$ aarch64-unknown-linux-gnu-objdump -D -b binary -m aarch64 kernel.orig | grep ctr_el0
```

This yielded 22 matches, which I manually patched in the binary to return the correct value. After booting the patched kernel, it was still unable to make it to Android, but `adb shell` worked, which meant that the patch was (at least somewhat) successful. After that I was able to reduce the patch to only a few "hot" instructions, but was still clueless about the actual reason for the hangups. At that point I suspected that my hypervisor and the kernel were somehow overwriting each other's memory, so I decided to rewrite the handling for `mrs x3, ctr_el0` (the instruction inside all the "hot" patches that actually mattered) in assembly without using any memory:

```armasm
// save X0, will use it as a scratch register
msr tpidr_el2, x0

// read the Exception Syndrome Register; the value we're interested in is 0x6232c061
// (https://esr.arm64.dev/#0x6232c061)
mrs x0, esr_el2

// compare x0 to 0x6232c061
// ARM does not support 32-bit immediates in instructions, so compare bit groups one by one
sub x0, x0, #0x61
ror x0, x0, #12
sub x0, x0, #0x32c
ror x0, x0, #12
sub x0, x0, #0x62
cbnz x0, 1f // if the ESR is incorrect, go to the generic handler

// increment the saved PC by 4, to account for the instruction's length
mrs x0, elr_el2
add x0, x0, #4
msr elr_el2, x0

// restore the saved X0, perform the requested read, and return to caller
mrs x0, tpidr_el2
mrs x3, ctr_el0
eret

1:

// restore x0 before falling through to the generic handler
mrs x0, tpidr_el2
```

And... it worked! At that point I knew that the logic itself was correct, and it was the C handler that was somehow causing troubles, so I started bisecting further.

```c
if(esr == 0x6232c061)
{
    asm volatile("mrs %0, ctr_el0":"=r"(regs[3]));
    return 4;
}
```

...worked. I looked up the address of the `msr_accessor` used for reading `CTR_EL0`, and found it to be at `msr_accessors+0x28`.

```c
if(esr == 0x6232c061)
{
    regs[3] = ((uint32_t(*)(void))(msr_accessors+5/*8 bytes per element*/))();
    return 4;
}
```

...didn't work. Is the relocation to blame? I checked the relocation code and tried to set the linkage base to be equal to the actual load address, but to no avail.

Then I decided to use the linker script to factor this out into a "new" symbol:

```
get_ctr_el0 = msr_accessors + 0x28;
```

```c
if(esr == 0x6232c061)
{
    uint32_t get_ctr_el0(void);
    regs[3] = get_ctr_el0();
    return 4;
}
```

And this ran correctly. So now I had two versions of semantically equivalent code, only one of which worked. Time for binary bisecting!

## Binary bisecting, or a poor girl's ICE

At that point I factored the offending code out into a separate function:

```c
static __attribute__((noinline,optimize(3))) void handle_mrs_x3_ctr_el0(uint64_t* regs)
{
    // asm volatile("mrs %0, ctr_el0":"=r"(regs[3])); // works
    // regs[3] = get_ctr_el0(); // also works
    regs[3] = ((uint32_t(*)(void))(msr_accessors+5))(); // does not work
}
```

And called it from the main handler:

```c
if(esr == 0x6232c061)
{
    handle_mrs_x3_ctr_el0(regs);
    return 4;
}
```

The issue still reproduced, which meant that I could now focus on a single function.

At first I thought that code size could be the culprit, so I added a bunch of NOPs into the beginning of the function, but to no avail. Then I disassembled it.

Working version:

```armasm
0000000040204fc0 <handle_mrs_x3_ctr_el0>:
    40204fc0:   a9be7bfd        stp     x29, x30, [sp, #-32]!
    40204fc4:   910003fd        mov     x29, sp
    40204fc8:   f9000bf3        str     x19, [sp, #16]
    40204fcc:   aa0003f3        mov     x19, x0
    40204fd0:   94000ab6        bl      40207aa8 <get_ctr_el0>
    40204fd4:   2a0003e0        mov     w0, w0
    40204fd8:   f9000e60        str     x0, [x19, #24]
    40204fdc:   f9400bf3        ldr     x19, [sp, #16]
    40204fe0:   a8c27bfd        ldp     x29, x30, [sp], #32
    40204fe4:   d65f03c0        ret
```

Broken version:

```armasm
0000000040204fc0 <handle_mrs_x3_ctr_el0>:
    40204fc0:   a9be7bfd        stp     x29, x30, [sp, #-32]!
    40204fc4:   f0000001        adrp    x1, 40207000 <phys_ceiling_names>
    40204fc8:   f944f821        ldr     x1, [x1, #2544]
    40204fcc:   910003fd        mov     x29, sp
    40204fd0:   f9000bf3        str     x19, [sp, #16]
    40204fd4:   aa0003f3        mov     x19, x0
    40204fd8:   9100a021        add     x1, x1, #0x28
    40204fdc:   d63f0020        blr     x1
    40204fe0:   2a0003e0        mov     w0, w0
    40204fe4:   f9000e60        str     x0, [x19, #24]
    40204fe8:   f9400bf3        ldr     x19, [sp, #16]
    40204fec:   a8c27bfd        ldp     x29, x30, [sp], #32
    40204ff0:   d65f03c0        ret
```

I started unifying the functions in assembly, making sure that the working version still works and the broken one still doesn't, until I arrived at this:

```armasm
handle_mrs_x3_ctr_el0:
stp x29, x30, [sp, #-32]!
mov x29, sp
str x19, [sp, #16]
mov x19, x0
adr x0, get_ctr_el0 // load x0 with the address of get_ctr_el0

#if 0 // broken version
blr x0 // call the function at address stored in x0
#else // working version
bl get_ctr_el0 // call the function get_ctr_el0
#endif

mov w0, w0
str x0, [x19, #24]
ldr x19, [sp, #16]
ldp x29, x30, [sp], #32
ret
```

It's easy to see that both versions are semantically equivalent -- which meant that either someone interrupts the code at that exact moment and fails to restore registers, or there is a microarchitectural bug. I quickly dismissed the first possibility: on ARM64, the only higher-privileged thing than a hypervisor is the trustzone, and it had no reason to receive any interrupts, so I assumed the latter. The SoC in this phone is a MediaTek MT6735 with Cortex-A53 cores, and Alisa helped me look through the list of Cortex-A53 errata, but we did not find anything remotely similar...

And then I remembered [a certain comment in Linux source](https://github.com/torvalds/linux/blob/841e384b841a3d89c50b4b2d6c5bb6abab1a7e39/arch/x86/boot/startup/map_kernel.c#L185). On some x86 systems, speculative access to some MMIO registers would cause the system to shut down. So I devised a thought experiment: if this was the case here, how would I prevent it? The easiest way to prevent any accesses to a physical address, including speculative ones, is to never have the address mapped, but that would interfere with the hypervisor's own use of MMIO space (it expects a 1:1 mapping of the whole address space as a design choice). I started contemplating a "lazy paging" scenario, where I would lazily map in ranges of memory that were architecturally accessed (CPUs don't signal page faults for speculative accesses), and then it hit me.

How do the two instructions differ? `blr x0` is dynamic, `bl get_ctr_el0` is static.

<aside-start-here />

How are dynamic dispatches different from static branches? Dynamic dispatches employ branch prediction, static branches know where they lead ahead of time and cannot mispredict.

:::aside
x86 AMD CPUs can actually mispredict an unconditional direct jump, but that's besides the point.
:::

What could an instruction mispredict to? ...Probably a null pointer, `0x0`. Not like it can spawn an address out of thin air.

What do I have in my 1:1 mapping at `0x0`? ...Eeeeeeh... The [bootrom](https://github.com/cyrozap/mediatek-lte-baseband-re/blob/c1620cb60d3ca4dbc90228745843bc8975c0b052/SoC/MT6735/Notes.md?plain=1#L8), which is locked out during bootloader initialization.

...And then I realized. The speculative accesses I was fighting were not data accesses, they were **instruction fetches**. And that meant that all I had to do is mark the pages as non-executable. Which I did and... the system booted all the way to Android, without any patches in the kernel, for the first time in half a year!

After I knew what to look for, a quick Google search for "arm64 speculative instruction fetch mmio" yielded the following gem from the [official ARM documentation](https://support.arm.com/documentation/102376/0200/Device-memory):

> There is a subtle distinction here that is easy to miss. Marking a region as Device prevents speculative data accesses only. Marking a region as non-executable prevents speculative instruction accesses. This means that, to prevent any speculative accesses, a region must be marked as both Device and non-executable.

## Not just security

Being somewhat of a hacker myself, I always considered Data Execution Prevention (DEP) to be exclusively a security measure invented to combat stack overflow attacks, so I didn't bother to implement it: the hypervisor was never meant to run in production, so defense-in-depth would be overengineering.

<aside-start-here />

But it turns out that on ARM it's not about security at all -- instead, it's essentially an attribute bit. ARM guarantees that, for regions mapped as Device memory (newspeak for MMIO), there will be no speculative accesses. However, that only applies to data accesses -- instruction fetches treat any executable memory as Normal memory. The only way to prevent a region of memory from being speculatively executable is to prevent it from being executable **at all**.

:::aside
A possible workaround, in case you *have* to run code from Device memory, would be to disable the Icache for the duration of its execution. However, ARM documentation says such accesses are still illegal and discourages this.
:::

Also, since I had to implement a way to map non-executable memory anyway, I decided to map all memory except the payload as non-executable, so my hypervisor now finally has a bit of defense-in-depth too!
