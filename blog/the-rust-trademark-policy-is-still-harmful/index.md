---
title: The Rust Trademark Policy is still harmful
time: November 10, 2024
discussion: https://www.reddit.com/r/rust/comments/1gnz5sm/the_rust_trademark_policy_is_still_harmful/
intro: |
    Four days ago, the Rust Foundation [released](https://blog.rust-lang.org/2024/11/06/trademark-update.html) a new [draft](https://drive.google.com/file/d/1hjTx11Fb-4W7RQLmp3R8BLDACc7zxIpG/view) of the Rust Language Trademark Policy. The previous draft caused division within the community several years ago, prompting its retraction with the aim of creating a new, milder version.

    Well, that failed. While certain issues were addressed (thank you, we appreciate it!), the new version remains excessively restrictive and, in my opinion, will harm both the Rust community as a whole *and* compiler and crate developers. While I expect the stricter rules to not be enforced in practice, I don't want to constantly feel like I'm under threat while contributing to the Rust ecosystem, and this is exactly what it would feel like if this draft is finalized.

    Below are some of my core objections to the draft.
---

Four days ago, the Rust Foundation [released](https://blog.rust-lang.org/2024/11/06/trademark-update.html) a new [draft](https://drive.google.com/file/d/1hjTx11Fb-4W7RQLmp3R8BLDACc7zxIpG/view) of the Rust Language Trademark Policy. The previous draft caused division within the community several years ago, prompting its retraction with the aim of creating a new, milder version.

Well, that failed. While certain issues were addressed (thank you, we appreciate it!), the new version remains excessively restrictive and, in my opinion, will harm both the Rust community as a whole *and* compiler and crate developers. While I expect the stricter rules to not be enforced in practice, I don't want to constantly feel like I'm under threat while contributing to the Rust ecosystem, and this is exactly what it would feel like if this draft is finalized.

Below are some of my core objections to the draft.


### Modifications

The draft says:

> The most basic rule is that the Rust trademarks cannot be used in ways that appear (to a casual observer) official, affiliated, or endorsed by the Rust Project or Rust Foundation, unless you have written permission from the Rust Foundation.

The phrase "to a casual observer" indicates to me that this was poorly thought out. To a casual observer, a fork of the Rust repository on GitHub may appear legitimate unless the `README` is changed. This implies that legally, anyone who forks the compiler, Cargo, or any other crate under `rust-lang` might be at risk, even if they're just experimenting or planning to submit a PR soon.

Note that the policy didn't just *overlook* forks accidentally! Elsewhere, the draft explicitly allows:

> Publicly distributing a modified version of the Rust programming language, compiler, [...], provided that the modifications are limited to:
> - code adjustments for the purpose of porting to a different platform, architecture, or system, or integrating the software with the packaging system of that platform.

Crucially, this list doesn't include feature developments or bug fixes.

While I'm sure this wasn't the point of this rule, it is legally enforceable anyway and *will* put the community at risk. I really hope this gets repharsed to something like:

> The most basic rule is that the Rust trademarks cannot be used in ways that falsely appear to be official or affiliated with the Rust Project or the Rust Foundation. Modifications of official Rust materials are exempt from this rule, as long as it is clear from context that this is a derived work rather than the original.


### Ecosystem

I'm concerned about how terms like "the Rust ecosystem" would be handled under this policy.

It is common to describe certain core crates like `serde` or `tokio` as *the* Rust crate for serialization, asynchronicity, etc. As far as I can see, this policy forbids such usage. Although there are few official examples of this wording online, it's extermely common in informal communication and blog posts.


### Languages

Regarding the word "Rust", the draft says:

> They may not be used: [...] to refer to any other programming language;

This appears to restrict phrases like "alternative to Rust", "reimplementation of Rust", and similar constructs. While such uses might be legal anyway, in my opinion, this sends a terrible message to the community.

Rust introduces several unique concepts that are certain to drive innovation in programming language theory, so I'd hope using "Rust" would be explicitly allowed for languages and compilers that clearly differentiate themselves from the official "Rust" language, such as [LCCC](https://github.com/lccc-project/lccc) or [mrustc](https://github.com/thepowersgang/mrustc).


### Teaching

The draft says:

> Using the Rust trademarks for social and small non-profit events like meetups, tutorials, and the like is allowed for events that are free to attend. [...] For commercial events (including sponsored ones), please check in with us.

In my opinion, this is overly restrictive in practice, perhaps more limiting than anticipated.

One problem is that we live in capitalism, so few events are free to attend due to the venue cost, if nothing else. This draft therefore requires explicit permission for meetups and workshops that are clearly not affiliated with the Rust Foundation in any way.

A more significant problem is this: for Rust's future to be bright, it needs to be teachable, a point well-understood by the core developers. However, college and online courses are often paid. I'm not aware of any other popular programming language that requires certification of teaching materials, and I fear that this requirement will deter academic folks from teaching Rust.

To avoid this outcome, both paid and for-profit events must be permitted as long as they don't appear to be endorsed by the Rust Foundation.


### Conferences

The draft says:

> The words “RustCamp,” “RustCon”, or “RustConf” cannot be used without explicit permission.

What happens when you restrict people from using certain words? That's right, they stop using them. ~~(Streisand effect? What's that?)~~

Does the Rust Foundation want people to stop talking about the camps and conferences altogether? Under this limitation, you can't even retweet a post about them or share your experiences about the camp or the conference.


### Conclusion

The policy *still* prioritizes protecting the Rust Foundation over protecting the legacy of Rust, the language. This is unhealthy, and this was proven time and again to be a bad idea by Borland, Sun, and other companies.

I love Rust, and I hope to continue developing ecosystem crates. One day, I hope to contribute to `rustc`. I do this because I want to be part of something that showcases the best of humanity, instead of worrying about how half the countries in the world want me dead and now the Rust Foundation can sue me if I fork a repo. Don't push me out and take away my one way to escape the horrors.

If you want to send your feedback to the Rust Foundation, [here's a feedback form](https://docs.google.com/forms/d/e/1FAIpQLSeU1Ocopa0v9UZn_ZSTkKQM7gqZIrt63lCFz-xtogcFHMtkAg/viewform). You have until November 20.
