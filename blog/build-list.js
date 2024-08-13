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
			...parsedYamlHeader
		});
	}
};

addFromDir(".");
addFromDir("ru");

posts.sort((a, b) => b.ordering - a.ordering);

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
