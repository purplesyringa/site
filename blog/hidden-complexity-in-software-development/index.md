---
title: Hidden complexity in software development
time: July 2, 2025
discussion: https://www.reddit.com/r/programming/comments/1lpk0fc/hidden_complexity_in_software_development/
intro: |
    This is a tech phenomenon that I keep getting blindsided by no matter how much I try to anticipate it.

    Physical work *feels* difficult. You can look at someone and realize you don't have nearly as much stamina, and even if you did, it still *feels* demanding.

    Research *feels* difficult. You're tasked with thinking about something no one else has considered yet. That rarely happens even outside of science -- try to tell a unique joke.

    But non-algorithmic programming? You're telling a machine that precisely follows instructions what you want it to do. At best, you're a technical translator. You're not working towards a PhD degree. You're just wiring things together without creating anything intrinsically *new*. It looks *simple*, and so it *feels* easy.

    > lol. lmao, even.

    Experience shows that it's anything but easy, but it's always been hard for me to pinpoint exactly why that is the case. And I think I've finally found a good answer.
---

This is a tech phenomenon that I keep getting blindsided by no matter how much I try to anticipate it.

Physical work *feels* difficult. You can look at someone and realize you don't have nearly as much stamina, and even if you did, it still *feels* demanding.

Research *feels* difficult. You're tasked with thinking about something no one else has considered yet. That rarely happens even outside of science -- try to tell a unique joke.

But non-algorithmic programming? You're telling a machine that precisely follows instructions what you want it to do. At best, you're a technical translator. You're not working towards a PhD degree. You're just wiring things together without creating anything intrinsically *new*. It looks *simple*, and so it *feels* easy.

> lol. lmao, even.

Experience shows that it's anything but easy, but it's always been hard for me to pinpoint exactly why that is the case. And I think I've finally found a good answer.


### Lithium

I've recently started caring about [Lithium](https://github.com/iex-rs/lithium) again and did some work on rough edges.

At the API level, all Lithium does is provide `throw` and `catch` functions to simulate typed exceptions with panics or a more low-level mechanism. It's not a good high-level construct, but it's a useful tool nevertheless.

As a prototype, it could be implemented in 50 lines max. Obviously, optimizing for performance increases the LoC count significantly, but it still seems like it ought to be manageable.

But there's 200 commits in this repo. There's a ton of breakage and various small issues that crop up and suddenly I can't just say this project is finished, forget about it, and use it as foundation for the next one.


### The issues

