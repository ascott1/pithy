import { describe, it, expect } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import {
	syntaxHighlighting,
	defaultHighlightStyle,
} from "@codemirror/language";
import { inlineRendering } from "./inlineRendering";

function createView(doc: string, cursor = 0): EditorView {
	const state = EditorState.create({
		doc,
		extensions: [
			markdown(),
			inlineRendering(),
			syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
		],
		selection: { anchor: cursor },
	});
	const container = document.createElement("div");
	document.body.appendChild(container);
	return new EditorView({ state, parent: container });
}

function getTextContent(view: EditorView): string {
	return view.contentDOM.textContent ?? "";
}

function hasClass(view: EditorView, className: string): boolean {
	return view.contentDOM.querySelector(`.${className}`) !== null ||
		view.dom.querySelector(`.${className}`) !== null;
}

describe("inlineRendering", () => {
	describe("headings", () => {
		it("hides # markers when cursor is outside", () => {
			const doc = "# Hello\n\nsome text";
			const view = createView(doc, doc.length);
			// The # and space should be hidden, leaving just "Hello"
			const lines = view.contentDOM.querySelectorAll(".cm-line");
			const firstLine = lines[0];
			expect(firstLine?.textContent).toBe("Hello");
		});

		it("shows # markers when cursor is inside heading", () => {
			const doc = "# Hello";
			const view = createView(doc, 3);
			const lines = view.contentDOM.querySelectorAll(".cm-line");
			const firstLine = lines[0];
			expect(firstLine?.textContent).toBe("# Hello");
		});

		it("applies heading class when not active", () => {
			const doc = "## Heading\n\ntext";
			const view = createView(doc, doc.length);
			expect(hasClass(view, "cm-md-h2")).toBe(true);
		});
	});

	describe("bold", () => {
		it("hides ** delimiters when cursor is outside", () => {
			const doc = "some **bold** text";
			const view = createView(doc, 0);
			expect(getTextContent(view)).toBe("some bold text");
		});

		it("shows ** delimiters when cursor is inside", () => {
			const doc = "some **bold** text";
			const view = createView(doc, 8);
			expect(getTextContent(view)).toBe("some **bold** text");
		});

		it("applies bold class", () => {
			const doc = "some **bold** text";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-strong")).toBe(true);
		});
	});

	describe("italic", () => {
		it("hides * delimiters when cursor is outside", () => {
			const doc = "some *italic* text";
			const view = createView(doc, 0);
			expect(getTextContent(view)).toBe("some italic text");
		});

		it("shows * delimiters when cursor is inside", () => {
			const doc = "some *italic* text";
			const view = createView(doc, 7);
			expect(getTextContent(view)).toBe("some *italic* text");
		});
	});

	describe("links", () => {
		it("hides brackets and URL when cursor is outside", () => {
			const doc = "click [here](https://example.com) now";
			const view = createView(doc, 0);
			expect(getTextContent(view)).toBe("click here now");
		});

		it("shows full link syntax when cursor is inside", () => {
			const doc = "click [here](https://example.com) now";
			const view = createView(doc, 10);
			expect(getTextContent(view)).toBe(
				"click [here](https://example.com) now",
			);
		});

		it("applies link class", () => {
			const doc = "click [here](https://example.com) now";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-link")).toBe(true);
		});
	});

	describe("inline code", () => {
		it("hides backticks when cursor is outside", () => {
			const doc = "use `code` here";
			const view = createView(doc, 0);
			expect(getTextContent(view)).toBe("use code here");
		});

		it("shows backticks when cursor is inside", () => {
			const doc = "use `code` here";
			const view = createView(doc, 6);
			expect(getTextContent(view)).toBe("use `code` here");
		});
	});

	describe("fenced code blocks", () => {
		it("applies code block class when cursor is outside", () => {
			const doc = "text\n\n```js\nconsole.log(1)\n```\n\nmore";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-code-block")).toBe(true);
		});

		it("applies fence class to fence lines", () => {
			const doc = "text\n\n```js\nconsole.log(1)\n```\n\nmore";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-code-fence")).toBe(true);
		});
	});

	describe("blockquotes", () => {
		it("hides > marker when cursor is outside", () => {
			const doc = "> quoted text\n\nnormal";
			const view = createView(doc, doc.length);
			const lines = view.contentDOM.querySelectorAll(".cm-line");
			expect(lines[0]?.textContent).toBe("quoted text");
		});

		it("shows > marker when cursor is inside", () => {
			const doc = "> quoted text";
			const view = createView(doc, 3);
			const lines = view.contentDOM.querySelectorAll(".cm-line");
			expect(lines[0]?.textContent).toBe("> quoted text");
		});

		it("applies blockquote class", () => {
			const doc = "> quoted\n\ntext";
			const view = createView(doc, doc.length);
			expect(hasClass(view, "cm-md-blockquote")).toBe(true);
		});
	});

	describe("nested formatting", () => {
		it("renders bold inside a heading", () => {
			const doc = "# A **bold** heading\n\ntext";
			const view = createView(doc, doc.length);
			expect(hasClass(view, "cm-md-h1")).toBe(true);
			expect(hasClass(view, "cm-md-strong")).toBe(true);
			// # and ** should be hidden
			const firstLine = view.contentDOM.querySelectorAll(".cm-line")[0];
			expect(firstLine?.textContent).toBe("A bold heading");
		});

		it("renders italic inside bold", () => {
			const doc = "some **bold *and italic*** text";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-strong")).toBe(true);
			expect(hasClass(view, "cm-md-em")).toBe(true);
			expect(getTextContent(view)).toBe("some bold and italic text");
		});

		it("renders bold inside a blockquote", () => {
			const doc = "> some **bold** text\n\nnormal";
			const view = createView(doc, doc.length);
			expect(hasClass(view, "cm-md-blockquote")).toBe(true);
			expect(hasClass(view, "cm-md-strong")).toBe(true);
			const firstLine = view.contentDOM.querySelectorAll(".cm-line")[0];
			expect(firstLine?.textContent).toBe("some bold text");
		});

		it("renders formatted link text", () => {
			const doc = "[**bold link**](https://example.com) text";
			const view = createView(doc, doc.length);
			expect(hasClass(view, "cm-md-link")).toBe(true);
			expect(hasClass(view, "cm-md-strong")).toBe(true);
		});

		it("reveals only active node when cursor is on nested child", () => {
			const doc = "**bold *italic* bold**";
			// cursor on "italic" — inside Emphasis but also inside StrongEmphasis
			const view = createView(doc, 10);
			// Both bold and italic should be raw (cursor is inside both)
			expect(getTextContent(view)).toBe("**bold *italic* bold**");
		});
	});

	describe("images", () => {
		it("replaces image syntax with img element when cursor is outside", () => {
			const doc = "text\n\n![alt](https://example.com/img.png)\n\nmore";
			const view = createView(doc, 0);
			const img = view.dom.querySelector(".cm-md-image");
			expect(img).not.toBeNull();
			expect(img?.getAttribute("src")).toBe("https://example.com/img.png");
			expect(img?.getAttribute("alt")).toBe("alt");
		});

		it("shows raw syntax when cursor is inside image", () => {
			const doc = "![alt](https://example.com/img.png)";
			const view = createView(doc, 5);
			const img = view.dom.querySelector(".cm-md-image");
			expect(img).toBeNull();
			expect(getTextContent(view)).toContain("![alt]");
		});

		it("handles empty alt text", () => {
			const doc = "text\n\n![](https://example.com/img.png)\n\nmore";
			const view = createView(doc, 0);
			const img = view.dom.querySelector(".cm-md-image");
			expect(img).not.toBeNull();
			expect(img?.getAttribute("alt")).toBe("");
		});

		it("blocks javascript: URI scheme", () => {
			const doc = "text\n\n![xss](javascript:alert(1))\n\nmore";
			const view = createView(doc, 0);
			const img = view.dom.querySelector(".cm-md-image");
			expect(img).toBeNull();
		});

		it("blocks data: URI scheme", () => {
			const doc = "text\n\n![x](data:text/html,<script>alert(1)</script>)\n\nmore";
			const view = createView(doc, 0);
			const img = view.dom.querySelector(".cm-md-image");
			expect(img).toBeNull();
		});

		it("renders multiple images in one document", () => {
			const doc = "![a](https://example.com/1.png)\n\n![b](https://example.com/2.png)\n\ntext";
			const view = createView(doc, doc.length);
			const imgs = view.dom.querySelectorAll(".cm-md-image");
			expect(imgs.length).toBe(2);
		});
	});

	describe("horizontal rules", () => {
		it("hides --- text and adds hr line class when cursor is outside", () => {
			const doc = "above\n\n---\n\nbelow";
			const view = createView(doc, 0);
			expect(hasClass(view, "cm-md-hr-line")).toBe(true);
			// The --- text should be hidden
			expect(getTextContent(view)).not.toContain("---");
		});

		it("shows raw --- when cursor is on the rule", () => {
			const doc = "above\n\n---\n\nbelow";
			const view = createView(doc, 8);
			expect(hasClass(view, "cm-md-hr-line")).toBe(false);
			expect(getTextContent(view)).toContain("---");
		});
	});
});
