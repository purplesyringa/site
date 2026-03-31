---
title: There is absolutely nothing wrong with Web
time: March 31, 2026
discussion: https://lobste.rs/s/h0ctyv/there_is_absolutely_nothing_wrong_with
intro: |
    This is a rant about how broken everything Web is based on is. You know, the usual. No offence intended towards framework developers, I'm glad this technology exists, but I'm sure you know this feeling. It gets too much sometimes.

    I've been meaning to improve this blog's technology for a while. It's held together by two [hacky scripts](https://github.com/purplesyringa/site/blob/9015c0d0d6087bf1f3e851b040970cee7da29518/blog/build.js) as opposed to a typical template engine, and that's very limiting.
---

This is a rant about how broken everything Web is based on is. You know, the usual. No offence intended towards framework developers, I'm glad this technology exists, but I'm sure you know this feeling. It gets too much sometimes.

I've been meaning to improve this blog's technology for a while. It's held together by two [hacky scripts](https://github.com/purplesyringa/site/blob/9015c0d0d6087bf1f3e851b040970cee7da29518/blog/build.js) as opposed to a typical template engine, and that's very limiting.


### Why not \___?

So, why isn't this a no-brainer? There are many static site generators, including classics like Hugo and Jekyll.

<aside-start-here />

The answer is in my design decisions. I really value the unique feel of this blog -- its bold color choice for the dark theme, a variable-width font for `inline code`, wide code snippets, headings on the left side, and sidenotes.

:::aside
Like this one. That's a sidenote.
:::

The last feature is by far the most complicated.


### Sidenotes

<aside-start-here />

Let's start with something simple &rightarrow;

:::aside
A sidenote with **formatting** and math: $2 + 2 = 5$.
:::

Here's what this snippet looks like in my Markdown source (almost):

```markdown
Let's start with something simple &rightarrow;

:::aside
A sidenote with **formatting** and math: $2 + 2 = 5$.
:::
```

Markdown disables all formatting inside HTML tags, so I can't just use `<aside>`. [markdown-it-container](https://www.npmjs.com/package/markdown-it-container) allows me to create a container that supports nested formatting, but "batteries-included" static site generators often don't support this.

Moving on, on desktop, sidenotes are shown *to the right* of the commented content. When there's little horizontal space (e.g. on mobile), they are shown *below* the content instead. This way, you always read the post in the intended order. So Markdown actually specifies both the beginning and the end of the commented part:

```markdown
<aside-start-here />

Let's start with something simple &rightarrow;

:::aside
A sidenote with **formatting** and math: $2 + 2 = 5$.
:::
```

`markdown-it-container` doesn't support self-closing annotations, so it has to be an HTML tag. Now, how should this be rendered in HTML?

```html
<div class="group">
    main content
    <aside>elaboration</aside>
</div>
```

<aside-start-here />

On desktop, I style the `<aside>` as `position: absolute; left: 100%; top: 0;` to align it to the top of the `<div>`.

:::aside
I used to place `<aside>` first and use [order](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/order) on mobile, but it doesn't account for screen readers, so I had to change it.
:::

Note that `<aside-start-here />` is rendered as a non-closed tag `<div class="group">`, and `:::aside` closes that `</div>`. This breaks renderers that assume Markdown AST nodes directly correspond to HTML AST nodes, which is pretty much every renderer.

So instead, I patch the produced HTML. Needless to say, that's not a common part of your average SSG pipeline, so you lose all benefits of powerful tools, like hot reload.


### Patches

There are other reasons to complicate HTML rendering and/or Markdown parsing:

