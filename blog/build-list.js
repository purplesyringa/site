import { Buffer } from "node:buffer";
import escapeHTML from "escape-html";
import fs from "node:fs";
import hljs from "highlight.js";
import markdownit from "markdown-it";
import markdownitContainer from "markdown-it-container";
import markdownitTexMath from "markdown-it-texmath";
import minifyHtml from "@minify-html/node";
import path from "node:path";
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
		posts.push({
			path: `${dir}/${name}`,
			parsedDate: new Date(parsedYamlHeader.time + " UTC"),
			...parsedYamlHeader
		});
	}
};

addFromDir(".");
addFromDir("ru");

posts.sort((a, b) => b.parsedDate - a.parsedDate);

let content = posts.map(post => {
	return `
		<div class="post-entry">
			<h2><a href="${escapeHTML(post.path)}/">${escapeHTML(post.title)}</a></h2>
			<time>${escapeHTML(post.time)}</time>
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
<rss version="2.0">
	<channel>
		<title>purplesyringa's blog</title>
		<link>https://purplesyringa.moe/blog/</link>
		<description>Posts from purplesyringa's blog.</description>
		<copyright>Alisa Sireneva, CC BY</copyright>
		<managingEditor>me@purplesyringa.moe</managingEditor>
		<webMaster>me@purplesyringa.moe</webMaster>
		<lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
		<docs>https://www.rssboard.org/rss-specification</docs>
		<ttl>60</ttl>
		${posts.map(post => `
			<item>
				<title>${escapeHTML(post.title)}</title>
				<link>${escapeHTML(`https://purplesyringa.moe/blog/${post.path}/`)}</link>
				<description>Here is some text containing an interesting description.</description>
				<author>me@purplesyringa.moe</author>
				${"" /* <comments>URL to hackernews</comments> */}
				<guid>${escapeHTML(`https://purplesyringa.moe/blog/${post.path}/`)}</guid>
				<pubDate>${post.parsedDate.toUTCString()}</pubDate>
			</item>
		`).join("")}
	</channel>
</rss>`);
