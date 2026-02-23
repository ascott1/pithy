import {
	Decoration,
	type DecorationSet,
	EditorView,
	ViewPlugin,
	type ViewUpdate,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import type { Extension, Range, SelectionRange } from "@codemirror/state";

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

function buildDecorations(view: EditorView): DecorationSet {
	if (view.composing) return Decoration.none;

	const decorations: Range<Decoration>[] = [];
	const { state } = view;
	const selections = state.selection.ranges;
	const doc = state.doc;

	for (const { from, to } of view.visibleRanges) {
		syntaxTree(state).iterate({
			from,
			to,
			enter(node) {
				const active = selectionIntersects(
					node.from,
					node.to,
					selections,
				);

				// --- ATX Headings ---
				if (/^ATXHeading[1-6]$/.test(node.name)) {
					if (active) return;
					const level = node.name.charCodeAt(10) - 48;

					const startLine = doc.lineAt(node.from);
					const endLine = doc.lineAt(node.to);
					for (let n = startLine.number; n <= endLine.number; n++) {
						decorations.push(
							Decoration.line({
								class: `cm-md-heading cm-md-h${level}`,
							}).range(doc.line(n).from),
						);
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
								decorations.push(
									Decoration.replace({}).range(
										cur.from,
										end,
									),
								);
							}
						} while (cur.nextSibling());
					}
					return;
				}

				// --- Bold ---
				if (node.name === "StrongEmphasis") {
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
							decorations.push(
								Decoration.replace({}).range(m.from, m.to),
							);
						}
						decorations.push(
							Decoration.mark({ class: "cm-md-strong" }).range(
								marks[0].to,
								marks[marks.length - 1].from,
							),
						);
					}
					return;
				}

				// --- Italic ---
				if (node.name === "Emphasis") {
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
							decorations.push(
								Decoration.replace({}).range(m.from, m.to),
							);
						}
						decorations.push(
							Decoration.mark({ class: "cm-md-em" }).range(
								marks[0].to,
								marks[marks.length - 1].from,
							),
						);
					}
					return;
				}

				// --- Links ---
				if (node.name === "Link") {
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
							Decoration.replace({}).range(
								node.from,
								textStart,
							),
						);
						decorations.push(
							Decoration.replace({}).range(textEnd, node.to),
						);
						decorations.push(
							Decoration.mark({ class: "cm-md-link" }).range(
								textStart,
								textEnd,
							),
						);
					}
					return;
				}

				// --- Inline code ---
				if (node.name === "InlineCode") {
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
								decorations.push(
									Decoration.replace({}).range(
										m.from,
										m.to,
									),
								);
							}
							decorations.push(
								Decoration.mark({
									class: "cm-md-inline-code",
								}).range(contentFrom, contentTo),
							);
						}
					}
					return false;
				}

				// --- Fenced code blocks ---
				if (node.name === "FencedCode") {
					if (active) return false;
					const startLine = doc.lineAt(node.from);
					const endLine = doc.lineAt(node.to);
					for (let n = startLine.number; n <= endLine.number; n++) {
						const isFence =
							n === startLine.number || n === endLine.number;
						decorations.push(
							Decoration.line({
								class: isFence
									? "cm-md-code-block cm-md-code-fence"
									: "cm-md-code-block",
							}).range(doc.line(n).from),
						);
					}
					return false;
				}

				// --- Blockquotes ---
				if (node.name === "Blockquote") {
					if (active) return;
					const startLine = doc.lineAt(node.from);
					const endLine = doc.lineAt(node.to);
					for (let n = startLine.number; n <= endLine.number; n++) {
						const line = doc.line(n);
						decorations.push(
							Decoration.line({
								class: "cm-md-blockquote",
							}).range(line.from),
						);
						const text = doc.sliceString(line.from, line.to);
						const match = text.match(/^((?:> ?)+)/);
						if (match) {
							decorations.push(
								Decoration.replace({}).range(
									line.from,
									line.from + match[1].length,
								),
							);
						}
					}
					return;
				}

				// --- Horizontal rules ---
				if (node.name === "HorizontalRule") {
					if (active) return false;
					decorations.push(
						Decoration.line({ class: "cm-md-hr-line" }).range(
							doc.lineAt(node.from).from,
						),
					);
					decorations.push(
						Decoration.replace({}).range(node.from, node.to),
					);
					return false;
				}
			},
		});
	}

	return Decoration.set(decorations, true);
}

const inlineRenderingPlugin = ViewPlugin.fromClass(
	class {
		decorations: DecorationSet;
		constructor(view: EditorView) {
			this.decorations = buildDecorations(view);
		}
		update(update: ViewUpdate) {
			if (
				update.docChanged ||
				update.selectionSet ||
				update.viewportChanged
			) {
				this.decorations = buildDecorations(update.view);
			}
		}
	},
	{
		decorations: (v) => v.decorations,
	},
);

const inlineRenderingTheme = EditorView.baseTheme({
	".cm-md-heading": {
		fontWeight: "700",
		textDecoration: "none",
	},
	".cm-md-heading *": {
		textDecoration: "none !important",
	},
	".cm-md-h1": { fontSize: "1.5em", lineHeight: "1.2" },
	".cm-md-h2": { fontSize: "1.3em", lineHeight: "1.25" },
	".cm-md-h3": { fontSize: "1.15em", lineHeight: "1.3" },
	".cm-md-h4": { fontSize: "1.05em", lineHeight: "1.35" },
	".cm-md-h5": { fontSize: "1em" },
	".cm-md-h6": { fontSize: "0.95em", opacity: "0.85" },

	".cm-md-strong": {
		fontWeight: "700",
	},

	".cm-md-em": {
		fontStyle: "italic",
	},

	".cm-md-link": {
		color: "var(--accent-color, #4078f2)",
		textDecoration: "underline",
		textUnderlineOffset: "2px",
		textDecorationColor:
			"color-mix(in srgb, var(--accent-color, #4078f2) 40%, transparent)",
	},

	".cm-md-inline-code": {
		fontFamily:
			"'SF Mono', 'Fira Code', 'Cascadia Code', 'JetBrains Mono', monospace",
		fontSize: "0.88em",
		backgroundColor: "rgba(128, 128, 128, 0.12)",
		borderRadius: "3px",
		padding: "1px 4px",
	},

	".cm-md-code-block": {
		backgroundColor: "rgba(128, 128, 128, 0.06)",
	},
	".cm-md-code-fence": {
		opacity: "0.35",
		fontSize: "0.85em",
	},

	".cm-md-blockquote": {
		borderLeft: "3px solid rgba(128, 128, 128, 0.35)",
		paddingLeft: "1em !important",
		opacity: "0.85",
	},

	".cm-md-hr-line": {
		borderBottom: "1px solid rgba(128, 128, 128, 0.25)",
		lineHeight: "0 !important",
		padding: "0.75em 0 !important",
	},
});

export function inlineRendering(): Extension {
	return [inlineRenderingPlugin, inlineRenderingTheme];
}
