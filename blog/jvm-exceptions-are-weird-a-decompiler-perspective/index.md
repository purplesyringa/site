---
title: "JVM exceptions are weird: a decompiler perspective"
time: November 2, 2025
intro: |
    Some time ago, I played around with decompiling Java class files in a more efficient manner than traditional solutions like [Vineflower](https://github.com/Vineflower/vineflower) allow. Eventually, I wrote [an article](../recovering-control-flow-structures-without-cfgs/) on my approach to decompiling control flow, which was a great performance boost for my prototype.

    At the time, I believed that this method can be straightforwardly extended to handling exceptional control flow, i.e. decompiling `try`..`catch` blocks. In retrospect, I should've known it wouldn't be so easy. It turns out that there are many edge cases, ranging from strange `javac` behavior to consequences of the JVM design and the class file format, that significantly complicate this. In this post, I'll cover these details, why simple solutions don't work, and what approach I've eventually settled on.
discussion:
  - https://news.ycombinator.com/item?id=45808899
  - https://www.reddit.com/r/programming/comments/1oy5r4o/jvm_exceptions_are_weird_a_decompiler_perspective/
  - https://lobste.rs/s/ooxamp/jvm_exceptions_are_weird_decompiler
translation:
    Russian: https://habr.com/ru/articles/965922/
tikzThemes:
  light: |
    \colorlet{Arrow}{red!80}
    \colorlet{Text}{black}
  dark: |
    \definecolor{Arrow}{rgb}{.9,.3,.5}
    \colorlet{Text}{white}
---

Some time ago, I played around with decompiling Java class files in a more efficient manner than traditional solutions like [Vineflower](https://github.com/Vineflower/vineflower) allow. Eventually, I wrote [an article](../recovering-control-flow-structures-without-cfgs/) on my approach to decompiling control flow, which was a great performance boost for my prototype.

At the time, I believed that this method can be straightforwardly extended to handling exceptional control flow, i.e. decompiling `try`..`catch` blocks. In retrospect, I should've known it wouldn't be so easy. It turns out that there are many edge cases, ranging from strange `javac` behavior to consequences of the JVM design and the class file format, that significantly complicate this. In this post, I'll cover these details, why simple solutions don't work, and what approach I've eventually settled on.


### JVM basics

JVM is a stack-based language. The majority of instructions interact exclusively with the stack, e.g. `iadd` pop two integers off the top of the stack and pushes their sum back. A small handful of instructions, like `if_icmpeq`, transfer control to a given address based on whether a primitive condition holds (e.g. `value1 == value2` in this case). That is sufficient to implement `if`, `while`, and many other explicit control flow constructs.

Exceptional control flow, however, is implicit, so it cannot be handled the same way. A single `try` block should catch all exceptions arising in its region, be that exceptions forwarded from a method call in `invokevirtual`, division by zero in `idiv`, or null pointer dereference in `getfield`. Such relations cannot be efficiently encoded in the bytecode itself, so they are stored separately in the *exception table*.

Each entry of the exception table specifies which region of instructions is associated with which exception handler. If an exception is raised within such a region, the stack is cleared, the exception object is pushed onto the stack, and control is transferred to the first instruction of the handler.

For example, here's what the bytecode and the exception table look like for a simple method containing one `try`..`catch` block (produced with `javap -c`):

```java
static void method() {
    try {
        System.out.println("Hello, world!");
    } catch (Exception ex) {
        System.out.println("Oops, an error happened");
    }
}
```

```java
Code:
   // The next 3 lines print "Hello, world!".
   0: getstatic     #7                  // Field java/lang/System.out:Ljava/io/PrintStream;
   3: ldc           #13                 // String Hello, world!
   5: invokevirtual #15                 // Method java/io/PrintStream.println:(Ljava/lang/String;)V
   // After printing "Hello, world!" successfully, jump to the end of the function.
   8: goto          20
   // The exception handler starts here. `ex` is initially pushed onto the stack, so this
   // instruction saves it to a local variable.
  11: astore_0
   // The next 3 lines print "Oops, an error happened".
  12: getstatic     #7                  // Field java/lang/System.out:Ljava/io/PrintStream;
  15: ldc           #23                 // String Oops, an error happened
  17: invokevirtual #15                 // Method java/io/PrintStream.println:(Ljava/lang/String;)V
   // Return from the function.
  20: return
Exception table:
   from    to  target type
   // Exceptions arising during instructions at addresses 0 (inclusive) to 8 (exclusive) are handled
   // by the handler at address 11.
       0     8    11   Class java/lang/Exception
```

If there are multiple rows in the exception table, the first matching one is used. For example, if there are nested `try` blocks, the inner `try` block will be listed first, followed by the outer one.


### Nesting

Note that the exception table is just a list of regions. JVM imposes no requirements on the nesting structure of those regions: for example, it's possible for two ranges to intersect without one being nested in the other, and it's possible for `target` to be located before `from` or even inside the `from`..`to` range.

As we'll soon see, this is not a hypothetical, and real-world class files do often violate these "obvious" assumptions. This makes the problem important to handle well not only if you're building an unconditionally correct decompiler, like I'm trying to do, but any decompiler.


### Finally

Before we tackle this, we need to discuss how javac handles `try`..`finally` blocks. The body of `finally` should be executed regardless of whether an exception is thrown, but where the control is transferred after the end of `finally` depends on the presence of an exception:

```tikz
% alt A graph with two visible paths. The first path is "end of try" to "finally" to "after try". The second path is "exception handler" to "finally" to "rethrow". The same node is used for "finally" in both paths, resulting in a star-like topology.
\draw[very thick,->,Arrow] (0,1) node[left,Text] {\large end of try} -- (.5,1) .. controls (1,1) and (2,.2) .. (2.5,.2) -- (3,.2);
\draw[very thick,->,Arrow] (0,-1) node[left,Text] {\large exception handler} -- (.5,-1) .. controls (1,-1) and (2,-.2) .. (2.5,-.2) -- (3,-.2);
\node[Text] at (3.7,0) {\large finally};
\draw[very thick,<-,Arrow] (7.4,1) node[right,Text] {\large after try} -- (6.9,1) .. controls (6.4,1) and (5.4,.2) .. (4.9,.2) -- (4.4,.2);
\draw[very thick,<-,Arrow] (7.4,-1) node[right,Text] {\large rethrow} -- (6.9,-1) .. controls (6.4,-1) and (5.4,-.2) .. (4.9,-.2) -- (4.4,-.2);
```

It's not clear how the `finally` block should know where to transfer control next. One option is to store the possible exception to rethrow in a hidden variable and treat `null` as a signal that the `try` block completed without an exception, but that's not enough. A `try` block can have exit points aside from fallthrough: `continue`, `break`, and even `return` can be exit points, each requiring a different post-`finally` target. Handling this properly would require a jump table, which is likely to be slow as-is, not to mention confusing to JIT compilers and static analyzers, including the one in JVM validating that uninitialized variables are not accessed.

Instead, `javac` does something simultaneously cursed and genius: it duplicates the `finally` body on each exit path. Let's consider the following snippet:

```java
static void method() {
    try {
        try_body();
    } catch (Exception ex) {
        throw ex;
    } finally {
        finally_body();
    }
}
```

```java
Code:
   0: invokestatic  #7                  // Method try_body:()V
   3: invokestatic  #12                 // Method finally_body:()V
   6: goto          18
   9: astore_0
  10: aload_0
  11: athrow
  12: astore_1
  13: invokestatic  #12                 // Method finally_body:()V
  16: aload_1
  17: athrow
  18: return
Exception table:
   from    to  target type
       0     3     9   Class java/lang/Exception
       0     3    12   any
       9    13    12   any
```

First, `javac` recognizes that the `try` body can fallthrough, so it adds a call to `finally_body` right after the `try` body, followed to a jump to `return`. The `catch` body cannot fallthrough, so `finally_body` is not inserted after `11: athrow`.

Secondly, `javac` recognizes that the `try` body can throw, so it wraps it in a catch-all handler (`0 3 12 any` in the table) that saves the thrown exception, calls `finally_body`, and then rethrows the saved exception. Similarly, the `catch` body can throw, so it's also wrapped in a catch-all handler (`9 13 12 any` in the table).

For whatever reason, the region of this last catch-all handler additionally covers the first instruction of the handler itself. I've narrowed it down to a [questionable line](https://github.com/openjdk/jdk/blob/b06459d3a83c13c0fbc7a0a7698435f17265982e/src/jdk.compiler/share/classes/com/sun/tools/javac/jvm/Gen.java#L1615) in `javac` code, but it's been there for so long I doubt anyone wants to touch it. Even if it's fixed at some point, old class files will still suffer from this problem, so it's not like we can hope to forget about it.


### Throwing instructions

Now, it might seem that the `astore_1` instruction can't throw, so this should be easy to fix during parsing -- just decrease `to` to `target` if no instructions in range `target`..`to` can throw. But this decision has wider-ranging implications than it seems.

For one thing, *any JVM instruction can throw*. [The JVM specification](https://docs.oracle.com/javase/specs/jvms/se25/html/jvms-6.html#jvms-6.3) is very clear about this: `VirtualMachineError` "[...] may be thrown at any time during the operation of the Java Virtual Machine". `VirtualMachineError` is a superclass of such bangers as `OutOfMemoryError` and `StackOverflowError`, and I don't think it's hard to imagine a JVM interpreter that throws `StackOverflowError` when a JVM-internal function runs out of stack, or `OutOfMemoryError` if any ad-hoc allocation fails. Even `astore_1` can realistically throw if the locals array is allocated on demand. At least we don't have to deal with `Thread.stop` throwing arbitrary exceptions at arbitrary points since Java 20.

But a false positive (i.e. catching an exception when it shouldn't be caught) is just one part of the problem. A false negative can also occur under certain conditions. Consider the following:

```java
static int method(boolean condition) {
    try {
        if (condition) {
            return 1;
        }
    } finally {
        finally_body();
    }
    return 2;
}
```

The main goal here is to create a `return` statement within a `try`..`finally` block. While the `if (condition)` part and the initialization of `1` will be covered by the `try` region, the `return` itself has to be preceded by a call to `finally_body`, which should be located outside `try`. So where does the `return` instruction itself go? It turns out that `javac` generates it outside the `try` block:

```java expansible
Code:
   // `if` stuff.
   0: iload_0
   1: ifeq          11
   // Store `1` for a later `return`.
   4: iconst_1
   5: istore_1
   // `finally` body.
   6: invokestatic  #7                  // Method finally_body:()V
   // Load the return value and return.
   9: iload_1
  10: ireturn
   // Fallthrough target, i.e. end of `try` body.
   // `finally` body.
  11: invokestatic  #7                  // Method finally_body:()V
   // Jump to `return 2`.
  14: goto          23
   // Exception handler. Save the exception, run `finally` body, rethrow the exception.
  17: astore_2
  18: invokestatic  #7                  // Method finally_body:()V
  21: aload_2
  22: athrow
   // `return 2`.
  23: iconst_2
  24: ireturn
Exception table:
   from    to  target type
       0     6    17   any
```

From the source code, we'd expect exceptions arising during `return` to be caught by the `try` block, yet they aren't. But *surely* `return` can only throw `VirtualMachineError`s that we can turn a blind eye to? Not quite: [according to the JVM specification](https://docs.oracle.com/javase/specs/jvms/se25/html/jvms-6.html#jvms-6.5.return), `return` can also throw ` IllegalMonitorStateException` if, for example, some monitors that were acquired during the execution of the function haven't been released by the time the function returns. `javac` generates code that never exhibits this behavior, and since monitors are incompatible with coroutines, it's likely that other frontends won't use this feature as much. But hand-written Java bytecode is not guaranteed to be valid in this regard, so a decompiler still has to take this design oddity into account.

My solution to this is unimpressive. If monitors can be statically verified to be correct, `return` cannot throw, and the worst thing that can happen is that OOM or stack overflow during `astore` is erroneously caught/not caught, which cannot happen on HotSpot or any other reasonably efficient JVM implementation. This means that we can assume that, for all intents and purposes, most instructions can't throw. On the other hand, if the well-formedness of monitors cannot be verified, the decompiler cannot produce Java code anyway, so how exactly `javac` interprets the resulting pseudocode doesn't matter.


### Reachability

Before we discuss other nuanced stuff, I want to cover a simpler topic.

JVM is weird in that it has two type checkers. If the bytecode compiler provides a table (called `StackMapTable`) containing information about which type each stack element has at each point, JVM only needs to verify that all operations are correctly typed. If such a table is not provided, JVM needs to infer types instead. Since type inference takes a non-trivial amount of time, `StackMapTable` is required to be present in all classfiles since Java 6. However, modern JVMs are still capable of loading old classfiles, so we'll be stuck with two type checkers for a while.

There is a major difference between the two type checkers: while type checking by verification (i.e. using `StackMapTable`) validates every instruction in the bytecode, type checking by inference necessarily verifies only every *reachable* instruction, since it cannot know the stack layout of unreachable instructions. This means that invalid combinations of bytecode instructions, like `iconst_1; ladd`, can be present in old classfiles, but not new ones.

How is this relevant to exception handling? Since rows in the exception table have two parameters `to` and `target` that typically coincide for Java code (`try` ends at `}`, immediately followed by `catch (...) {`), but are frequently distinct in bytecode (e.g. due to a `goto` inbetween), you might foolishly try to expand the `try` range to the right to `target` if no instruction in range `to`..`target` can throw. This expansion has an odd side-effect: if no instruction in range `from`..`to` has previously been reachable, but `to`..`target` is reachable, then you've just made the exception handler reachable when it was unreachable in bytecode. And in old classfiles, this might make valid code seem incorrectly typed. That's bad!

Of course, you might not be interested in handling old classfiles, but it's about time to discuss why this band-aid doesn't have a chance to work regardless.


### Ranges

It might seem intuitive that one `try`..`catch` block should be compiled to one row in the exception table, but that's not the case. Since `finally` blocks need to be duplicated at each exit point, and you obviously wouldn't want exceptions inside `finally` to be caught, some subregions need to be excluded from exception handling. For example:

```java
try {
    if (condition) {
        return 1;
    } else {
        return 2;
    }
} finally {
    finally_body();
}
```

```java expansible
Code:
   0: iload_0
   1: ifeq          11
   4: iconst_1
   5: istore_1
   // `return 1` exit point.
   6: invokestatic  #7                  // Method finally_body:()V
   9: iload_1
  10: ireturn
  11: iconst_2
  12: istore_1
   // `return 2` exit point.
  13: invokestatic  #7                  // Method finally_body:()V
  16: iload_1
  17: ireturn
   // Exceptional exit point.
  18: astore_2
  19: invokestatic  #7                  // Method finally_body:()V
  22: aload_2
  23: athrow
Exception table:
   from    to  target type
       0     6    18   any
      11    13    18   any
```

Even if the `finally` block is absent, `javac` merely treats it as empty, still excluding `return` and `goto` statements from the `try` ranges:

```java
try {
    if (condition) {
        return 1;
    } else {
        return 2;
    }
} catch (Exception ex) {
    return 3;
}
```

```java
Code:
   0: iload_0
   1: ifeq          6
   4: iconst_1
   5: ireturn // exceptions during this `return` are not handled.
   6: iconst_2
   7: ireturn
   8: astore_1
   9: iconst_3
  10: ireturn
Exception table:
   from    to  target type
       0     5     8   Class java/lang/Exception
       6     7     8   Class java/lang/Exception
```

(This also means that the code between `to` and `target` is not always just a `goto` or a `return` -- it may also include the contents of the `finally` block, which are not guaranteed to be non-throwing.)

Perhaps the most confusing implication is that while exception handling ranges can cross control flow constructs (e.g. it's possible for `from` to be located outside an `if` and for `to` to be inside an `if`), ranges of *exemption* from EH correspond to single positions in source code, and thus cannot cross control flow. So in the eyes of a decompiler, the code above should be parsed like this:

```java
try #1 {
    if (condition) {
        int tmp = 1;
        exempt #1 {
            return tmp;
        }
    } else {
        int tmp = 2;
        exempt #1 {
            return tmp;
        }
    }
} catch (Exception ex) {
}
return 3;
```

...and not by creating one `try` block for each row. The decompiler can then verify that `exempt` blocks are present on each exit path of a `try` block and have matching contents, and simplify the code to a `try`..`finally`. The details are fuzzy and I haven't figured out everything myself yet, but I believe it can be implemented in a single pass.


### Outro

One minor issue I haven't mentioned yet is how to represent exception handlers in the IR. When a handler is entered, the stack is cleared and the exception object is pushed onto the stack. My approach to decompilation assumes the existence of a linear order comprised of individual instructions from the bytecode -- so where would the stack store go? It can't just be inserted at the entry to the handler, since the first instruction of the exception handler may also be reachable by a `goto` or any other explicit control flow mechanism, not just with `try`..`catch`. There is no other possibility but to treat this stack store as special in some way and introduce it into the IR at a later point, i.e. when a syntactic block is created for `try`..`catch`.

I wanted this post to be about Java gimmicks rather than my decompiler in particular, so that's it for now. If I've missed anything important or you wanted to share an idea, feel free to message me.
