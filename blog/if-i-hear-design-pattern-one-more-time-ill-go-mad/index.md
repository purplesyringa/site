---
title: If I hear "design pattern" one more time, I'll go mad
time: September 4, 2025
discussion: https://www.reddit.com/r/programming/comments/1n8nb8z/if_i_hear_design_pattern_one_more_time_ill_go_mad/
intro: |
    Here, read the intro of the Wikipedia page for [Command pattern](https://en.wikipedia.org/wiki/Command_pattern) with me:

    > In object-oriented programming, the command pattern is a behavioral design pattern in which an object is used to encapsulate all information needed to perform an action or trigger an event at a later time.

    You know what I call this? A function.

    > This information includes the method name, the object that owns the method and values for the method parameters.

    Here's your command:

    ```javascript
    const command = () => object.method(arg1, arg2);
    // or in PHP, or in Rust, or in C++...
    ```

    Every time I hear about a design pattern I don't recognize, I get afraid that I'm missing some crucial idea, and then I Google it and find out that I've been unknowingly using it since I started programming. Yeah, bro, I used the data store pattern earlier today (a variable).
---

Here, read the intro of the Wikipedia page for [Command pattern](https://en.wikipedia.org/wiki/Command_pattern) with me:

> In object-oriented programming, the command pattern is a behavioral design pattern in which an object is used to encapsulate all information needed to perform an action or trigger an event at a later time.

You know what I call this? A function.

> This information includes the method name, the object that owns the method and values for the method parameters.

Here's your command:

```javascript
const command = () => object.method(arg1, arg2);
// or in PHP, or in Rust, or in C++...
```

Every time I hear about a design pattern I don't recognize, I get afraid that I'm missing some crucial idea, and then I Google it and find out that I've been unknowingly using it since I started programming. Yeah, bro, I used the data store pattern earlier today (a variable).


### What is a pattern?

It's hard to formulate the reason I'm so annoyed because it's hard to define what we're talking about in the first place.

"Iterator" is called a pattern, but it's not a pattern in the same sense that "[mediator](https://en.wikipedia.org/wiki/Mediator_pattern#C#)" is a pattern. Iterators are an interface formalized in the programming language or ecosystem. Mediators are templates for class hierarchies. Iterators are rigid and have to implement the same interface to be usable across abstraction boundaries. Mediators are merely a best practice, a recommendation. So I have no idea why we call both of them behavioral patterns.

In practice, when I say "iterator", I don't mean a design pattern, I mean a type implementing `IEnumerator` -- even if the language doesn't support interfaces and it's all in my head. I don't think about iterators solving design problems, I don't think of them as pieces of architecture, they're just an end to a mean. For all I care, a per-collection `forEach` method taking a closure is another valid approach to iteration -- the only major difference is that iterators have better language and ecosystem support, so that's what I use.


### So?

I'm not saying you shouldn't use patterns in general, nor that naming patterns is bad in general. I'm saying that the word "pattern" itself is meaningless, and that "glue" patterns specifically are so malleable it doesn't make sense to name them.

The problem with glue patterns is that they do not form *concepts* in the same sense that iterators do -- they don't add tangible value. For example, all the "strategy" pattern really means is "use an `interface`". It's not just that it's implemented via interfaces, it's literally the main reason interfaces exist in the first place. Why do we need another term for that?

Point is: patterns are *useful*, but it doesn't make sense to *focus* on them. I "iterate over a collection", not "use an iterator". I "build an abstraction", not "use a strategy". I "pass a closure", not "create a command".


### They are jarring

Good design is invisible. It's not supposed to get in your way.

When you say "facade", my mind screeches to a halt, because I spend most of my time working on code, and that's not a phrase I see in code. To interpret it, I need to switch gears and translate it from the human language to the programming language. Only then can I fully internalize how it fits into my design. That's the exact opposite of staying out of the way.

I'm not going through that process because I don't know what a facade is, or because I haven't used it. It's because, when I have a problem, I stare at the type hierarchy and add links that allow the code to typecheck. Those links do turn out to be facades, adapters, strategies, etc. most of the time, but I rarely have to match the problem to patterns to find one that fits, or even think about applying a pattern -- it happens by accident. This means that the mental connection between pattern names and implementations is never strengthened.

So when I hear someone name a pattern, I imagine a 10 year old meticulously writing down individual rewrites for their math homework. Sure it's helpful while you're learning, and the underlying concepts are useful to know, but please don't waste my time on minutiae. Do five rewrites in one go, I'll figure it out. Make a newtype without calling it a decorator, I beg you. I don't want to feel like I'm checking your homework.


### Naming

There's an argument that naming patterns is good because it allows to communicate complex concepts succinctly. Fine, let's try it. You say "factory", I write `function newFoo`. You say "adapter", I write `interface FooWrapper`. You say "prototype", I write `function clone`.

There's a huge mismatch between the pattern terminology and actual code. My brain is pretty small; it can fit concepts like functions and closures, but if I have to fit in other words that mean the same thing, like "command" and "action", it takes up real estate I'd rather spend on something more useful.

Of all things in communication, the worst you can do is use a random-sounding word that has a better-known synonym. You guys hate monads for the same reason. (That's not a double standard: I like monads, but I can never remember what a functor is, either.)

Occasionally, we even get funny situations like the prototype pattern having little in common with what the language calls "prototype". Looking at you, JavaScript. `Class.prototype` and the prototype pattern basically never overlap, even if they mean the same thing if you think hard about it.

If the name *does* match the ecosystem standard, please go ahead and use it. If you have a trait for iteration over a tree structure, you might as well call it `Visitor`. You shouldn't *need* traits like `Factory`, since first-class functions are prevalent in the 21st century, but if for whatever reason you do, feel free to call it that. But don't think of them as *patterns* -- think of them as idiomatic names.

Overall, there is value in simplicity. "`fn new` in a trait" is *longer*, but it can still be *easier* to interpret than "abstract factory" if there's no translation step.


### Transferrable?

Regardless of what wording you prefer, please do me a favor and tell me your intent directly. I want to know the problem you're solving, like "decoupling X from Y". I don't want you to tell me the pattern you chose to solve it, since that's typically self-evident.

It's also entirely language-dependent. Patterns don't really form a shared vocabulary in this manner, unlike goals.

For example, lazy initialization does not just affect the implementation when you switch a functional language -- it stops making any sense if the language has no mutability, since that means there's no initialization as a concept at all. Lazy *calculations* might exist, sure, but that's not the same thing.

Singletons are completely unnecessary if the ecosystem is used to global variables. Patterns based on subclassing cannot be used in languages preferring composition over inheritance. OOP patterns look silly in non-OOP languages like Rust, and many Rust patterns, like [branded lifetimes](https://arhan.sh/blog/the-generativity-pattern-in-rust/), refer to concepts that can't even be fathomed in other languages. Even among OOP languages, the differences are so vast that patterns like delegates exist exclusively to bridge approaches between languages.


### My head hurts

The Wikipedia page for [abstract factory](https://en.wikipedia.org/wiki/Abstract_factory_pattern) opens with this intro:

> The abstract factory pattern in software engineering is a design pattern that provides a way to create families of related objects without imposing their concrete classes, by encapsulating a group of individual factories that have a common theme without specifying their concrete classes. According to this pattern, a client software component creates a concrete implementation of the abstract factory and then uses the generic interface of the factory to create the concrete objects that are part of the family.

To understand this paragraph, I have to focus on individual sub-sentences *really* hard. Last time it felt like this was when I was reading law. My head hurts when I try to parse this text despite me using abstract factories all the time. That's not funny.

- "create families of related objects" -- so constructors, e.g. `fn new() -> Foo`.
- "without imposing their concrete classes" -- so constructors returning type-erased values, e.g. `fn new() -> Box<dyn FooTrait>`.
- "by encapsulating a group of individual factories that have a common theme" -- so named constructors, e.g. `fn new_foo() -> Box<dyn FooTrait>`.
- "creates a concrete implementation of the abstract factory" -- this doesn't tell me if the methods take `self` or not. Perhaps it was written before generics were a thing, so the only way to implement abstract types was through virtual methods.
- "uses the generic interface of the factory" -- so the factory itself is behind a trait.
- "to create the concrete objects that are part of the family" -- this might mean that the return types should be considered part of the factory trait, and we're actually talking `fn new_foo() -> impl FooTrait` or, alternatively, `type Foo: FooTrait; fn new_foo() -> Self::Foo;`.

Now that I can see the signature, I understand that it's incredibly obvious. And that makes it *useless*, since I would've converged on the same approach by straightforward iterative design.

So why does the article have 10 pages of text, a UML diagram, a dense paragraph of overview, and a long-winded implementation example? Isn't it ironic that an article about patterns is overengineered? Was there *really* no way to write this better? If I wanted to go mad thinking abstractly about trivial things, I'd study category theory.


### Principles

You likely know a good amount of people in your life who have memorized math "rules" without understanding their purpose or the underlying reasons. This leads either to overestimating the importance of the rules (forming cargo cult-like behavior), or underestimating it (e.g. [not understanding how units work](https://www.youtube.com/watch?v=nUpZg-Ua5ao); some of you might [fall into the same trap](https://news.ycombinator.com/item?id=45065425)). It's likely they believe that good mathematicians multiply big numbers in their heads, or something. *You* might understand that trivia like PEMDAS doesn't deserve focus, but beginners don't.

Good architecture is achieved by doing the necessary minimum (with requirements including maintainability and extensibility!) by composing the tools your programming language provides. The result might match an existing pattern, but that doesn't mean that you need to choose from the list of patterns initially.

When you tell beginners about patterns, they start following them religiously, and you get overengineered Java spaghetti. And that's fine, beginners tend to do that. But I think the balance can be improved by teaching patterns as mere *examples* of real-world applications of underlying principles, like SOLID, without explicitly calling them out as patterns to *follow*.

Also, many patterns are easy to invent from scratch. Say I'm writing a JavaScript library for, I don't know, handling HTML, so I use `document.createElement` everywhere. Then I realize that I might want my code to work on the server side as well. I don't want to hard-code the choice of using `document` from the browser vs from a shim library, and instead I want to let the user make the choice (maybe they know a better polyfill than me! who knows). So I decide to take `document` as an argument since, what do you know, I'm already calling a method on it, so this is a cheap modification. `document` is now an abstract factory.

Of course, it might take a while to converge on a good implementation, but it's probably a good idea to at least consider such an approach.


### Conclusion

Patterns have certainly mattered historically. Closures, interfaces, and first-class functions haven't always existed in mainstream languages. Many patterns were non-obvious back then, and so it's not a surprise that they emerged. But even Java itself, the language so terrible at fostering good architecture, it has become a joke, has had all of those feature for at least 10 years. So can we abolish "command" and "strategy" now, pretty please?

It's certainly useful to know about the commonly utilized patterns, but experience with real code and a little agility will quickly get you up to speed regardless of how much time you've invested in studying patterns in theory, as long as you know the basic concepts. They're good as a temporary mnemonic for newbies, not as must-know terminology. Walk me through a use case or two and then never mention the pattern by name again, I beg you.
