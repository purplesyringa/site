import { Buffer } from "node:buffer";
import childProcess from "node:child_process";
import escapeHTML from "escape-html";
import fs from "node:fs";
import hljs from "highlight.js";
import markdownit from "markdown-it";
import markdownitContainer from "markdown-it-container";
import markdownitTexMath from "markdown-it-texmath";
import minifyHtml from "@minify-html/node";
import path from "node:path";
import process from "node:process";
import temml from "temml";
import tmp from "tmp";
import YAML from "yaml";

tmp.setGracefulCleanup();

const articleDirectory = process.env.INIT_CWD;

const fileText = fs.readFileSync(`${articleDirectory}/index.md`, "utf-8");
const [_, yamlHeader, markdown] = fileText.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)/);
const parsedYamlHeader = YAML.parse(yamlHeader);

let html = fs.readFileSync("_template.html", "utf-8");
html = html.replace(/{{ root }}/g, escapeHTML(path.relative(articleDirectory, process.cwd() + "/..")));
html = html.replace(/{{ title }}/g, escapeHTML(parsedYamlHeader.title));
html = html.replace(/{{ time }}/g, escapeHTML(parsedYamlHeader.time));

let spoilerId = 0;

const md = markdownit({
	html: true,
	typographer: true,
	highlight(code, language, opts) {
		if (language === "") {
			return "";
		}
		if (language === "tikz") {
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
			return `<div class="expansible-code"><input type="checkbox" id="expansible${spoilerId}"><div class="highlighted">${highlighted}</div><label for="expansible${spoilerId}"></label></div>`;
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

html = html.replace(/{{ body }}/g, md.render(markdown).replace(/<aside-inline-here \/>/g, "</div>"));

html = minifyHtml.minify(Buffer.from(html), {});

fs.writeFileSync(`${articleDirectory}/index.html`, html);