- [Tables need to be horizontally scrollable](../you-might-want-to-use-panics-for-error-handling/#:~:text=One%20simple%20commonly%20used%20project) on mobile devices and narrow screens, which requires wrapping `<table>` in a container.
- I need a simple syntax for videos, both [local](../we-built-the-best-bad-apple-in-minecraft/#:~:text=These%20structures%20should%20be%20built) and [from YouTube](../optimization-lessons-from-a-minecraft-structure-locator/), i.e. `![]()` should emit `<video>` and `<iframe>` tags. This includes setting `aspect-ratio` for `<iframe>` and configuring autoplay and seizure warnings per-video.
- I need to draw [diagrams](../recovering-control-flow-structures-without-cfgs/#:~:text=There%E2%80%99s%20a%20non%2Dzero%2Dcost%20approach%20to%20this) with [tikz](https://tikz.net/) without running scripts by hand.
- I want [split views](../optimization-lessons-from-a-minecraft-structure-locator/#:~:text=After%20keeping%20my%20PC%20running%20for%20a%20couple%20days) for pictures, custom emojis <img class="emoji" src="/images/ferrisClueless.webp" title=":ferrisClueless:" />, and inlining for small SVGs.

Maybe it's asking for too much, but to me, that's basic functionality -- most of it is necessary either for accessibility or to get the point across without saying "sorry, my blog software doesn't support this, so I'll have to show it differently".


### Frameworks

Point is, I need to take out all the batteries from the battery-included tools, and the best way to do it is to drop the tools altogether.

On the other extreme are feature-complete Web frameworks like React and Dioxus. They are extensive enough to support everything I need.

<aside-start-here />

The devil is in the details. PHP and other old-timey tools don't support hot reload, which would normally be acceptable, but building [tikz](https://tikz.net/) to SVG is slow as hell and can make page rebuilds take up to 10 seconds. Besides, SSR-based tools can't run on GitHub Pages, and I value having a reliable hosting when I'm posted on HN.

:::aside
I *guess* I could start a local server and run `wget -r` during build, but come on.
:::

<aside-start-here />

In contrast, most front-end frameworks import a ton of JavaScript for hydration even with SSR -- so even though the site works without JS, the framework doubles the size of the page. This equally applies to modern Wasm-based frameworks. I could strip `<script>` tags, but that would prevent me from adding opt-in interactivity.

:::aside
If you have JS enabled, press <kbd>Ctrl+Enter</kbd> to see what I mean.
:::


### Silver bullet?

That's when I learnt about [Astro](https://astro.build/). On paper, it has everything I need:

- Static [JSX-like components](https://docs.astro.build/en/reference/astro-syntax/) supporting [scoped styles](https://docs.astro.build/en/guides/styling/).
- [SSG for production and hot reload for development](https://docs.astro.build/en/develop-and-build/) (SSR is available, but opt-in).
- Isolated [interactive islands](https://docs.astro.build/en/concepts/islands/) using React, Vue, or even plain JS.
- Generating pages [from on-disk Markdown files](https://docs.astro.build/en/guides/routing/).

At first, this made me really sad, because it meant I didn't have an excuse to develop my own framework. But as I started porting the site, cracks began to appear.

Since Astro is based on frontend technology, all code goes through [Vite](https://vite.dev/), even if it's server-side-only. Vite is a bundler, and it's responsible for packaging your project and its dependencies into JS files. More subtly, it makes directives like this work:

```javascript
import imageUrl from "./images/pic.png";
```

Behind the scenes, Vite intercepts the import, optimizes the picture, and makes it seem like `pic.png` is actually a JS file with the following contents:

```javascript
export default "<url to picture>";
```

It uses a similar mechanism to handle CSS imports and other cool stuff.


### Assets

I had naturally assumed that this mechanism would be extensible. My [kitchen sink](/sink/) page contains tons of icons, so I'd like to write something like:

```html
<Icon name="custom-cpp" />
```

I'm using [Nerd Fonts](https://www.nerdfonts.com/), which distributes a WOFF font. My plan was to have `<Icon>` extract the glyph from the script, create a virtual file for the resulting SVG, and use the mechanism that Vite uses for on-disk pictures or CSS modules to import it from JS.

Vite isn't the one that performs the build, though. It's based on [Rollup](https://rollupjs.org/), or alternatively [Rolldown](https://rolldown.rs/). (Note: we're already three levels deep.) Rollup docs denote [a single page](https://rollupjs.org/plugin-development/) to plugin development. Here's an example quote from the docs:

> If `external` is `true`, then absolute ids will be converted to relative ids based on the user's choice for the `makeAbsoluteExternalsRelative` option. This choice can be overridden by passing either `external: "relative"` to always convert an absolute id to a relative id or `external: "absolute"` to keep it as an absolute id. When returning an object, relative external ids, i.e. ids starting with `./` or `../`, will not be internally converted to an absolute id and converted back to a relative id in the output, but are instead included in the output unchanged. If you want relative ids to be renormalised and deduplicated instead, return an absolute file system location as `id` and choose `external: "relative"`.

After reading it, I still have no clue what `external` *does*, what absolute and relative IDs are and how their behavior differs. But I digress. Amid all this mess, there is [an example](https://rollupjs.org/plugin-development/#file-urls) for generating a virtual file using the `emitFile` API. Rollup plugins are valid Vite plugins, so it was easy to make such a plugin:

```javascript
import src from "virtual:example";
// `src` points to a file containing "Hello, world!"
```

```javascript
...
const referenceId = this.emitFile({
    type: "asset",
    name: "example.txt",
    source: "Hello, world!",
});
return `export default import.meta.ROLLUP_FILE_URL_${referenceId};`;
...
```

But when I started a dev server, Vite told me that *actually*, I can't use `emitFile`:

```
context method emitFile is not supported in serve mode. This plugin is likely not vite-compatible.
```

Turns out Vite uses Rollup for release only. How does it bundle files in development?

```javascript
const cssContent = await getContentWithSourcemap(css)
const code = [
  `import { updateStyle as __vite__updateStyle, removeStyle as __vite__removeStyle } from ${JSON.stringify(
    path.posix.join(config.base, CLIENT_PUBLIC_PATH),
  )}`,
  `const __vite__id = ${JSON.stringify(id)}`,
  `const __vite__css = ${JSON.stringify(cssContent)}`,
  `__vite__updateStyle(__vite__id, __vite__css)`,
  // css modules exports change on edit so it can't self accept
  `${modulesCode || 'import.meta.hot.accept()'}`,
  `import.meta.hot.prune(() => __vite__removeStyle(__vite__id))`,
].join('\n')
return { code, map: { mappings: '' } }
```

Oh, I see. It inlines the stylesheet into JS. What does it do with pictures?

```typescript
export async function fileToDevUrl(
  environment: Environment,
  id: string,
  asFileUrl = false,
): Promise<string> {
  ...
  let rtn: string
  if (publicFile) {
    // in public dir during dev, keep the url as-is
    rtn = id
  } else if (id.startsWith(withTrailingSlash(config.root))) {
    // in project root, infer short public path
    rtn = '/' + path.posix.relative(config.root, id)
  } else {
    // outside of project root, use absolute fs path
    // (this is special handled by the serve static middleware
    rtn = path.posix.join(FS_PREFIX, id)
  }
  const base = joinUrlSegments(config.server.origin ?? '', config.decodedBase)
  return joinUrlSegments(base, removeLeadingSlash(rtn))
}
```

Right, those are routed to `/@fs/<absolute path>`. Wait, what?

In a nutshell, there are no virtual files in dev mode. If you want a URL with dynamic contents, use `data:`. Oh, and `import.meta.ROLLUP_FILE_URL_${referenceId}` isn't actually the right thing to do even in release: it only works in JS files and produced complex code, instead you should use the [undocumented](https://github.com/vitejs/vite/issues/13459) magic string `__VITE_ASSET__${referenceId}__` because `???`. I love this ecosystem.

Say we somehow resolve all that and make `import imageSrc from "virtual:nerd-fonts/custom-cpp.svg"` work. Now we *just* need to tie two things together:

```typescript
---
// Usage: <Icon name="custom-cpp" />

interface Props {
    name: string;
}

const { name } = Astro.props;
const src = await import(`virtual:nerd-fonts/${name}.svg`);
---
<img src={src} />
```

...wait a second. That's a [dynamic import](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import). This wouldn't be an issue in typical NodeJS, which can resolve imports on-demand. But all code goes through Vite, which tries to build a Web bundle, so it wants to resolve modules before evaluation. Had we used an on-disk relative path, this would be translated roughly like this:

```javascript
const src = await import(`./images/${name}.svg`);
// ->
import file1 from "./images/file1.svg";
import file2 from "./images/file2.svg";
const src = {
    file1,
    file2,
    ...
}[name];
```

...totally negating the entire point of extracting only utilized glyphs from the font. Our only redemption is that this doesn't even compile, because apparently dynamic imports aren't supported for virtual files.

Well, okay, screw fast builds, let's just build everything and rely on tree shaking. What's the best library for extracting SVGs from fonts, anyway? [ttf2svg](https://www.npmjs.com/package/ttf2svg)? No, `npm audit` says that one has 15 vulnerabilities. I know, [harfbuzzjs](https://www.npmjs.com/package/harfbuzzjs) looks trustworthy:

```javascript
import { decompress } from "wawoff2";
import fs from "node:fs";
import hbPromise from "harfbuzzjs";

const iconNames = ...;
const renderString = ...;

const fontData = await decompress(fs.readFileSync(new URL("font.woff2", import.meta.url)));

const hb = await hbPromise;
const blob = hb.createBlob(fontData);
const face = hb.createFace(blob, 0);
const font = hb.createFont(face);

const buffer = hb.createBuffer();
buffer.addText(renderString);
buffer.guessSegmentProperties();
hb.shape(font, buffer);
const output = buffer.json();
const svgs = {};
output.forEach((glyph, i) => {
    const path = font.glyphToPath(glyph.g);
    svgs[iconNames[i]] = `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2048 2048">
            <path fill-rule="evenodd" clip-rule="evenodd" d="${path}" fill="#fff" />
        </svg>
    `;
});
buffer.destroy();

font.destroy();
face.destroy();
blob.destroy();
```

WHY ARE MY ICONS UPSIDE DOWN? Okay, we'll get to that in a bit, let's try something easier first, why are they cropped wrong? I guess I should've computed the viewbox instead of hard-coding numbers, let's use the `harfbuzzjs` API to get the bou-

> **[Calling `hExtents` seems to brick the font instance](https://github.com/harfbuzz/harfbuzzjs/issues/173)**
> 
> When running this code, I get:
> 
> ```
> { ascender: 0, descender: 0, lineGap: 0 }
> (empty line)
> ```
> 
> It seems like other operations break after `hExtents` as well: for instance, calling `buffer.json` seems to always return glyphs with ID `0`.
> 
> ---
>
> **[Fix memory corruption in hExtents/vExtents](https://github.com/harfbuzz/harfbuzzjs/pull/174)**
>
> We were allocation 12 bytes but the `hb_font_extents_t` struct is actually 48 bytes (the rest are private fields).

You know what, I don't want SVGs, let's try something else.


### Fonts

Maybe extracting SVGs from the font wasn't the brightest idea. It's going to trigger many HTTP requests, which might be slow. Let's use the font directly, we just need to drop unused glyphs. Let's see what [Astro docs](https://docs.astro.build/en/guides/fonts/) have to say about it:

> This API helps you keep your site performant with automatic [web font optimizations](https://web.dev/learn/performance/optimize-web-fonts) including preload links, optimized fallbacks, and opinionated defaults. See common usage examples.

So, that doesn't sound like optimizing fonts, that sounds like optimizing everything around fonts. I trust them though <img class="emoji" src="/images/ferrisClueless.webp" title=":ferrisClueless:" />. Let's check [the config](https://docs.astro.build/en/reference/configuration-reference/#fonts).

> `font.subsets`
> 
> Type: `Array<string>`  
> Default: `["latin"]`  
> Added in: astro@6.0.0
>
> Defines a list of font subsets to preload.
>
> `subsets: ["latin"]`

That doesn't look like it supports Unicode codepoints.

> `font.unicodeRange`

That's encouraging!

> Determines when a font must be downloaded and used based on a specific range of unicode characters. If a character on the page matches the configured range, the browser will download the font and all characters will be available for use on the page. To configure a subset of characters preloaded for a single font, see the subsets property instead.

Oh, that only affects preloading. Soooo... there basically isn't any font optimization.

Don't get me wrong, I'd gladly write it myself, but it's not clear how to automatically rebuild the font if new icons are used, and given that virtual files don't really work, I think it's fair to say this wouldn't work reliably anyway.


### UnoCSS

You know what, that's fair, no one cares about build performance and disk space anyway. Let's just extract SVGs by hand and see what we can do with that.

```shell
$ git submodule add https://github.com/ryanoasis/nerd-fonts nerd-fonts
<downloaded 1.3 GB>
$ cd nerd-fonts/src/svgs
$ ls -w100 *.svg
ada_nf.svg      elm_nf.svg     karma.svg          rollup.svg
apple.svg       emacs_nf.svg       kotlin_nf.svg      R.svg
argdown.svg     error.svg      less.svg       ruby_nf.svg
asm_nf.svg      eslint.svg     license.svg        rust.svg
...
ejs.svg         java.svg       rails.svg          yml.svg
electron_nf.svg     jenkins.svg    react.svg          zig.svg
elixir_nf.svg       jinja.svg      reasonml.svg       zip.svg
elixir_script.svg   julia.svg      rescript.svg       zsh_nf.svg
```

A plain `import ... from "<path>.svg"` will either inline `data:` URLs in each `<img>` (bad for page size and caching), or trigger multiple HTTP requests (bad for latency).

Someone told me [UnoCSS](https://unocss.dev/) solves this cleanly: add a class with `background-image: url(data:...)` for each icon and let it remove unused definitions. It took me a while to figure out what UnoCSS does, but it's basically Tailwind on steroids: it [parses HTML with regex](https://stackoverflow.com/a/1732454/5417677) to get a list of used classes and emits only the required CSS rules.

You know, like tree shaking, but for CSS. So it's basically a CSS bundler implemented from scratch. You know, like Rollup is a bundler and Vite is kinda a bundler. You can [integrate it with Astro](https://unocss.dev/integrations/astro) if you want three bundlers in one build. Do you like bundlers?

```shell
$ npm run build
$ firefox
```

The dimensions are wrong, but that's expected from replacing a font with an SVG. I still have to hard-code the icon list, and I need to add other icon sources from `npm` because `src/svgs` doesn't contain all glyphs. But maybe it's not that b-

```shell
$ ls dist/_astro/
...
.rw-r--r--  960 purplesyringa purplesyringa 31 Mar 01:48 -I 󰕙 github-mark-white.S2fJVXLq.svg
.rw-r--r--  960 purplesyringa purplesyringa 31 Mar 01:48 -I 󰕙 github-mark-white.S2fJVXLq_Z2nI3it.svg
...
```

Why are there two identi--

```shell
$ npm ls
bookshelf@0.0.1 /home/purplesyringa/bookshelf
├── @astrojs/check@0.9.8
├── @astrojs/compiler-rs@0.1.6
├── @astrojs/mdx@5.0.3
├── @astrojs/rss@4.0.18
├── @astrojs/sitemap@3.7.2
├── @emnapi/core@1.9.1 extraneous
├── @emnapi/runtime@1.9.1 extraneous
├── @emnapi/wasi-threads@1.2.0 extraneous
├── @napi-rs/wasm-runtime@1.1.1 extraneous
├── @shikijs/transformers@4.0.2
├── @tybys/wasm-util@0.10.1 extraneous
├── @unocss/astro@66.6.7
├── @unocss/preset-icons@66.6.7
├── astro@6.1.1
├── sharp@0.34.5
├── tslib@2.8.1 extraneous
├── typescript@5.9.3
└── unocss@66.6.7
```

Why are there extraneous packages right after `npm clean-in`--

```shell
$ cat dist/index.html
...
    <section data-astro-cid-w6ymvdtg="true" data-astro-cid-w56doo5x="true" class="parent">
    <div class="container" data-astro-cid-w56doo5x> 
    
        <p>
Hi! 👋 I'm <b>Alisa Sireneva</b> (she/her), a 21yo dev from Moscow.
</p>
    
 </div>
</section>  
...
```

Why are there multiple spaces despite `minifyHtml`?


### Sigh

There are always going to be bugs and missing features. I'm a developer, I get it.

What I don't get is why [Astro decided](https://github.com/withastro/astro/issues/6011#issuecomment-1409046168) that actually, React, Vue, and the rest of the frameworks were wrong to consider whitespace in templates insignificant, and so they're going to make everyone write long lines and use `{/* prettier-ignore */}`. No, I can't just use React for templating, that turns the component into an island.

Why did we have like three tools post-processing each other's output to implement different sides of the same functionality, requiring [non-trivial integrations](https://github.com/unocss/unocss/tree/main/packages-integrations/vite/src) to keep HMR working, and then someone went, "You know what will fix frontend development once and for all? Making another wrapper around these tools!"

I want to like Astro. I absolutely love the island architecture and easy content discovery. I want to like JavaScript. I love having a terse GC language with an enormous ecosystem, built-in WebAssembly support, and JSX. I want to like Vite, and Rolldown, and [unified](https://unifiedjs.com/)... but why do none of these tools offer a way to implement tree-shaking for fonts despite supporting it for JS and CSS? Why do I have to learn from single-page docs? And why do I have to use software with bangers like:

```javascript
generatedFiles = [...generatedFiles, { image, width, format }];
```

It could be *so* simple. Allow components to directly emit virtual files. Might as well let them populate global sets that can then be read out in some centralized manner -- that'd handle CSS bundling and font subsetting. It wouldn't even need any clever hot-reload logic -- just pure library functions. But it's not the way Rollup works, so it's not the way Vite works, so it's not the way Astro works. And Rollup is not going away.


### Fin.

I'll probably finish the Astro port. It hurts that features that were trivial in my scripts are becoming complicated, and it won't resolve issues with slow tikz builds, but it's probably easier to maintain than whatever I'm using right now. Small steps.

Longer term I'll probably switch to a custom framework. Cliché, I know. Boo hoo. I drafted a Rust version as an experiment, but I have no clue how well hot reloading code would work, so I'll probably switch to JS. Having JSX would be great, and the pitfalls should be less... arbitrary. That's not happening anytime soon, though.

In the meantime, enjoy a slightly improved HTML layout: `links` seems to parse it better, if anyone still uses it, and accessibility should be improved.
