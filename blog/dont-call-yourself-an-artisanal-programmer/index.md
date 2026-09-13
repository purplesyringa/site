---
title: Don't call yourself an artisanal programmer
time: September 13, 2026
intro: |
    So I've been meaning to write about something, but got sucked into this rabbit hole, and now I'm kinda scared if I've been a subject of propaganda that worked on me.

    A common theme in software development AI discourse is that there are "serious" engineers who value the end result the most, and "artisanal" coders who value the experience of coding over the final product. This dichotomy doesn't correctly describe me and likely many others, but let's play along with it for now.

    I want my programs to be reliable. Patience and care are two important ingredients for this. When designing programs from ground up and taking my time, I just *know* that they're correct, and that any mistake is due to a typo. Unit tests, LLM reviews, and provers can guarantee the code is 99% right, but not that it's 100% right: optimal, maintainable, readable, and that it doesn't rely or break due to undocumented incompliant behaviors. I want that 100%. I can only begin to achieve this with deep connection to code, so I refuse to use LLMs, since they isolate me from the low-level details that matter and cannot guarantee correctness by construction.
---

So I've been meaning to write about something, but got sucked into this rabbit hole, and now I'm kinda scared if I've been a subject of propaganda that worked on me.

A common theme in software development AI discourse is that there are "serious" engineers who value the end result the most, and "artisanal" coders who value the experience of coding over the final product. This dichotomy doesn't correctly describe me and likely many others, but let's play along with it for now.

I want my programs to be reliable. Patience and care are two important ingredients for this. When designing programs from ground up and taking my time, I just *know* that they're correct, and that any mistake is due to a typo. Unit tests, LLM reviews, and provers can guarantee the code is 99% right, but not that it's 100% right: optimal, maintainable, readable, and that it doesn't rely or break due to undocumented incompliant behaviors. I want that 100%. I can only begin to achieve this with deep connection to code, so I refuse to use LLMs, since they isolate me from the low-level details that matter and cannot guarantee correctness by construction.


### Contrast

Which is where the dichotomy falls apart in my eyes. By the book, I'm an artisanal coder, since I don't use LLMs. But if the industry calls the opposite of "artisanal programmer" a "software engineer", isn't there a subtext that I'm not an engineer? This may seem like a minor point, but it matters on a subconcious level. Care, attention, and precision are *the* defining characteristic of engineering -- so why is this term appropriated by people who are becoming closer and closer to managers?

*Back in my day* (ahem), developers hated management that wouldn't allocate time to dealing with tech debt, and the "move fast and break things" attitude was considered childish. Nowadays, common knowledge among developers says that vibecoding (I'm using the term loosely) is the norm, and wasting time on reliability work is unserious. Who is in the right?


### Redefining terms

To me the answer seems obvious: you can't earn the title of Software Engineer by building stuff blindly. An engineer should know what they're doing. But let's ask someone more knowledgeable about this topic.

