import {
	Decoration,
	type DecorationSet,
	EditorView,
	ViewPlugin,
} from "@codemirror/view";
import {
	syntaxTree,
	HighlightStyle,
	syntaxHighlighting,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";
import { StateField, type EditorState, type Extension, type Range, type SelectionRange } from "@codemirror/state";
import { openUrl } from "@tauri-apps/plugin-opener";
import { wikilinkNavigateFacet } from "$lib/editor/wikilink";

// --- Hoisted decoration constants (immutable, shared across all builds) ---
const REPLACE = Decoration.replace({});
const MARK_STRONG = Decoration.mark({ class: "cm-md-strong" });
const MARK_EM = Decoration.mark({ class: "cm-md-em" });
const MARK_LINK = Decoration.mark({ class: "cm-md-link" });
const MARK_WIKILINK = Decoration.mark({ class: "cm-md-wikilink" });
const MARK_INLINE_CODE = Decoration.mark({ class: "cm-md-inline-code" });
const LINE_BLOCKQUOTE = Decoration.line({ class: "cm-md-blockquote" });
const LINE_CODE_BLOCK = Decoration.line({ class: "cm-md-code-block" });
const LINE_CODE_FENCE = Decoration.line({
	class: "cm-md-code-block cm-md-code-fence",
});
const LINE_HR = Decoration.line({ class: "cm-md-hr-line" });
const HEADING_LINE_DECOS: Decoration[] = [
	Decoration.line({ class: "cm-md-heading" }), // unused index 0
	Decoration.line({ class: "cm-md-heading cm-md-h1" }),
	Decoration.line({ class: "cm-md-heading cm-md-h2" }),
	Decoration.line({ class: "cm-md-heading cm-md-h3" }),
	Decoration.line({ class: "cm-md-heading cm-md-h4" }),
	Decoration.line({ class: "cm-md-heading cm-md-h5" }),
	Decoration.line({ class: "cm-md-heading cm-md-h6" }),
];

// --- Micro-optimization: Set lookup instead of regex for heading names ---
const HEADING_NAMES = new Set([
	"ATXHeading1",
	"ATXHeading2",
	"ATXHeading3",
	"ATXHeading4",
	"ATXHeading5",
	"ATXHeading6",
]);

/**
 * Returns true if any selection range intersects [from, to].
 * Empty cursors use inclusive boundary checks so that the cursor
 * at a delimiter edge still counts as "inside".
 */
function selectionIntersects(
	from: number,
	to: number,
	ranges: readonly SelectionRange[],
): boolean {
	for (const r of ranges) {
		if (r.empty) {
			if (from <= r.head && r.head <= to) return true;
		} else {
			const a = Math.min(r.from, r.to);
			const b = Math.max(r.from, r.to);
			if (from < b && a < to) return true;
		}
	}
	return false;
}

/**
 * Resolves a document position to a link URL, if the position is inside
 * a rendered (non-active) Link node. Returns null if the cursor is inside
 * the link (raw markdown visible), or if the URL is not an external link.
 */
function getLinkUrl(view: EditorView, pos: number): string | null {
	const { state } = view;
	let node = syntaxTree(state).resolveInner(pos);

	// Walk up to find the Link parent
	while (node && node.name !== "Link") {
		if (!node.parent) return null;
		node = node.parent;
	}
	if (!node || node.name !== "Link") return null;

	// If cursor intersects the link range, raw markdown is visible — don't open
	if (selectionIntersects(node.from, node.to, state.selection.ranges)) {
		return null;
	}

	// Find the URL child node
	const cur = node.cursor();
	if (cur.firstChild()) {
		do {
			if (cur.name === "URL") {
				const url = state.doc.sliceString(cur.from, cur.to);
				if (
					url &&
					(url.startsWith("http://") ||
						url.startsWith("https://") ||
						url.startsWith("mailto:"))
				) {
					return url;
				}
				return null;
			}
		} while (cur.nextSibling());
	}

	return null;
}

/**
 * Resolves a document position to a wikilink target, if the position is inside
 * a rendered (non-active) WikiLink node. Returns null if the cursor is inside
 * the link (raw markdown visible).
 */
function getWikilinkTarget(view: EditorView, pos: number): string | null {
	const { state } = view;
	let node = syntaxTree(state).resolveInner(pos);

	while (node && node.name !== "WikiLink") {
		if (!node.parent) return null;
		node = node.parent;
	}
	if (!node || node.name !== "WikiLink") return null;

	if (selectionIntersects(node.from, node.to, state.selection.ranges)) {
		return null;
	}

	// Extract text between the two WikiLinkMark children
	const cur = node.cursor();
	let markEnd = -1;
	let markStart = -1;
	if (cur.firstChild()) {
		do {
			if (cur.name === "WikiLinkMark") {
				if (markEnd < 0) {
					markEnd = cur.to; // end of opening [[
				} else {
					markStart = cur.from; // start of closing ]]
				}
			}
		} while (cur.nextSibling());
	}

	if (markEnd >= 0 && markStart >= 0 && markEnd < markStart) {
		return state.doc.sliceString(markEnd, markStart);
	}
	return null;
}

/**
 * Scans from the start of `text` for blockquote prefix characters (`>` and space).
 * Returns the length of the prefix, or 0 if not a blockquote line.
 * Replaces regex /^((?:> ?)+)/ with a direct character scan.
 */
function blockquotePrefixLen(text: string): number {
	let i = 0;
	const len = text.length;
	while (i < len) {
		const ch = text.charCodeAt(i);
		if (ch === 62 /* > */) {
			i++;
			if (i < len && text.charCodeAt(i) === 32 /* space */) {
				i++;
			}
		} else {
			break;
		}
	}
	return i;
}

function buildDecorations(state: EditorState): DecorationSet {
	const decorations: Range<Decoration>[] = [];
	const selections = state.selection.ranges;
	const doc = state.doc;

	// Iterate the full document so that decorations (especially line
	// decorations that change font-size/padding) persist at all positions.
	// CM6 accounts for StateField decorations in height estimation,
	// preventing invisible text when scrolling long documents.
	const tree = syntaxTree(state);
	tree.iterate({
		enter(node) {
			const name = node.name;
			const active = selectionIntersects(
				node.from,
				node.to,
				selections,
			);

			// --- ATX Headings ---
			if (HEADING_NAMES.has(name)) {
				if (active) return;
				const level = name.charCodeAt(10) - 48;
				const lineDeco = HEADING_LINE_DECOS[level];

				const startLine = doc.lineAt(node.from);
				const endLine = doc.lineAt(node.to);
				for (let n = startLine.number; n <= endLine.number; n++) {
					decorations.push(lineDeco.range(doc.line(n).from));
				}

				const cur = node.node.cursor();
				if (cur.firstChild()) {
					do {
						if (cur.name === "HeaderMark") {
							let end = cur.to;
							if (
								end < doc.length &&
								doc.sliceString(end, end + 1) === " "
							) {
								end++;
							}
							decorations.push(REPLACE.range(cur.from, end));
						}
					} while (cur.nextSibling());
				}
				return;
			}

			// --- Bold ---
			if (name === "StrongEmphasis") {
				if (active) return;
				const cur = node.node.cursor();
				const marks: { from: number; to: number }[] = [];
				if (cur.firstChild()) {
					do {
						if (cur.name === "EmphasisMark") {
							marks.push({ from: cur.from, to: cur.to });
						}
					} while (cur.nextSibling());
				}
				if (marks.length >= 2) {
					for (const m of marks) {
						decorations.push(REPLACE.range(m.from, m.to));
					}
					decorations.push(
						MARK_STRONG.range(
							marks[0].to,
							marks[marks.length - 1].from,
						),
					);
				}
				return;
			}

			// --- Italic ---
			if (name === "Emphasis") {
				if (active) return;
				const cur = node.node.cursor();
				const marks: { from: number; to: number }[] = [];
				if (cur.firstChild()) {
					do {
						if (cur.name === "EmphasisMark") {
							marks.push({ from: cur.from, to: cur.to });
						}
					} while (cur.nextSibling());
				}
				if (marks.length >= 2) {
					for (const m of marks) {
						decorations.push(REPLACE.range(m.from, m.to));
					}
					decorations.push(
						MARK_EM.range(
							marks[0].to,
							marks[marks.length - 1].from,
						),
					);
				}
				return;
			}

			// --- Links ---
			if (name === "Link") {
				if (active) return;
				const cur = node.node.cursor();
				let textStart = -1;
				let textEnd = -1;
				let seenFirstMark = false;

				if (cur.firstChild()) {
					do {
						if (cur.name === "LinkMark") {
							if (!seenFirstMark) {
								textStart = cur.to;
								seenFirstMark = true;
							} else if (textEnd < 0) {
								textEnd = cur.from;
							}
						}
					} while (cur.nextSibling());
				}

				if (
					textStart >= 0 &&
					textEnd >= 0 &&
					textStart < textEnd
				) {
					decorations.push(
						REPLACE.range(node.from, textStart),
					);
					decorations.push(REPLACE.range(textEnd, node.to));
					decorations.push(
						MARK_LINK.range(textStart, textEnd),
					);
				}
				return;
			}

			// --- Wikilinks ---
			if (name === "WikiLink") {
				if (active) return;
				const cur = node.node.cursor();
				const marks: { from: number; to: number }[] = [];
				if (cur.firstChild()) {
					do {
						if (cur.name === "WikiLinkMark") {
							marks.push({ from: cur.from, to: cur.to });
						}
					} while (cur.nextSibling());
				}
				if (marks.length >= 2) {
					for (const m of marks) {
						decorations.push(REPLACE.range(m.from, m.to));
					}
					decorations.push(
						MARK_WIKILINK.range(
							marks[0].to,
							marks[marks.length - 1].from,
						),
					);
				}
				return;
			}

			// --- Inline code ---
			if (name === "InlineCode") {
				if (active) return false;
				const cur = node.node.cursor();
				const marks: { from: number; to: number }[] = [];
				if (cur.firstChild()) {
					do {
						if (cur.name === "CodeMark") {
							marks.push({ from: cur.from, to: cur.to });
						}
					} while (cur.nextSibling());
				}
				if (marks.length >= 2) {
					const contentFrom = marks[0].to;
					const contentTo = marks[marks.length - 1].from;
					if (contentFrom < contentTo) {
						for (const m of marks) {
							decorations.push(REPLACE.range(m.from, m.to));
						}
						decorations.push(
							MARK_INLINE_CODE.range(contentFrom, contentTo),
						);
					}
				}
				return false;
			}

			// --- Fenced code blocks ---
			if (name === "FencedCode") {
				if (active) return false;
				const startLine = doc.lineAt(node.from);
				const endLine = doc.lineAt(node.to);
				for (let n = startLine.number; n <= endLine.number; n++) {
					const isFence =
						n === startLine.number || n === endLine.number;
					decorations.push(
						(isFence
							? LINE_CODE_FENCE
							: LINE_CODE_BLOCK
						).range(doc.line(n).from),
					);
				}
				return false;
			}

			// --- Blockquotes ---
			if (name === "Blockquote") {
				if (active) return;
				const startLine = doc.lineAt(node.from);
				const endLine = doc.lineAt(node.to);
				for (let n = startLine.number; n <= endLine.number; n++) {
					const line = doc.line(n);
					decorations.push(LINE_BLOCKQUOTE.range(line.from));
					const prefixLen = blockquotePrefixLen(
						doc.sliceString(line.from, line.to),
					);
					if (prefixLen > 0) {
						decorations.push(
							REPLACE.range(line.from, line.from + prefixLen),
						);
					}
				}
				return;
			}

			// --- Horizontal rules ---
			if (name === "HorizontalRule") {
				if (active) return false;
				decorations.push(
					LINE_HR.range(doc.lineAt(node.from).from),
				);
				decorations.push(REPLACE.range(node.from, node.to));
				return false;
			}
		},
	});

	return Decoration.set(decorations, true);
}

// StateField owns decorations so CM6 accounts for them at all document
// positions in its height map.  This prevents invisible-text bugs caused
// by line decorations (headings with larger font-size / padding) being
// absent outside the viewport when a ViewPlugin provides them.
const inlineDecoField = StateField.define<DecorationSet>({
	create(state) {
		return buildDecorations(state);
	},
	update(decos, tr) {
		// Rebuild when content, selection, or the incrementally-parsed
		// syntax tree changes.  Scroll is free — no rebuild needed.
		if (
			tr.docChanged ||
			tr.selection ||
			syntaxTree(tr.state) !== syntaxTree(tr.startState)
		) {
			return buildDecorations(tr.state);
		}
		return decos;
	},
	provide: (f) => EditorView.decorations.from(f),
});

// ViewPlugin retained only for event handlers (link clicks, meta-key tracking).
const inlineRenderingPlugin = ViewPlugin.fromClass(
	class {
		constructor(_view: EditorView) {}
	},
	{
	eventHandlers: {
		mousedown(event: MouseEvent, view: EditorView) {
			if (!event.metaKey || event.button !== 0) return false;
			const pos = view.posAtCoords({
				x: event.clientX,
				y: event.clientY,
			});
			if (pos === null) return false;

			// Wikilinks
			const wikiTarget = getWikilinkTarget(view, pos);
			if (wikiTarget) {
				event.preventDefault();
				const navigate = view.state.facet(wikilinkNavigateFacet);
				navigate(wikiTarget);
				return true;
			}

			// External links
			const url = getLinkUrl(view, pos);
			if (!url) return false;
			event.preventDefault();
			openUrl(url);
			return true;
		},
		keydown(event: KeyboardEvent, view: EditorView) {
			if (event.key === "Meta") {
				view.contentDOM.classList.add("cm-meta-held");
			}
		},
		keyup(event: KeyboardEvent, view: EditorView) {
			if (event.key === "Meta") {
				view.contentDOM.classList.remove("cm-meta-held");
			}
		},
	},
});

const inlineRenderingTheme = EditorView.baseTheme({
	// Minor third scale (1.2): title 2.488 → H1 2.074 → H2 1.728 → H3 1.44 → H4 1.2 → H5 1 → H6 0.833
	".cm-md-heading": {
		fontWeight: "700",
	},
	".cm-md-h1": {
		fontSize: "2.074em",
		lineHeight: "1.15",
		letterSpacing: "-0.02em",
		paddingTop: "0.75em !important",
		paddingBottom: "0.15em !important",
	},
	".cm-md-h2": {
		fontSize: "1.728em",
		lineHeight: "1.2",
		letterSpacing: "-0.015em",
		paddingTop: "0.6em !important",
		paddingBottom: "0.1em !important",
	},
	".cm-md-h3": {
		fontSize: "1.44em",
		lineHeight: "1.25",
		letterSpacing: "-0.01em",
		fontWeight: "600",
		paddingTop: "0.5em !important",
	},
	".cm-md-h4": {
		fontSize: "1.2em",
		lineHeight: "1.35",
		fontWeight: "600",
		paddingTop: "0.4em !important",
	},
	".cm-md-h5": {
		fontSize: "1em",
		fontWeight: "600",
		letterSpacing: "0.01em",
		paddingTop: "0.3em !important",
	},
	".cm-md-h6": {
		fontSize: "1em",
		fontWeight: "600",
		letterSpacing: "0.025em",
		paddingTop: "0.3em !important",
	},

	".cm-md-strong": {
		fontWeight: "700",
	},

	".cm-md-em": {
		fontStyle: "italic",
	},

	".cm-md-link": {
		color: "var(--link-color, var(--accent-color, #4078f2))",
		textDecoration: "underline",
		textDecorationColor: "transparent",
		textUnderlineOffset: "2px",
		textDecorationThickness: "1.5px",
		borderRadius: "2px",
		transition: "text-decoration-color 150ms ease, background-color 150ms ease",
	},

	".cm-meta-held .cm-md-link:hover": {
		cursor: "pointer",
		textDecorationColor: "var(--link-color, var(--accent-color, #4078f2))",
		backgroundColor:
			"color-mix(in srgb, var(--link-color, var(--accent-color, #4078f2)) 8%, transparent)",
	},

	".cm-md-wikilink": {
		color: "var(--link-color, var(--accent-color, #4078f2))",
		textDecoration: "underline",
		textDecorationColor: "transparent",
		textUnderlineOffset: "2px",
		textDecorationThickness: "1.5px",
		borderRadius: "2px",
		transition: "text-decoration-color 150ms ease, background-color 150ms ease",
	},

	".cm-meta-held .cm-md-wikilink:hover": {
		cursor: "pointer",
		textDecorationColor: "var(--link-color, var(--accent-color, #4078f2))",
		backgroundColor:
			"color-mix(in srgb, var(--link-color, var(--accent-color, #4078f2)) 8%, transparent)",
	},

	".cm-md-inline-code": {
		fontFamily:
			"'SF Mono', 'Fira Code', 'Cascadia Code', 'JetBrains Mono', monospace",
		fontSize: "0.88em",
		backgroundColor: "var(--code-bg, rgba(128, 128, 128, 0.12))",
		borderRadius: "3px",
		padding: "1px 4px",
	},

	".cm-md-code-block": {
		backgroundColor: "var(--code-block-bg, rgba(128, 128, 128, 0.06))",
	},
	".cm-md-code-fence": {
		opacity: "0.35",
		fontSize: "0.85em",
	},

	".cm-md-blockquote": {
		borderLeft: "3px solid var(--border-color, rgba(128, 128, 128, 0.35))",
		paddingLeft: "1em !important",
		opacity: "0.85",
	},

	".cm-md-hr-line": {
		borderBottom: "1px solid var(--border-color, rgba(128, 128, 128, 0.25))",
		lineHeight: "0 !important",
		padding: "0.75em 0 !important",
	},
});

/**
 * A highlight style based on defaultHighlightStyle but with underlines
 * removed from headings and links — we handle those via decorations.
 */
const highlightStyle = HighlightStyle.define([
	{ tag: tags.meta, color: "#404740" },
	{ tag: tags.link, color: "var(--link-color, var(--accent-color, #4078f2))" },
	{ tag: tags.heading, fontWeight: "bold" },
	{ tag: tags.emphasis, fontStyle: "italic" },
	{ tag: tags.strong, fontWeight: "bold" },
	{ tag: tags.strikethrough, textDecoration: "line-through" },
	{ tag: tags.keyword, color: "#708" },
	{
		tag: [
			tags.atom,
			tags.bool,
			tags.url,
			tags.contentSeparator,
			tags.labelName,
		],
		color: "#219",
	},
	{ tag: [tags.literal, tags.inserted], color: "#164" },
	{ tag: [tags.string, tags.deleted], color: "#a11" },
	{
		tag: [tags.regexp, tags.escape, tags.special(tags.string)],
		color: "#e40",
	},
	{ tag: tags.definition(tags.variableName), color: "#00f" },
	{ tag: tags.local(tags.variableName), color: "#30a" },
	{ tag: [tags.typeName, tags.namespace], color: "#085" },
	{ tag: tags.className, color: "#167" },
	{ tag: [tags.special(tags.variableName), tags.macroName], color: "#256" },
	{ tag: tags.definition(tags.propertyName), color: "#00c" },
	{ tag: tags.comment, color: "#940" },
	{ tag: tags.invalid, color: "#f00" },
]);

export function inlineRendering(): Extension {
	return [
		inlineDecoField,
		inlineRenderingPlugin,
		inlineRenderingTheme,
		syntaxHighlighting(highlightStyle, { fallback: true }),
	];
}
