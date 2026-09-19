---
title: Write while learning
time: September 19, 2026
intro: |
    When learning new topics, we always ask questions we can't find answers to. "Why are there two APIs that do seemingly the same thing?" "How do I achieve this goal?" "Why does this code not work even though it looks similar to the example?"

    As we research and get more familiar with tools, we gain understanding. At some point, we become experts and know how to answer earlier questions. But it's worth covering *how* we get from point A to point B to help others make progress, too. In my experience, it's common that the answers seem obvious post-factum, but there's a missing link between facing questions and knowing the terms to look for.

    Usually I have to get familiar with the project architecture, read its code, look at what it interacts with, scan the bug tracker, etc., before I built a model in my head that answers my questions. And then I discover that the model is easy to understand and has docs, and I agree with experts that it's a reasonable and well-designed model -- forgetting that I-the-novice failed to find it despite the documentation existing!
---

When learning new topics, we always ask questions we can't find answers to. "Why are there two APIs that do seemingly the same thing?" "How do I achieve this goal?" "Why does this code not work even though it looks similar to the example?"

As we research and get more familiar with tools, we gain understanding. At some point, we become experts and know how to answer earlier questions. But it's worth covering *how* we get from point A to point B to help others make progress, too. In my experience, it's common that the answers seem obvious post-factum, but there's a missing link between facing questions and knowing the terms to look for.

Usually I have to get familiar with the project architecture, read its code, look at what it interacts with, scan the bug tracker, etc., before I built a model in my head that answers my questions. And then I discover that the model is easy to understand and has docs, and I agree with experts that it's a reasonable and well-designed model -- forgetting that I-the-novice failed to find it despite the documentation existing!


### Example

For example: I recently got into Minecraft modding, and [KubeJS](https://kubejs.com/), a tool for reconfiguring Minecraft with JavaScript, uses syntax like this to add an item to a tag:

```javascript
ServerEvents.tags('item', (event) => {
  event.add('tag_name', 'item_name')
})
```

Immediately I'm left wondering: what is `ServerEvents`, and why are the operations performed in the closure? Is that closure invoked immediately and it's just a way to get access to `event`? Why is it called "event" if it doesn't react to any player action?

It turns out that the answer is: KubeJS integrates with a mod loader, in this case NeoForge, which [offers events](https://docs.neoforged.net/docs/concepts/events/). The examples on that page show events like "entity jumps", which are clearly game-related events, but at the very bottom we have:

> Lifecycle events run once in every mod's lifecycle during startup. [...] The registry events [...] include `NewRegistryEvent`, `DataPackRegistryEvent.NewRegistry` and, for each registry, `RegisterEvent`.

After more research, I understand that the closure I'm registering really is an event handler, and NeoForge delivers this event when the world is loaded and handles it synchronously. I also understand that this is closer to a mixin/patch point than an event, but it uses the same underlying mechanism and is thus called the same.

To obtain this information I had to:

1. Read KubeJS docs.
2. Read KubeJS code to learn about the connection with NeoForge.
3. Read NeoForge docs.
4. Read NeoForge code to learn about the connection to mixins.


### Resources

We all value learning resources, but I find that most often, we offer docs for beginners and for experts, but few for those *transitioning* from one to another.

I remember the time when I didn't know how Web worked, and all articles on the topic went like "your computer sends ones and zeros to Google, and Google sends ones and zeros back". But how do they reach Google specifically? Now I know that Ethernet uses certain bit sequences to start packets, and about IP addresses being resolved to MAC addresses via ARP, and about HTTP and cryptography. But I learned pretty much all of that by accident, like learning how packet boundaries work from a school teacher or finding out about ARP by seeing packets in WireShark.

Try it for yourself: tell me how someone who heard that HTTPS makes connections secure go from [the Wikipedia page on HTTPS](https://en.wikipedia.org/wiki/HTTPS) to [Diffie-Hellman](https://en.wikipedia.org/wiki/Diffie%E2%80%93Hellman_key_exchange) without prior knowledge that asymmetric crypto is the main thing that gives HTTPS its guarantees.

Of course, Wikipedia is the epitome of "experts writing for experts", but our accessible documentation is seldom better: "experts writing for 5-year-olds" is just as annoying when you need to know the details.


### My plea

I started this blog as a way to teach cool stuff to people who only know the basics. My goal is to raise readers to my own level, not just above baseline, like pop-sci journals do. Sometimes I have to make concessions and adjust complexity, but I think it works great overall. I try to follow the same approach when writing documentation: for each high-level API, I try to mention the low-level details it's based on, like which algorithm is used, to let people follow breadcrumbs and educate themselves.

But I-the-expert no longer remember all the issues I-the-novice was facing. The only time when I still remember my confusion and the terms I tried to search for, while also knowing the solution, is just after struggling and finally finding an answer.

Which is where I get to the title of the article. When you face a problem and spend days finding a solution, *write* about your successes -- or even failures. This will help others who are stuck on the same issue, and it will help experts find missing information or unclear wording in their docs ([which is why documenting failures is useful, too](https://en.wikipedia.org/wiki/Selection_bias)). Blogs, social networks, anything goes, really -- even if only your friends see it, some of them may still find it useful.

Even if it takes you a while to reach a simple conclusion, that's most likely not your fault -- quite the opposite, it's very useful to know that something trivial is so inaccessible! It's valuable to learn about such omissions: for any person like you, there are ten people who *could* easily comprehend the answer, but failed to find it. Maintainers will thank you for that, and if not them, then others who face the same problem will.