In 2021, Hillel Wayne ran [the crossover project](https://www.hillelwayne.com/post/are-we-really-engineers/), where he interviewed multiple people moving to software engineering from other engineering fields. His goal was to answer the question: do people with actual experience in both fields consider our job engineering? The answer was "yes", but here's the part that interests me more:

> That said, many of the crossovers also added an additional qualification: software engineering is real engineering, but a lot of people who write software aren’t doing software engineering. This is not a problem with them, rather a problem with our field: we don’t have a rich enough vocabulary to talk about what these developers do. Not everybody who works with electricity is going to be an electrical engineer; many will be electricians. And this is okay. [...] But we use things like "programmer", "software engineer", and "software developer" interchangeably. What is the difference between a software engineer and a software developer? Some people propose the word "software craftsman".

What jumps out to me is that the titles Wayne uses are completely opposite to the ones popular nowadays. He calls the people who care, and who today we call artisanal coders, "engineers", and on the opposite side, he calls those who we label vibecoders "craftsmen". At least to me, this is more intuitive naming! There is arguably more art in prompt engineering than writing code by hand.

"Artisanal" made sense as the antonym of "vibecoded" historically, but claiming this term freed up "engineer" and effectively gave up the debate on whether vibecoding is a form of engineering. Vibecoding became the obvious default, and artisanal coding the outlier. How did we get here?


### Talking points

If you look for this change in "common sense", you'll find it everywhere. The "only" "correct" approach changed *radically*. Why did we use to consider PVS-Studio posts promotional material, but now we worship automated review tools? How did we jump from strong type systems to free-form specifications so quickly?

Nothing like this has happened before in the programming community. Sure, there were arguments about which type system or web framework is better, and the common opinion changed over time, but this is different. A devoted believer in strong type systems would consider a PHP developer an idiot, but still a developer. Even a caricature Rust zealot was still a Rust programmer. And the arguments were based on whether one option is more reliable, easier to use or learn than the other, not whether you should worry about reliability or misuse at all.

This is different -- vibecoding is portrayed not as a *better* solution, but as the only *sane* solution, spitting in the face of past experience. Someone who doesn't use LLMs is not a software engineer, they're an artisanal coder. It's an attack on an identity.


### History

Redefining terms and portraying yourself as the sane side and others as beings not worthy of consideration is a new tactic in the software world. I've seen it before, though: it's very much part of the fascist playbook.

I remember the time when we were all joking "the antonym of vibecoder is software engineer", and then like a month passed and suddenly everyone was comfortable calling themselves artisanal programmers and leaving "software engineer" to "responsible" AI users. The term propagated with the speed of memes, seemingly without any natural reason.

I don't believe this is a psyop, but I do think we should be more mindful about what we call ourselves, because words have power. Real composers call themselves composers, not artisan composers; AI writers have to call themselves AI writers; but the term "programmer" has not been debated, and this has real-world consequences.

Many gamers who defend artists and hate AI with a passion turn 180 once you mention software: suddenly LLM code is acceptable, and obviously everyone uses LLMs anyway, and AI disclaimers for code are unnecessary. They are not experts, so rhetoric matters more than facts. And most of us suck at rhetorics!


### Why us?

We should've learned this lesson earlier, and I think I know at least one factor that caused us to lose this battle first among all professions. Before I wrote this post, I wondered why "artisanal" programming that requires education, hard work, and complex thinking to perform is considered a game, while spewing ideas into a textarea using methods spread by word of mouth is considered the real thing.

Now I'm thinking it's due to rampant anti-intellectualism in software communities.

Before Rust was popular and everyone knew it has its uses, what was the most common counterargument against it? That it's like raw math and unusable for practical purposes. The same thing is still said about Haskell, especially about monads, which Rust demonstrated can be easily understood (`Result`, `Option`, and `Future` are monads!). Hell, even pointers are considered complicated, because, oh no, *you have to learn something before using them*! We yearn for easy solutions, but what we actually mean by that is that we refuse to read and just want to copy-paste code from StackOverflow, oh, wait, wrong decade, Claude. We look at professions that have to study in college to work and say "actually, we're better than them and deserve to dictate how the world runs". If that's not anti-intellectualism, I don't know what is.

So *of course* we negate the benefits of education, and hard work, and thinking, and taking our time -- those things are woke and we're better than that. /s


### Now what?

We need to put up a better fight. The goal is not to flip the script and say avoiding AI is the only correct way to write software, because that won't work; the goal is to make sure AI-free coding keeps its place in public discussions and is not equated to recreational programming.

Terminology-wise, I'll use "AI-assisted coding" whenever AI is used and "AI-free software engineering" when doing stuff by hand. It keeps the AI usage notice, but flips the programming vs engineering half and drops the word "artisanal" that can be interpreted as a form of play. I think it's a good start, but ideas are welcome.

More generally, we need to challenge the assumption that AI-assisted programming is the only sane way to write code. The public understands that using AI-generated art for any purpose is icky; we need to convince it the same principle applies to AI-generated code. If a person witnessing AI "art" can feel lied to without knowing how to draw, there is no reason why this won't work for programs.

We need to talk about the care we put into software development, how we design code, the problems we're facing, and how beautiful the way we collaborate is. We should highlight that glitchful speedrunning is entertaining because we can see the glitches arise from understandable human mistakes, and how cool computer art is, and how heart-warming it is when a developer polishes their app to improve user experience. We need to, and I understand this is not our *forte*, talk about our humanity.

Keep safe in these trying times.