Lithium relies on some low-level rustc mechanisms and nightly features, so it needs a CI to let me quickly react to things changing. I automatically [run a CI job](https://github.com/iex-rs/lithium/blob/147520243e1eb9e2679e7ed612254055bd9c7c09/.github/workflows/ci.yml#L7) every week, but I'm thinking about increasing the rate because I'm uncomfortable with breakage being found a bit later than I'd like.

You might think that breakage mostly happens due to changes to nightly features, but that isn't the case. Lithium has quite a bit of platform-specific code, so I run CI on many targets. And oh boy, do they work *badly*.

- MIPS targets have been [absolutely broken](https://github.com/rust-lang/rust/issues/102722) until this spring.
- Windows [arm64ec](http://www.emulators.com/docs/abc_arm64ec_explained.htm) oscillates between working and breaking spectacularly. That's surprising for a Tier 2 target. When I added CI, I stumbled upon [a certain issue](https://github.com/rust-lang/rust/issues/138541) that was fixed just a week ago, and less than a week later [it broke again](https://github.com/rust-lang/rust/issues/143253).
- Wasm exception handling support is... spotty. I'll need to see if things have changed, but there have been [several](https://github.com/rust-lang/rust/issues/132416) different [issues](https://github.com/rust-lang/rust/issues/135665), which I've [worked on for a bit](https://github.com/rust-lang/rust/pull/136200), but then I realized quite a few UI tests [fail on Emscripten](https://github.com/rust-lang/rust/pull/136199) and it's all a bit too overwhelming for me to resolve in a structured manner.
- Even on x86_64, LLVM [miscompiles](https://github.com/llvm/llvm-project/issues/112943) unwinding from a function that uses a foreign ABI (e.g. when an MS ABI function is invoked from a GNU ABI function).
- Oh, and if that wasn't enough, there's also [a bug in Wine](https://bugs.winehq.org/show_bug.cgi?id=57700) that causes code compiled for `*-pc-windows-gnullvm` to have unaligned thread locals. There's [an unmerged fix](https://gitlab.winehq.org/wine/wine/-/merge_requests/7251). Also, `panic!` [just hangs](https://github.com/rust-lang/rust/issues/135717) on `i686-pc-windows-gnu` under Wine, but I can't even tell if it's a Rust bug or a Wine bug.

So ultimately, the main reason Lithium is so unstable is external design deficiencies and bugs. It's logically simple, but the lack of a reliable foundation forces me to use hacks or abandon otherwise good approaches.

Or I can desperately try to fix upstream. I can patch rustc, I can maybe touch unwinders and Cranelift. LLVM is where I draw the line, and it looks like that's just not enough.

This fragility shows up anywhere you look. A significant chunk of complexity comes from setting up CI. You'd think people had perfected this by now, but apparently not:

- rustc does not support the `aarch64-pc-windows-gnu` target. `x86_64-pc-windows-gnu` and `aarch64-pc-windows-gnullvm` are both supported. I don't know why.
- GitHub Actions [doesn't install Rust on aarch64](https://github.com/actions/partner-runner-images/issues/77) by default. So I have to install it [by hand](https://github.com/iex-rs/lithium/blob/9e7a1b551231d87d13746d57af0de4b7fb36eb7e/.github/workflows/ci.yml#L228) and then also [install MinGW by hand](https://github.com/iex-rs/lithium/blob/9e7a1b551231d87d13746d57af0de4b7fb36eb7e/.github/workflows/ci.yml#L241) to support the `gnullvm` target.
- Wine [hangs](https://github.com/actions/partner-runner-images/issues/31) on aarch64 Ubuntu on GitHub Actions, so I have to [use a non-default CI image](https://github.com/iex-rs/lithium/blob/9e7a1b551231d87d13746d57af0de4b7fb36eb7e/.github/workflows/ci.yml#L318).
- Wine [doesn't support](https://bugs.winehq.org/show_bug.cgi?id=58092) the arm64ec target out of the box yet, which means I can't test it on CI and have to trust that running code on real Windows will catch all the bugs.
- [Cross](https://github.com/cross-rs/cross), the Cargo wrapper for cross-platform building and testing via emulators like qemu, has odd bugs I don't even know how to describe.
- Rust standard library has quite a few (well-controlled) memory leaks that [cargo valgrind](https://github.com/jfrimmel/cargo-valgrind) [hasn't always suppressed](https://github.com/jfrimmel/cargo-valgrind/issues/111), and there's still [some new leaks](https://github.com/jfrimmel/cargo-valgrind/pull/127) discovered occasionally. Even worse, these leaks are non-deterministic.
- There don't seem to be existing tools for automatically running tests under WASI or cross-compiling tests (which is necessary because I don't want to run the whole compiler suite under Wine, just the tests themselves), so I had to [make my own](https://github.com/iex-rs/lithium/tree/ad96472cecccb15fc6b2ba8639f37030d0796e69/ci).
- `cargo test --target <target>` has skipped over doctests without me knowing, and then started running them after an update, and that's revealed problems like having to add rustc flags to both `RUSTFLAGS` and `RUSTDOCFLAGS`.

In fact, I think that nightly-only and internal compiler features are *the least* of my worries. They *shouldn't* just work, yet they absolutely do. Yes, there's some code smell like [having to special-case Miri](https://github.com/iex-rs/lithium/blob/ad96472cecccb15fc6b2ba8639f37030d0796e69/src/backend/panic.rs#L60), and I'm not *happy* about relying on std or rustc internals, but them being less of a problem than anything else is telling.


### Foundations

It's infuriating that the tools that are supposed to help us lead us to our demise. And I think this is a common trope in software development.

If you need to solve a complex algorithmic problem, or if you need to optimize a program, you can usually do that. It might be tricky, you might need to research something or write ugly `unsafe` code, but then you implement it and it *works*. You write a clever pile of code and it's *done*, until the requirements change significantly.

But reliable products are more than code snippets. They always make assumptions, like a frontend developer assuming the JavaScript runtime works correctly, a Rust programmer trusting LLVM, or the Docker runtime trusting Linux not to be stupid. And if this trust fails, you start to lose your sanity. Nothing you can do is *guaranteed* to solve the issue. It's a vibes thing, it's a "just ship it and hotfix if something breaks" world, it's pure madness.

And it's *real*. Only for societal reasons, only because someone didn't consider an edge case somewhere and now fixing that requires more effort than anyone wants to invest, but that doesn't make it any less real. It sucks, and it shouldn't have happened, and it wouldn't if we didn't subscribe to the bollocks "worse is better" ideology, but now we have to live with the consequences.

Computing is now obscure, unreliable sorcery. To program is to harness this inscrutable magic. And when you reframe it this way, it finally feels difficult.
