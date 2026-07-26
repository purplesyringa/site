---
title: Impressions of Teal
time: July 26, 2026
intro: |
    About two months ago, I found [the Teal programming language](https://teal-language.org/), which describes itself as a statically-typed dialect of Lua. It transpiles to Lua and runs typecheck on the developer's machine, so it fills roughly the same niche as TypeScript. It's used by quite a few projects, including [LuaRocks](https://github.com/luarocks/luarocks) and [inspect](https://github.com/kikito/inspect.lua).

    For better or worse, I *am* trying to write Lua without feeling like it's year 2010, and Teal promised to be suitable for this purpose.

    This post is a retrospective on my experience: what went right and what, in my opinion, could be improved. Mostly the latter.

    Disclaimer: while I stumbled upon multiple issues, I deeply respect everyone who works on this project, and I consider it a valuable tool. No offence intended.
---

About two months ago, I found [the Teal programming language](https://teal-language.org/), which describes itself as a statically-typed dialect of Lua. It transpiles to Lua and runs typecheck on the developer's machine, so it fills roughly the same niche as TypeScript. It's used by quite a few projects, including [LuaRocks](https://github.com/luarocks/luarocks) and [inspect](https://github.com/kikito/inspect.lua).

For better or worse, I *am* trying to write Lua without feeling like it's year 2010, and Teal promised to be suitable for this purpose.

This post is a retrospective on my experience: what went right and what, in my opinion, could be improved. Mostly the latter.

Disclaimer: while I stumbled upon multiple issues, I deeply respect everyone who works on this project, and I consider it a valuable tool. No offence intended.


### Annotations

I think the main selling point of Teal is just how simple its annotations are. Look at the example on the website:

```lua
local function add(a: number, b: number): number
    return a + b
end

local s: number = add(1, 2)
print(s)
```

It's concise and non-intrusive, like TypeScript. Compare it to [LuaLS annotations](https://luals.github.io/wiki/annotations/), which offers a competitive Lua typing project:

```lua
---@param a number
---@param b number
---@return number
local function add(a, b)
    return a + b
end

---@type number
local s = add(1, 2)
print(s)
```

Nasty!


### Soundness

This is where its obvious advantages end, though. As we'll find out in a bit, the simplicity is quite fragile.

I'm a Rust-pilled developer, so I expect some degree of soundness from type systems. "Soundness" means that if the type checker says the program is fine, you shouldn't encounter type errors in runtime. Sounds easy?

But soundness is fundamentally difficult to achieve in dynamic languages due to [variance](https://en.wikipedia.org/wiki/Type_variance). The issue arises when mutability and subtypes collide. Here's an example:

- Suppose that some function `f` takes a parameter `x: number[]`.
- Outside the function, create a list `a: integer[]`. You should seemingly be able to pass it as an argument to `f`, since every integer is a number: `f(a)`.
- From the perspective of `f`, `x` is `number[]`, so it can append `1.5` to the list.
- But this causes the variable `a: integer[]` to contain a non-integer value `1.5`!

The only real way to solve this is with `readonly` annotations, like in TypeScript. Teal doesn't offer anything of similar functionality. It also doesn't have `nil` safety, not even opt-in, so e.g. all table accesses return `T`, not `T | nil`.

I won't hold this specific issue against Teal, as even TypeScript [supports bivariance by default](https://www.typescriptlang.org/tsconfig/#strictFunctionTypes) and doesn't have `nil` safety for arrays, but it does hint that Teal considers soundness -- which I'd call the main point of typing -- an afterthought.


### Unions

Teal has very rudimentary support for discriminated unions (or sum types, or `enum`s with fields). For comparison, here's what that looks like in Rust:

```rust
enum Enum {
    Variant1 {
        a: String,
    },
    Variant2 {
        a: i32.
        b: i32,
    },
}
```

...and in TypeScript, which simulates them with union types:

```typescript
interface Variant1 {
    type: "variant1"
    a: string
}

interface Variant2 {
    type: "variant2"
    a: number
    b: number
}

type Enum = Variant1 | Variant2
```

Note that we have to explicitly add a field of a [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types) to disambiguate types in runtime. Teal supports neither first-class sum types, nor literal types, so you have to tell it how to disambiguate variants explicitly -- otherwise you can't make a type union:

```lua
local record Variant1
where self.type == "variant1"
    type: string
    a: string
end

local record Variant2
where self.type == "variant2"
    type: string
    a: number
    b: number
end

local type Enum = Variant1 | Variant2
```

`where` denotes a [type predicate](https://www.typescriptlang.org/docs/handbook/2/narrowing.html#using-type-predicates). It can contain arbitrary code, and the compiler doesn't validate that the definition actually makes sense. So you have to write the equivalent of `unsafe` code every time you want to define a discriminated union!

Furthermore, Teal can't validate that newly created instances of `Variant*` have the right `type`s, because `type` is just a `string`. To enforce that, you have to use single-variant `enum`s to simulate some aspects of literal types:

```lua
local enum Variant1Type "variant1" end
local record Variant1
where self.type == "variant1" -- still required, but at least type-checked
    type: Variant1Type
    a: string
end

...

-- Error: string "variatn1" is not a variant of Variant1Type
local x: Variant1 = {
    type = "variatn1", -- hypothetical misspelling
    a = "b",
}
```

Suffice to say, this is messier than it should be. But all the underlying mechanisms are already there, so hopefully Teal supports sum types as a first-class citizen one day.


### is

Teal adds two operators to the language: `as` and `is`. `as` is an unchecked typecast that gets optimized away in runtime, and `is` checks if a variable has a given runtime type.

The main use for `is` is pattern-matching union types:

```lua
local function f(x: Enum)
    -- Doesn't work, says `x` has type `Variant1 | Variant2` even inside `if` branches.
    if x.type == "variant1" then
        local y: Variant1 = x
    else
        local y: Variant2 = x
    end

    -- Works correctly, substitutes the predicate in `where`.
    if x is Variant1 then
        local y: Variant1 = x
    else
        local y: Variant2 = x
    end
end
```

Its semantics don't seem to be thought-out well, though. `is T` resolves the type to `T` in any context, not just for union types:

```lua
local function f(x: any)
    if x is Variant1 then
        local y: Variant1 = x
    end
end
```

But in this context, `x` is not even guaranteed to be a table, so the access `x.type == "variant1"` substituted from `where` may fail. While I would expect the *result* of `where` to be trusted blindly, I wouldn't expect the calculation itself to be trusted, since it's type-checked -- but, apparently, not strictly enough.

Furthermore, in this context, `is` works even on types without `where` -- those get an implicit `where type(self) == "table"`, meaning that `is` itself is effectively unsound:

```lua
local record Record
end

local function f(x: any)
    if x is Record then
        local y: Record = x
        -- Assumes `y` is `Record` even though the runtime check only verifies it's a table.
    end
end
```

In addition, `x is any` lowers to `type(x) == "table"` (which is strange because `any` is supposed to represent any value), and `x is nil` lowers to `x == nil`, despite the fact that comparison operators can be overloaded.

While each issue can be solved individually (and I reported some of them), their multitude makes me question the design decisions that allowed them to manifest.


### Metatables

Lua implements OOP with [prototypes](https://en.wikipedia.org/wiki/Prototype-based_programming). To create an object, you need to do two things:

```lua
-- 1. Create a data-only table with the right fields.
local obj = {
    a = 1,
    b = 2,
    ...
}

-- 2. Set its metatable.
setmetatable(obj, {
    __index = Type,
})
```

The metatable controls many properties of the object, including how attribute accesses to absent keys are resolved. This allows us to share method definitions across object instances:

```lua
-- Define methods once.
local Person = {
    say_hello = function(self) print("Hello", self.name) end,
}

-- Create multiple objects whose attribute accesses fall back to `Person`.
local alice = { name = "Alice" }
setmetatable(alice, { __index = Person })
local bob = { name = "Bob" }
setmetatable(bob, { __index = Person })

-- `alice` doesn't have a `say_hello` field, but `alice` inherits from `Person`, which does, so
-- that's what `alice.say_hello` resolves to.
alice.say_hello(alice)
-- A colon instead of a dot is syntax sugar for passing the receiver as the first argument.
bob:say_hello()
```

The states of the object after `setmetatable` and before `setmetatable` are fundamentally different: one enables method calls, overloaded operators, and static properties, while the other one doesn't. But Teal doesn't see the difference:

```lua
local record Person
    name: string
end

function Person:say_hello()
    print("Hello", self.name)
end

local alice: Person = { name = "alice" }
-- Oops, forgot `setmetatable`!
alice:say_hello() -- compiles just fine, errors in runtime
```

This is quite a surprising omission. While `setmetatable(obj, metatable)` mutates `obj`, it also returns `obj`, so you could plausibly imagine Teal introducing a signature like

```lua
setmetatable: function<T>(onlyfields<T>, metatable<T>): T
```

...and inferring the type of `{ name = "alice" }` as `onlyfields<Person>`, not `Person`. In&nbsp;fact, Teal already supports the `metatable<T>` type, so this feels like an honest omission -- but I can't help but get surprised that such a basic thing got forgotten.


### Effects

Lua offers coroutines, which can be used to implement iterators and async functions. A coroutine is created by `coroutine.create(closure)`, and any call `coroutine.yield(...)` nested arbitrarily deep in `closure` sends a value upward to the coroutine owner (e.g. a `for` loop or an async runtime). The owner can then resume the coroutine from after the yield point.

As every other time someone "solves" [the coloring problem](https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/), it turns out that static typing brings it back, but worse. Let's look at a concrete example:

```lua
local function f()
    coroutine.yield(1)
end

local function g()
    f()
end

local function h()
    local coro = coroutine.create(g)
    local ok, value = coroutine.resume(coro)
    assert(ok and value == 1)
end
```

`f` yields a `number`, but that fact is not represented in its signature: `f` takes no parameters and returns no values, so there's nothing to annotate. By the time we get to `h`, we've already passed through `g`, which has forgotten that it yields anything at all (the body of `g` doesn't mention `coroutine` once, and `f`'s signature doesn't mention it yields either), so we've lost all type information. Then how do we type `value` in `h`?

We need an annotation that survives nested calls. That's called an *effect*. We could imagine an effect `yields <type>` and a requirement that every call to a function with `yields <type>` must itself occur in a function annotated with `yields <type>`:

```lua
local function f() yields number
    coroutine.yield(1) -- requires `f` to have `yields number`
end

local function g() yields number
    f() -- requires `g` to have `yields number`
end

local function h()
    local coro = coroutine.create(g) -- takes `function() yields T` for a generic `T`
    local ok, value = coroutine.resume(coro) -- returns `boolean, T`
    assert(ok and value == 1)
end
```

Teal doesn't have an effect system: it just marks `coroutine.resume` and `coroutine.yield` as taking/returning `any` values (TypeScript's `unknown`) and requires the user to insert type casts. That's quite unfortunate, since even this rudimentary form of effects would make coroutine-based iterators sound.

This also means that `coroutine.wrap` has to be annotated as `function<F>(F): F`, unsoundly ignoring yielded values. `function(function(): R yields T): (function(): R | T)` would be a better signature if Teal supported effects (still not quite correct due to arguments, but that requires typestate).

While coroutines are rare in computational Lua, they are used extensively in GUIs and server logic. I wouldn't call supporting it a *must*, but event-driven software is an important niche, and it would be great to see this limitation fixed.


### Numbers

Teal has a strange relationship with numbers, though it's arguably Lua's fault.

Modern Lua has two numeric types: `integer` and `float`. They behave completely differently, e.g. `integer`s wrap-around on overflow and `float`s work like floats do. Teal exposes `integer` as a subtype of `number`, but doesn't offer `float`, meaning that this function cannot be annotated safely:

```lua
local function f(a: number, b: number)
    if a > 0 and b > 0 then
        -- Valid for `float`s, invalid for `integer`s.
        assert(a + b > 0)
    end
end
```

It quickly becomes clear that Teal inconsistently treats `integer` as meaning "any `number` that is whole" or "`integer` according to Lua rules" in different places:

- The type system seemingly follows the former interpretation.
- `string.char` takes an `integer` value, and `math.floor` and `math.modf` return `integer` values in the first sense.
- `next` takes an `integer` and `tonumber` returns an `integer` in the second sense.
- `local x: integer = 1.0` is a type error, as per Lua semantics.
- `x is integer` is lowered to `math.type(x) == "integer"`, which uses Lua semantics.
- ...except on older Lua versions, where the polyfill follows the first interpretation.


### stdlib

On a positive note, there is a surprisingly small amount of issues in the standard library annotations. We've already seen issues with `coroutine` methods, `setmetatable`, and `integer`s, but the remaining problems are very minor:

- `math.max` has an overload `function(any...): any`, but Lua doesn't allow comparing e.g. strings to numbers. I don't know why it's needed.

- `utf8` methods take `number`s instead of `integer`s for some reason.

- `debug.setlocal` takes a parameter of type `any`, i.e. top type, which means "you can pass any value here without type checking", while what it actually tries to say is "no type is provably correct", i.e. the bottom type, which Teal lacks.

- Metamethods on `metatable<T>` are declared with generics instead of existentials, but the compiler seems to special-case them to work correctly.


### Modules

Using a type defined in a different file requires a manual import. The design is similar to TypeScript, but there are subtle differences between JavaScript and Lua that make this choice questionable.

In TypeScript, you can export a named type and import it in one of two ways:

```typescript
// a.ts
export interface Test {
    field: string
}

// b.ts
import { Test } from "./a.ts"
// or
import type { Test } from "./a.ts"
```

`import` compiles to a runtime import and is useful for types with runtime presence, such as classes. `import type` is zero-cost and erased during transpilation.

Lua doesn't have named exports: each module returns only one value, so named returns are simulated by returning a table. But Teal's equivalent of `import type` doesn't recognize the idiomatic way to construct it!

```lua
-- a.tl
local interface Test end
return {
    Test = Test,
}

-- b.tl
-- Error: 'require' did not return a type, got record (Test: Test)
local type Test = require("a").Test
```

Instead, you have to define a temporary record and return *that*:

```lua
-- a.tl
local record A
    interface Test end
end

return A
```

`record`s are just tables in disguise, so you can still populate them with runtime values via `function A.f()` and `A.x = ...` -- but you can't use a table literal directly.

But the fact that `record`s can be `return`ed from modules means that some types imported with `local type` exist in runtime, which is completely unlike TypeScript:

```lua
-- b.tl
local type Test = require("a").Test
return Test

-- compiles to:
local Test = require("a").Test
return Test
```

This matters because the compiler and the runtime resolve modules differently: when importing external libraries, Teal will use the `.d.tl` type annotations, and Lua will use the untyped `.lua` code. If the `.d.tl` file contains helper type definitions that don't exist in `.lua`, you may get errors in runtime despite using `local type`, without a warning.

So `local type = require` is less powerful than `local = require`, but at the same time it's not guaranteed to be compile-time-only. Then why does it exist? Optimization aside, there seems to be just one reason: `interface`s don't have a runtime representation, so `local = require` fails for them, forcing the use of `local type = require`.

None of this screams "great design" to me. If materializing types is considered cheap, `interface`s with nested types should be materialized as well, so that `nil` accesses on reexports are avoided and `local type` becomes unnecessary. And if it's expensive, `local type` should never materialize automatically.


### Syntax

This section is a little subjective, but hopefully convincing.

Lua has a simple grammar: Lua code can be parsed pretty much without lookahead and is easy to generate. There are many compatible Lua parsers and runtimes.

Teal grammar looks similar to Lua, but it's much subtler, and the docs don't make that obvious. Here's one example.

`record`, `interface`, `global`, etc. are contextual keywords in Teal, meaning that they can also be used as variable names. This is necessary for compatibility with pre-existing Lua code. So a Teal parser needs to disambiguate between `local record` and `local record Name ... end`, but in general that's impossible:

```lua
-- Consider this snippet:
local record global interface X end end

-- Interpretation #1: a local named `record`, followed by an empty global interface named `X`, and
-- then the parent scope is closed.
    local record
    global interface X end
end

-- Interpretation #2: a local record named `global`, containing an empty interface `X`.
local record global
    interface X end
end

-- Ambiguous example -- which line closes the `do` scope?
do
local record global interface X end end
local record global interface X end end
```

The official compiler uses subtle heuristics, so writing a Teal parser requires copying them precisely. This is the reason why [my partial reimplementation of Teal](https://github.com/purplesyringa/orangetl) is so long.

A simpler difference is that Teal is whitespace-sensitive, unlike Lua. The same unannotated code, transpiled with the same Teal compiler, can behave differently depending on whether the file is named `test.tl` or `test.lua`:

```lua
f()
(g)()
```

- Teal interpretation: `f(); g()`
- Lua interpretation: `f()(g)()` (three chained/curried calls).

Teal assumes lines beginning with `(` start a new statement, Lua treats them as the continuation of a previous expression. Leading `(` is rare in Lua, so this seldom causes trouble, but in Teal it's common to see `(obj as T).f()`, so they had to adjust it. I wish they changed the syntax of `as` instead, perhaps to `cast<T>(obj).f()` or `obj.as<T>.f()`.

Yet this whitespace sensitivity is not applied in a different context where it would help. Both `function` and `function()` are valid types in Teal, but `function(` is always parsed greedily without consideration for whitespace:

```lua
local x: function 
(f)() -- syntax error, because this is parsed as `function(f)()`
```

Other than that, it's just wild how inconsistent the syntax is compared to Lua:

- As in Lua, a variable can't be used inside its own definition: in `local x = f(x)`, the second `x` refers to the previous value of `x`. Yet the name of a type is visible inside its definition.
- `enum`s are declared as `enum Name "a" "b" "c" end`, while Lua always separates values with commas or semicolons.
- `{T}` and `is {T}` are both valid in `record` definitions and indicate that the array part has type `T`. One is presented in the docs and the other is used in the compiler.
- The optional sign `?` can be specified both before and after the parameter name.
- Both `userdata` and `is userdata` compile, but only one has the right semantics.


### Focus

Given such glaring issues in the core parts of the language -- `tl`, I assume, initially stood for Typed Lua, and there are quite a few problems with both the "typed" part and the "Lua" part -- one would hope that the maintenance efforts were focused on strengthening the foundation.

In practice, though, Teal tries to do *so much more*, with varying results:

- It has *two* different macro systems:
  - Newer, syntax-based Rust-like macros.
  - Older macro expressions, which are pretty much just functions inlined in compile time. They were ostensibly introduced for type predicates, but their scope has increased since then, together with bugginess. Furthermore, the existence of `macroexp` methods makes code generation type-dependent.
- It has a compatibility mechanism for targeting every version from Lua 5.1 to Lua 5.4. It uses [compat53](https://github.com/lunarmodules/lua-compat-5.3) under the hood, but it *also* inlines a few definitions separately, and in some cases the two disagree.
- Teal does not follow upstream syntax changes -- it has some semantics that Lua 5.5 dropped, so there are now *forward*-compatibility features as well, and their syntax for `global` differs from Lua 5.5.

Teal seems to be more of an experimentation ground than a reliable tool -- it's a kitchen sink of ideas, some better than others. It's more of a general-purpose, all-around Lua preprocessor than a typed superset.

And that's fine! Despite quite a few problems and limitations, I learned a lot from studying Teal, and I'm glad that it exists. It's clearly useful at least to some people, and it removes a fair share of footguns. Maybe it's not a great typing language, but it's versatile enough that you can (mis)use it for some wild stuff.

But I wish I didn't have to learn this lesson the hard way. Teal should advertise its purpose and capabilities better -- in light of the above, the [website](https://teal-language.org/) and the [documentation](https://teal-language.org/book/latest/) are frankly misleading:

> Teal is a statically-typed dialect of Lua. It extends Lua with type annotations, allowing you to specify arrays, maps and records, as well as interfaces, union types and generics.

Yes, but: it also adds a ton of stuff unrelated to type annotations, and the listed type system features are underbaked.

> It aims to fill a niche similar to that of TypeScript in the JavaScript world, but adhering to Lua's spirit of minimalism, portability and embeddability. 

Yes, but: transpilation requiring type checking is not TypeScript-like, a language with two macro systems is not minimalistic, and the bolted-upon compatibility module doesn't enable portable *semantics*.

> Here is a quick taste of what Teal code looks like: 
>
> ```lua
> local function add(a: number, b: number): number
>    return a + b
> end
> 
> local s = add(1, 2)
> print(s)
> ```

Yes, but: realistic Teal code has type definitions and OOP patterns, which quickly get uglier than the example hints at.


### Conclusion

I think Teal is having a bit of an identity crisis, but it refuses to admit that. We would all be better off if it made a decision on what it's trying to grow into. That's difficult, and perhaps not a priority for development -- but I think it would do wonders for adoption and production readiness.
