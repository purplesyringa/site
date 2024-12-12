import { Buffer } from "node:buffer";
import escapeHTML from "escape-html";
import fs from "node:fs";
import hljs from "highlight.js";
import markdownit from "markdown-it";
import markdownitTexMath from "markdown-it-texmath";
import minifyHtml from "@minify-html/node";
import { stripHtml } from "string-strip-html";
import temml from "temml";
import YAML from "yaml";

const md = markdownit({
	html: true,
	typographer: true,
	highlight(code, language) {
		return language === "" ? "" : hljs.highlight(code, { language }).value;
	},
});
md.use(markdownitTexMath, {
	engine: temml,
});

const posts = [];

const addFromDir = dir => {
	for (const name of fs.readdirSync(dir)) {
		let fileText;
		try {
			fileText = fs.readFileSync(`${dir}/${name}/index.md`, "utf-8");
		} catch (e) {
			if (e.code === "ENOTDIR" || e.code === "ENOENT") {
				continue;
			} else {
				throw e;
			}
		}

		const [_, yamlHeader, markdown] = fileText.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)/);
		const parsedYamlHeader = YAML.parse(yamlHeader);
		if (parsedYamlHeader.draft) {
			continue;
		}
		posts.push({
			...parsedYamlHeader,
			path: `${dir}/${name}`,
			parsedDate: new Date(parsedYamlHeader.time + " UTC"),
			discussion: typeof parsedYamlHeader.discussion === "string" ? [parsedYamlHeader.discussion] : parsedYamlHeader.discussion || []
		});
	}
};

addFromDir(".");
addFromDir("ru");

posts.sort((a, b) => b.parsedDate - a.parsedDate);

let content = posts.map(post => {
	const discussion = post.discussion.map(url => {
		const space = (
			url.startsWith("https://internals.rust-lang.org") ? "IRLO" :
				url.startsWith("https://news.ycombinator.com") ? "Hacker News" :
					url.startsWith("https://codeforces.com") ? "Codeforces" :
						url.startsWith("https://www.reddit.com") ? "Reddit" :
							url.startsWith("https://lobste.rs") ? "Lobsters" :
								url.startsWith("https://t.me") ? "Telegram" :
									"???"
		);
		return `<a class="discussion" href="${escapeHTML(url)}"><i class="nf nf-md-comment" title="Comment"></i> ${space}</a>`;
	}).join("");
	const translation = Object.entries(post.translation || {}).map(([language, url]) => {
		return `<a class="discussion" href="${escapeHTML(url)}"><i class="nf nf-md-translate" title="Translation"></i> ${language}</a>`;
	}).join("");
	return `
		<div class="post-entry">
			<h2><a href="${escapeHTML(post.path)}/">${escapeHTML(post.title)}</a></h2>
			<time>${escapeHTML(post.time)}</time>
			${discussion}
			${translation}
			${md.render(post.intro || "")}
			<p>
				<a href="${escapeHTML(post.path)}/">Keep reading</a>
			</p>
		</div>
	`;
}).join("");

let html = fs.readFileSync("_index_template.html", "utf-8");
html = html.replace(/{{ content }}/g, content);

html = minifyHtml.minify(Buffer.from(html), {});

fs.writeFileSync("index.html", html);

fs.writeFileSync("feed.rss", `<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
	<channel>
		<title>purplesyringa's blog</title>
		<link>https://purplesyringa.moe/blog/</link>
		<description>Posts from purplesyringa's blog.</description>
		<copyright>Alisa Sireneva, CC BY</copyright>
		<managingEditor>me@purplesyringa.moe (Alisa Sireneva)</managingEditor>
		<webMaster>me@purplesyringa.moe (Alisa Sireneva)</webMaster>
		<lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
		<docs>https://www.rssboard.org/rss-specification</docs>
		<ttl>60</ttl>
		<atom:link href="https://purplesyringa.moe/blog/feed.rss" rel="self" type="application/rss+xml" />
		${posts.map(post => `
			<item>
				<title>${escapeHTML(post.title)}</title>
				<link>${escapeHTML(`https://purplesyringa.moe/blog/${post.path}/${post.path === "./webp-the-webpage-compression-format" ? "nojs.html" : ""}`)}</link>
				<description>${stripHtml(md.render(post.intro || "")).result}</description>
				<author>me@purplesyringa.moe (Alisa Sireneva)</author>
				${post.discussion.length > 0 ? `<comments>${escapeHTML(post.discussion[0])}</comments>` : ""}
				<guid>${escapeHTML(`https://purplesyringa.moe/blog/${post.path}/`)}</guid>
				<pubDate>${post.parsedDate.toUTCString()}</pubDate>
			</item>
		`).join("")}
	</channel>
</rss>`);
