import { Buffer } from "node:buffer";
import childProcess from "node:child_process";
import escapeHTML from "escape-html";
import fs from "node:fs";
import hljs from "highlight.js";
import Jimp from "jimp";
import markdownit from "markdown-it";
import markdownitContainer from "markdown-it-container";
import markdownitTexMath from "markdown-it-texmath";
import minifyHtml from "@minify-html/node";
import path from "node:path";
import process from "node:process";
import { stripHtml } from "string-strip-html";
import temml from "temml";
import tmp from "tmp";
import YAML from "yaml";

tmp.setGracefulCleanup();

const articleDirectory = process.env.INIT_CWD;

const fileText = fs.readFileSync(`${articleDirectory}/index.md`, "utf-8");
const [_, yamlHeader, markdown] = fileText.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)/);
const parsedYamlHeader = YAML.parse(yamlHeader);

const relPath = path.relative(process.cwd(), articleDirectory);
const locale = relPath.startsWith("ru/") ? "ru_RU" : "en_US";

const image = await Jimp.read("og_template.png");
const font = await Jimp.loadFont("../fonts/lilitaone.fnt");
image.print(
	font,
	100,
	100,
	{
		text: locale === "en_US" ? parsedYamlHeader.title : "purplesyringa's blog",
		alignmentX: Jimp.HORIZONTAL_ALIGN_CENTER,
		alignmentY: Jimp.VERTICAL_ALIGN_MIDDLE,
	},
	1000,
	430,
);
await image.writeAsync(`${articleDirectory}/og.png`);

let spoilerId = 0;

const md = markdownit({
	html: true,
	typographer: true,
	highlight(code, language, opts) {
		if (language === "") {
			return "";
		}
		if (language === "tikz") {
			const altText = code.startsWith("% alt ") ? code.match(/^% alt (.*)/)[1] : "";

			const outputDir = tmp.dirSync({ unsafeCleanup: true }).name;
			let rendered = "";
			for (const theme of ["light", "dark"]) {
				const defaultColor = { light: "black", dark: "white" }[theme];
				fs.writeFileSync(`${outputDir}/diagram.tex`, String.raw`\documentclass{standalone}
\usepackage[svgnames]{xcolor}
\usepackage{tikz}
\usepackage[sfdefault]{roboto}
\usetikzlibrary{arrows.meta}
\usetikzlibrary{decorations.pathreplacing}
\usetikzlibrary{shapes.geometric}

\begin{document}

\begin{tikzpicture}[draw=${defaultColor},text=${defaultColor}]
${parsedYamlHeader.tikzThemes[theme]}
${code}
\end{tikzpicture}

\end{document}`);
				// childProcess.execFileSync("pdflatex", ["-interaction=batchmode", `-output-directory=${outputDir}`, "diagram.tex"]);
				// childProcess.execFileSync("libreoffice", ["--convert-to", "svg", "--outdir", outputDir, `${outputDir}/diagram.pdf`]);
				childProcess.execFileSync("latex", ["-interaction=batchmode", `-output-directory=${outputDir}`, "diagram.tex"]);
				childProcess.execFileSync("dvisvgm", ["--optimize", "--no-fonts", `--output=${outputDir}/diagram`, `${outputDir}/diagram.dvi`]);
				let svg = fs.readFileSync(`${outputDir}/diagram.svg`, "utf-8");
				svg = svg.replace(/<\/svg>/, `<title>${escapeHTML(altText)}</title></svg>`);
				svg = svg.replace(/<\?xml.*?\?>/, "");
				svg = svg.replace(/<!DOCTYPE.*?>/, "");
				rendered += `<div class="diagram only-${theme}">${svg}</div>`;
			}
			// Fuck you, that's why
			return {
				indexOf: () => 0,
				toString: () => rendered,
			};
		}
		const highlighted = hljs.highlight(code, { language }).value;
		if (opts === "expansible") {
			spoilerId++;
			return `<div class="expansible-code"><input type="checkbox" id="expansible${spoilerId}"><div class="highlighted">${highlighted}</div><label for="expansible${spoilerId}">Expand</label></div>`;
		} else {
			return highlighted;
		}
	},
});
md.use(markdownitContainer, "aside", {
	render(tokens, idx) {
		return tokens[idx].nesting === 1 ? "<div class='aside-group'><aside>\n" : "</aside>\n";
	},
});
md.use(markdownitTexMath, {
	engine: temml,
});

let html = fs.readFileSync("_template.html", "utf-8");
html = html.replace(/{{ root }}/g, escapeHTML(path.relative(articleDirectory, process.cwd() + "/..")));
html = html.replace(/{{ title }}/g, escapeHTML(parsedYamlHeader.title));
html = html.replace(/{{ path }}/g, escapeHTML(relPath));
html = html.replace(/{{ description }}/g, stripHtml(md.render(parsedYamlHeader.intro || "")).result);
html = html.replace(/{{ time }}/g, escapeHTML(parsedYamlHeader.time));
html = html.replace(/{{ locale }}/g, locale);
if (parsedYamlHeader.discussion) {
	const discussionSpace = (
		parsedYamlHeader.discussion.startsWith("https://codeforces.com") ? "Codeforces" :
			parsedYamlHeader.discussion.startsWith("https://www.reddit.com") ? "Reddit" :
				parsedYamlHeader.discussion.startsWith("https://t.me") ? "Telegram" :
					"???"
	);
	html = html.replace(/{{ discussion }}/g, escapeHTML(parsedYamlHeader.discussion));
	html = html.replace(/{{ discussion_space }}/g, discussionSpace);
	html = html.replace(/{% (end)?if discussion %}/g, "");
} else {
	html = html.replace(/{% if discussion %}.*?{% endif discussion %}/g, "");
}
html = html.replace(
	/{{ body }}/g,
	md.render(markdown)
		.replace(/<aside-inline-here \/>/g, "</div>")
		.replace(/<table>/g, "<div class='table-wrapper'><table>")
		.replace(/<\/table>/g, "</table></div>")
		.replace(/<h3>(.*?)<\/h3>\s*<div class='aside-group'><aside>([\s\S]*?)<\/aside>/g, "<div class='aside-group'><aside>$2</aside><h3>$1</h3>")
		.replace(/<h3>(.*?)<\/h3>\s*<p>/g, "<p class='next-group'><span class='side-header' role='heading' aria-level='3'><span>$1</span></span>")
);

html = minifyHtml.minify(Buffer.from(html), {});

fs.writeFileSync(`${articleDirectory}/index.html`, html);
