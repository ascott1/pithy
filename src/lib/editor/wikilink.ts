import { type MarkdownConfig } from "@lezer/markdown";
import { tags } from "@lezer/highlight";
import { Facet } from "@codemirror/state";
import {
	autocompletion,
	type CompletionContext,
	type CompletionResult,
} from "@codemirror/autocomplete";
import type { Extension } from "@codemirror/state";
import { fuzzyScore } from "$lib/fuzzy";

// ── Lezer inline parser ──────────────────────────────────────────────

export const wikilinkExtension: MarkdownConfig = {
	defineNodes: [
		{ name: "WikiLink", style: tags.link },
		{ name: "WikiLinkMark", style: tags.processingInstruction },
	],
	parseInline: [
		{
			name: "WikiLink",
			before: "Link",
			parse(cx, _next, pos) {
				// Must start with [[
				if (
					cx.char(pos) !== 91 /* [ */ ||
					cx.char(pos + 1) !== 91 /* [ */
				)
					return -1;

				// Scan forward for ]]
				let end = pos + 2;
				const max = cx.end;
				while (end < max) {
					const ch = cx.char(end);
					if (ch === 10 /* \n */) return -1; // no newlines allowed
					if (ch === 93 /* ] */ && end + 1 < max && cx.char(end + 1) === 93) {
						// Found closing ]]
						// Build: WikiLinkMark[[ ... WikiLinkMark]]
						const children = [
							cx.elt("WikiLinkMark", pos, pos + 2),
							cx.elt("WikiLinkMark", end, end + 2),
						];
						cx.addElement(
							cx.elt("WikiLink", pos, end + 2, children),
						);
						return end + 2;
					}
					end++;
				}
				return -1;
			},
		},
	],
};

// ── Link resolution ──────────────────────────────────────────────────

export function normalizeForMatch(s: string): string {
	return s
		.toLowerCase()
		.replace(/[\s\-_]+/g, "-");
}

export interface FileEntry {
	path: string;
	stem: string;
}

export function resolveWikilink(
	target: string,
	files: FileEntry[],
): string | null {
	if (!target || !target.trim()) return null;
	const norm = normalizeForMatch(target);
	for (const f of files) {
		if (normalizeForMatch(f.stem) === norm) return f.path;
	}
	return null;
}

// ── Facets ────────────────────────────────────────────────────────────

export const wikilinkNavigateFacet = Facet.define<
	(target: string) => void,
	(target: string) => void
>({
	combine(values) {
		return values[values.length - 1] ?? (() => {});
	},
});

export const wikilinkFilesFacet = Facet.define<FileEntry[], FileEntry[]>({
	combine(values) {
		return values[values.length - 1] ?? [];
	},
});

// ── Autocomplete ─────────────────────────────────────────────────────

function wikilinkCompletions(
	context: CompletionContext,
): CompletionResult | null {
	// Scan backwards for unclosed [[
	const line = context.state.doc.lineAt(context.pos);
	const textBefore = line.text.slice(0, context.pos - line.from);
	const openIdx = textBefore.lastIndexOf("[[");
	if (openIdx < 0) return null;

	// Check there's no ]] between [[ and cursor
	const afterOpen = textBefore.slice(openIdx + 2);
	if (afterOpen.includes("]]")) return null;

	const from = line.from + openIdx + 2;
	const query = afterOpen;

	const files = context.state.facet(wikilinkFilesFacet);
	const scored: { stem: string; score: number }[] = [];

	for (const f of files) {
		if (query.length === 0) {
			scored.push({ stem: f.stem, score: 0 });
		} else {
			const s = fuzzyScore(query, f.stem);
			if (s !== null) scored.push({ stem: f.stem, score: s });
		}
	}

	scored.sort((a, b) => b.score - a.score);

	return {
		from,
		filter: false,
		options: scored.map((s) => ({
			label: s.stem,
			apply: s.stem + "]]",
		})),
	};
}

// ── Extension factory ────────────────────────────────────────────────

export function wikilinks(options: {
	files: FileEntry[];
	onNavigate: (target: string) => void;
}): Extension {
	return [
		wikilinkNavigateFacet.of(options.onNavigate),
		wikilinkFilesFacet.of(options.files),
		autocompletion({ override: [wikilinkCompletions] }),
	];
}
