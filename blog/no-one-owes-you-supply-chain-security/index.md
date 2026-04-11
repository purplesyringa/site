---
title: No one owes you supply-chain security
time: April 11, 2026
intro: |
    In case you're unaware, I'm not a developer. I'm actually an autistic catgirl annoyed by suboptimal use of computing power, and fixing that happens to involve programming. Crucially, it also includes discussing foundational technology with people behind the scenes, and apparently that makes me more aware of social aspects of this sphere.

    So, I have *opinions* about [criticism of crates.io for supply-chain attacks](https://kerkour.com/rust-supply-chain-nightmare). After a dozen similar articles, I have some select words to voice about why it's off the mark.
---

In case you're unaware, I'm not a developer. I'm actually an autistic catgirl annoyed by suboptimal use of computing power, and fixing that happens to involve programming. Crucially, it also includes discussing foundational technology with people behind the scenes, and apparently that makes me more aware of social aspects of this sphere.

So, I have *opinions* about [criticism of crates.io for supply-chain attacks](https://kerkour.com/rust-supply-chain-nightmare). After a dozen similar articles, I have some select words to voice about why it's off the mark.


### Typo-squatting

Before I cover the main point, let's talk about about how supply-chain attacks happen in the first place, and why some common ideas for fixing them don't work out.

There are multiple reasons when a malicious dependency is added to a project. The least discreet reason this can happen is typo-squatting. It happens when a malicious library has a name similar to a real library, e.g. `num_cpu` vs `num_cpus`. Commonly cited solutions include using direct URLs or namespacing.

Well, let's see if that helps. Say you get a PR adding the following lines to `Cargo.toml`:

```toml
[dependencies]
bitflags = { git = "https://github.com/bitflags/bitflags" }
itertools = { git = "https://github.com/itertools/itertools" }
rand_core = { git = "https://github.com/rust-random/rand_core" }
```

One of these URLs is fake. Can you tell which one? It's `itertools` -- the correct URL is https://github.com/rust-itertools/itertools. https://github.com/itertools is a random account. https://github.com/rust-bitflags is not registered at all, by the way.

If you think you can remember the URLs for each package you use, you're probably wrong. Since many crates are managed by GitHub organizations, not individuals, it isn't even enough to remember that you can (likely) trust `dtolnay` and `BurntSushi`. Though this still isn't conservative enough -- https://gitlab.com/BurntSushi is free and and https://glthub.com is on sale, so attackers have plenty other choices.

By making crate IDs longer, whether by namespacing within crates.io, GitHub organizations, or via domains, you only make it harder for users to remember them precisely, and thus harder to recognize typo-squatting.


### Sandboxing

Rust gives build scripts and procedural macros full access to your PC. Worse, `rust-analyzer` runs `cargo check` when you open the project directory, so it can effectively become a 0-click RCE.

Some people tried to solve this. There's [an open issue](https://github.com/rust-lang/rfcs/issues/1515) for `build.rs` sandboxing, and there were [some experiments](https://github.com/dtolnay/watt) about compiling procedural macros to WebAssembly.

But this is hardly workable. While `cargo build` can become safe, you usually run `cargo test` or `cargo run` immediately afterwards, which is impossible to sandbox. Making Rust development secure involves more than build time and requires powerful system-level isolation that `cargo` alone cannot be responsible for.


### Code in VCS

An oft brought-up issue is that the code on `crates.io` and in Git don't always match.

To begin with, this is not trivial to solve. You can't just turn crates.io into a DNS, mapping crate names to repository URLs, since [crates.io is designed](https://doc.rust-lang.org/stable/cargo/reference/publishing.html#cargo-yank) to avoid giving crate maintainers the ability to break downstream consumers by deleting stuff:

> One of the major goals of crates.io is to act as a permanent archive of crates that does not change over time, and allowing deletion of a version would go against this goal.

This restriction was likely set due to [the left-pad incident](https://en.wikipedia.org/wiki/Npm_left-pad_incident), when a popular library was deleted from `npm`, breaking CI builds. `npm` could quickly fix this *because* it's centralized. Thin crates.io wouldn't stand a chance, so it saves and serves copies.

crates.io could still pull files from the repo on `cargo publish`. But if the maintainer can just force-push afterwards, it's not a good security mechanism.

Maybe crates.io could periodically scan repositories for history changes. But what does that mean exactly? Does removing the release commit from `master`, but keeping it on a tag count? What if I host the repo on a custom forge, which serves one history to the crates.io `User-Agent` and different history to the rest of us?

Or maybe there's a good reason to have different code in Git and `crates.io`. If the crate contains autogenerated code, you should probably generate it in CI on release. Wouldn't want to run expensive codegen in `build.rs` on each install, would you?

Every option has downsides: they can break existing packages or have false-positives on [benevolent rewrites](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository). I'd still like `cargo audit` to scan repositories, but it can't be a hard limit, and that means it can be designed around.


### Moderation

All these issues have an unacknowledged shared assumption that keeping malicious code off crates.io is "Rust's" responsibility. That if you decide to use a dependency and then `cargo add totally-safe-package` steals your credentials, it's an inherent fault of crates.io. Which is really misplaced if you think about how Rust is developed.

I'm sure many of you use open-source software and remember the MIT license:

> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, [...], FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. [...]

This applies just as much to Rust. Despite its popularity, Rust is not sponsored by large companies, like Microsoft or RedHat, in the same sense that GitHub or Linux are. The compiler and `std` are primarily developed by volunteers, who don't get anything out of it except for rare donations from other members of the community.

The fact that this scheme works *at all* is ridiculous to me. Perhaps it's so unbelievable, that people think there's secret employment in Rust Foundation or something, when in fact it only paid four software engineers [in 2024](https://rustfoundation.org/wp-content/uploads/2025/10/form-990-2024.pdf), excluding grants.

So it feels bonkers to me when people say they know a solution to supply-chain attacks, such as (underline the correct variant):

- Making Linux distro maintainers responsible instead (duplicating work).
- Sandboxing as much as possible (which is to say, not a lot and not by much).
- Moving stuff into the standard library (overloading T-libs more than it already is).
- Your other favorite technical solution.

Futilely fixing societal problems with technology is every developer's forte, but let's just be real for a second. The issue is the lack of manpower, not that no one made a nice GUI for blockchain-powered Web of Trust yet, or whatever.

We're not *nearly* close to the level of security a centralized registry can provide. On the software side, [in 2025](https://rustfoundation.org/media/strengthening-rust-security-with-alpha-omega-a-progress-update/) Rust teams made or piloted tools for typo squatting detection, dynamic build script analysis, and real-time code scanning. On the personal side, Rust Foundation hired [on-call engineers](https://github.com/rust-lang/team/pull/1877) in 2025 and [a second infrastructure engineer](https://rustfoundation.org/media/welcoming-infrastructure-engineer-ubiratan-soares-to-the-rust-foundation-team/) in 2026. If that sounds overdue, well, they had [net loss in 2023](https://rustfoundation.org/wp-content/uploads/2025/10/form-990-2024.pdf) -- software isn't cheap.

You can't expect the same level of security from a small, pre-moderated, stable-release, private-funded registry, like Ubuntu's, and a large, post-moderated, rolling-release, mostly-voluntary registry. The scale doesn't compare, and suggesting that complicating an overworked team's job will resolve the issue is just insulting.


### Audit

What I'm saying is that you're responsible for auditing the crates you use. Maybe that's not how it'd work in a perfect world, but in our capitalist hellscape no one else can be argued to be accountable for this, which leaves you, the crate user.

And Rust gives you all the tools for this.

Lockfiles limit installation to verified versions. `cargo add <dep>@<version>` installs the crate at a specific version. [cargo-vet](https://mozilla.github.io/cargo-vet/) can be useful. crates.io shows a 90-day download plot, which is difficult for malicious crates to simulate, and is responsive to malware reports via email. If you're feeling adventurous, you can check [the submitted crate sources](https://docs.rs/crate/crabtime/1.1.4/source/) by clicking "browse source" in the crate's sidebar. A quick check covers most simple attacks. `cargo update --dry-run` shows the list of outdated crates without actually updating dependencies so that you know which ones to recheck.

For sandboxing, [cargo-chef](https://github.com/lukemathwalker/cargo-chef) is the most straightforward way to sandbox builds, aside from Nix isolation. Supporting `rust-analyzer` and `cargo run` is trickier, but I suspect you can often just use `firejail`. Bonus points if someone shares their script for this!

Few of us are security researchers, but in all fairness, much of it is common sense and a little curiosity. And if you don't audit code, then who will?
