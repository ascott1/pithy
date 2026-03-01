<script lang="ts">
	import { onMount } from "svelte";
	import { EditorView, keymap, type KeyBinding } from "@codemirror/view";
	import { Compartment, EditorSelection, EditorState, type Extension } from "@codemirror/state";
	import {
		defaultKeymap,
		history,
		historyKeymap,
		indentMore,
		indentLess,
	} from "@codemirror/commands";
	import { markdown } from "@codemirror/lang-markdown";
	import { languages } from "@codemirror/language-data";
	import {
		syntaxHighlighting,
		defaultHighlightStyle,
	} from "@codemirror/language";
	import {
		search,
		searchKeymap,
		SearchQuery,
		getSearchQuery,
		setSearchQuery,
		findNext,
		findPrevious,
		replaceNext,
		replaceAll,
		closeSearchPanel,
	} from "@codemirror/search";
	import type { Panel } from "@codemirror/view";
	import { inlineRendering, vaultRootFacet } from "$lib/editor/inlineRendering";
	import {
		wikilinkExtension,
		wikilinks,
		type FileEntry,
	} from "$lib/editor/wikilink";

	interface Props {
		doc: string;
		lang?: Extension;
		title?: string;
		titleDisabled?: boolean;
		dirty?: boolean;
		saving?: boolean;
		saveError?: string | null;
		renameError?: string | null;
		onchange?: (doc: string) => void;
		onsave?: () => void;
		onready?: (api: { focus: () => void; focusTitle: () => void; insertAtCoords: (text: string, coords: { x: number; y: number }) => boolean; insertAtCursor: (text: string) => void }) => void;
		ontitlechange?: (value: string) => void;
		ontitleblur?: () => void;
		ontitlekeydown?: (e: KeyboardEvent) => void;
		fileStems?: FileEntry[];
		onnavigate?: (target: string) => void;
		vaultRoot?: string;
	}

	let {
		doc,
		lang,
		title = "",
		titleDisabled = false,
		dirty = false,
		saving = false,
		saveError = null,
		renameError = null,
		onchange,
		onsave,
		onready,
		ontitlechange,
		ontitleblur,
		ontitlekeydown,
		fileStems = [],
		onnavigate,
		vaultRoot = "",
	}: Props = $props();

	const wikilinkCompartment = new Compartment();

	let container: HTMLDivElement;
	let view: EditorView | undefined = $state();
	let applyingExternal = false;
	let titleInputEl: HTMLInputElement | null = null;
	let dirtyEl: HTMLSpanElement | null = null;
	let errorEl: HTMLDivElement | null = null;
	let saveErrorEl: HTMLDivElement | null = null;

	const theme = EditorView.theme({
		"&": {
			height: "100%",
			backgroundColor: "var(--editor-bg)",
			color: "var(--editor-text)",
		},
		"&.cm-focused": {
			outline: "none",
		},
		".cm-scroller": {
			display: "flex",
			flexDirection: "column",
			overflow: "auto",
			fontFamily: "var(--editor-font-family)",
			fontSize: "var(--editor-font-size)",
			lineHeight: "var(--editor-line-height)",
		},
		".cm-content": {
			// Override CM6's flex-grow:2 which, in our column-flex scroller,
			// caps height at the viewport instead of expanding to content.
			// Without this, contentDOM.getBoundingClientRect() reports a
			// small fixed box that scrolls offscreen, and CM6's
			// visiblePixelRange() sees zero visible content.
			flex: "0 0 auto",
			boxSizing: "border-box",
			maxWidth: "var(--content-max-width)",
			width: "100%",
			margin: "0 auto",
			padding: "0 2em 2em",
			caretColor: "var(--editor-cursor)",
		},
		".cm-line": {
			padding: "0",
		},
		".cm-cursor, .cm-dropCursor": {
			borderLeftColor: "var(--editor-cursor)",
		},
		"&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
			backgroundColor: "var(--editor-selection) !important",
		},
		".cm-activeLine": {
			backgroundColor: "transparent",
		},
		".cm-gutters": {
			display: "none",
		},
	});

	function focusTitleInput() {
		if (titleInputEl) {
			titleInputEl.focus();
			titleInputEl.setSelectionRange(
				titleInputEl.value.length,
				titleInputEl.value.length,
			);
		}
	}

	function toggleInlineDelimiter(view: EditorView, delimiter: string): boolean {
		const { state } = view;
		const len = delimiter.length;
		const changes = state.changeByRange((range) => {
			const text = state.sliceDoc(range.from, range.to);
			// Check if selection itself is wrapped
			if (text.startsWith(delimiter) && text.endsWith(delimiter) && text.length >= len * 2) {
				const inner = text.slice(len, -len);
				return {
					changes: { from: range.from, to: range.to, insert: inner },
					range: EditorSelection.range(range.from, range.from + inner.length),
				};
			}
			// Check surrounding context (cursor inside delimiters, or selection surrounded)
			const before = state.sliceDoc(Math.max(0, range.from - len), range.from);
			const after = state.sliceDoc(range.to, Math.min(state.doc.length, range.to + len));
			if (before === delimiter && after === delimiter) {
				return {
					changes: [
						{ from: range.from - len, to: range.from, insert: "" },
						{ from: range.to, to: range.to + len, insert: "" },
					],
					range: EditorSelection.range(range.from - len, range.to - len),
				};
			}
			// Wrap
			const wrapped = delimiter + text + delimiter;
			return {
				changes: { from: range.from, to: range.to, insert: wrapped },
				range: EditorSelection.range(range.from + len, range.from + len + text.length),
			};
		});
		view.dispatch(state.update(changes, { userEvent: "input" }));
		return true;
	}

	function toggleCodeBlock(view: EditorView): boolean {
		const { state } = view;
		const range = state.selection.main;
		const fromLine = state.doc.lineAt(range.from);
		const toLine = state.doc.lineAt(range.to);

		// Check if already in a code block (line before starts with ``` and line after starts with ```)
		const lineBefore = fromLine.number > 1 ? state.doc.line(fromLine.number - 1) : null;
		const lineAfter = toLine.number < state.doc.lines ? state.doc.line(toLine.number + 1) : null;

		if (lineBefore?.text.startsWith("```") && lineAfter?.text.startsWith("```")) {
			// Remove the fence lines
			view.dispatch({
				changes: [
					{ from: lineAfter.from - 1, to: lineAfter.to },
					{ from: lineBefore.from, to: lineBefore.to + 1 },
				],
				userEvent: "input",
			});
		} else {
			// Wrap selection in fences
			const openFence = "```\n";
			const closeFence = "\n```";
			view.dispatch({
				changes: { from: fromLine.from, to: toLine.to, insert: openFence + state.sliceDoc(fromLine.from, toLine.to) + closeFence },
				selection: { anchor: fromLine.from + openFence.length, head: toLine.to + openFence.length },
				userEvent: "input",
			});
		}
		return true;
	}

	const markdownKeymap: KeyBinding[] = [
		{ key: "Mod-b", run: (v: EditorView) => toggleInlineDelimiter(v, "**") },
		{ key: "Mod-i", run: (v: EditorView) => toggleInlineDelimiter(v, "*") },
		{ key: "Mod-e", run: (v: EditorView) => toggleInlineDelimiter(v, "`") },
		{ key: "Mod-Shift-x", run: (v: EditorView) => toggleInlineDelimiter(v, "~~") },
		{ key: "Mod-Shift-c", run: toggleCodeBlock },
		{ key: "Tab", run: indentMore },
		{ key: "Shift-Tab", run: indentLess },
	];

	function createSearchPanel(view: EditorView): Panel {
		const dom = document.createElement("div");
		dom.className = "cm-search-bar";

		// State
		let caseSensitive = false;
		let wholeWord = false;
		let regexp = false;
		let replaceVisible = false;

		// -- Replace toggle arrow --
		const replaceToggle = document.createElement("button");
		replaceToggle.className = "cm-search-bar-toggle-replace";
		replaceToggle.textContent = "\u25B6"; // ▶
		replaceToggle.title = "Toggle replace";
		replaceToggle.addEventListener("click", () => {
			replaceVisible = !replaceVisible;
			replaceToggle.textContent = replaceVisible ? "\u25BC" : "\u25B6"; // ▼ or ▶
			replaceRow.style.display = replaceVisible ? "" : "none";
			if (replaceVisible) replaceInput.focus();
		});

		// -- Search row --
		const searchRow = document.createElement("div");
		searchRow.className = "cm-search-bar-row";

		const searchInput = document.createElement("input");
		searchInput.className = "cm-search-bar-input";
		searchInput.type = "text";
		searchInput.placeholder = "Find\u2026";
		searchInput.setAttribute("main-field", "true");

		function dispatchQuery() {
			const q = new SearchQuery({
				search: searchInput.value,
				caseSensitive,
				wholeWord,
				regexp,
				replace: replaceInput.value,
			});
			view.dispatch({ effects: setSearchQuery.of(q) });
		}

		searchInput.addEventListener("input", dispatchQuery);
		searchInput.addEventListener("keydown", (e) => {
			if (e.key === "Enter") {
				e.preventDefault();
				if (e.shiftKey) findPrevious(view);
				else findNext(view);
			}
			if (e.key === "Escape") {
				e.preventDefault();
				closeSearchPanel(view);
				view.focus();
			}
		});

		// -- Toggle buttons --
		function makeToggle(label: string, title: string, getter: () => boolean, setter: (v: boolean) => void) {
			const btn = document.createElement("button");
			btn.className = "cm-search-bar-btn cm-search-bar-toggle";
			btn.textContent = label;
			btn.title = title;
			btn.addEventListener("click", () => {
				setter(!getter());
				btn.classList.toggle("active", getter());
				dispatchQuery();
			});
			return btn;
		}

		const caseBtn = makeToggle("Aa", "Case sensitive", () => caseSensitive, (v) => { caseSensitive = v; });
		const wordBtn = makeToggle("wd", "Whole word", () => wholeWord, (v) => { wholeWord = v; });
		const regexBtn = makeToggle(".*", "Regular expression", () => regexp, (v) => { regexp = v; });

		const toggleGroup = document.createElement("div");
		toggleGroup.className = "cm-search-bar-group";
		toggleGroup.append(caseBtn, wordBtn, regexBtn);

		// -- Separator --
		const sep = document.createElement("span");
		sep.className = "cm-search-bar-sep";

		// -- Nav buttons --
		const prevBtn = document.createElement("button");
		prevBtn.className = "cm-search-bar-btn cm-search-bar-nav";
		prevBtn.textContent = "\u2039"; // ‹
		prevBtn.title = "Previous match (Shift+Enter)";
		prevBtn.addEventListener("click", () => findPrevious(view));

		const nextBtn = document.createElement("button");
		nextBtn.className = "cm-search-bar-btn cm-search-bar-nav";
		nextBtn.textContent = "\u203A"; // ›
		nextBtn.title = "Next match (Enter)";
		nextBtn.addEventListener("click", () => findNext(view));

		const navGroup = document.createElement("div");
		navGroup.className = "cm-search-bar-group";
		navGroup.append(prevBtn, nextBtn);

		// -- Match count --
		const matchCount = document.createElement("span");
		matchCount.className = "cm-search-bar-count";
		matchCount.textContent = "0/0";

		// -- Close button --
		const closeBtn = document.createElement("button");
		closeBtn.className = "cm-search-bar-btn cm-search-bar-close";
		closeBtn.textContent = "\u00D7"; // ×
		closeBtn.title = "Close (Escape)";
		closeBtn.addEventListener("click", () => {
			closeSearchPanel(view);
			view.focus();
		});

		searchRow.append(searchInput, toggleGroup, sep, navGroup, matchCount, closeBtn);

		// -- Replace row --
		const replaceRow = document.createElement("div");
		replaceRow.className = "cm-search-bar-row cm-search-bar-replace-row";
		replaceRow.style.display = "none";

		const replaceInput = document.createElement("input");
		replaceInput.className = "cm-search-bar-input";
		replaceInput.type = "text";
		replaceInput.placeholder = "Replace\u2026";
		replaceInput.addEventListener("input", dispatchQuery);
		replaceInput.addEventListener("keydown", (e) => {
			if (e.key === "Enter") {
				e.preventDefault();
				replaceNext(view);
			}
			if (e.key === "Escape") {
				e.preventDefault();
				closeSearchPanel(view);
				view.focus();
			}
		});

		const replaceBtn = document.createElement("button");
		replaceBtn.className = "cm-search-bar-btn";
		replaceBtn.textContent = "Replace";
		replaceBtn.addEventListener("click", () => replaceNext(view));

		const replaceAllBtn = document.createElement("button");
		replaceAllBtn.className = "cm-search-bar-btn";
		replaceAllBtn.textContent = "All";
		replaceAllBtn.addEventListener("click", () => replaceAll(view));

		const replaceActions = document.createElement("div");
		replaceActions.className = "cm-search-bar-group";
		replaceActions.append(replaceBtn, replaceAllBtn);

		replaceRow.append(replaceInput, replaceActions);

		const rowsWrapper = document.createElement("div");
		rowsWrapper.className = "cm-search-bar-rows";
		rowsWrapper.append(searchRow, replaceRow);
		dom.append(replaceToggle, rowsWrapper);

		function updateMatchCount() {
			const query = getSearchQuery(view.state);
			if (!query.valid || !query.search) {
				matchCount.textContent = "0/0";
				return;
			}
			const cursor = query.getCursor(view.state.doc);
			let total = 0;
			let currentIdx = 0;
			const sel = view.state.selection.main;
			let found = false;
			let iter = cursor.next();
			while (!iter.done) {
				total++;
				if (!found && iter.value.from === sel.from && iter.value.to === sel.to) {
					currentIdx = total;
					found = true;
				}
				iter = cursor.next();
			}
			matchCount.textContent = found ? `${currentIdx}/${total}` : `${total > 0 ? "?" : 0}/${total}`;
		}

		// Sync input from external query state (e.g. selection-based search)
		function syncFromState() {
			const query = getSearchQuery(view.state);
			if (searchInput.value !== query.search) searchInput.value = query.search;
			if (replaceInput.value !== query.replace) replaceInput.value = query.replace;
			if (caseSensitive !== query.caseSensitive) {
				caseSensitive = query.caseSensitive;
				caseBtn.classList.toggle("active", caseSensitive);
			}
			if (wholeWord !== query.wholeWord) {
				wholeWord = query.wholeWord;
				wordBtn.classList.toggle("active", wholeWord);
			}
			if (regexp !== query.regexp) {
				regexp = query.regexp;
				regexBtn.classList.toggle("active", regexp);
			}
		}

		return {
			dom,
			top: true,
			mount() {
				syncFromState();
				updateMatchCount();
				searchInput.focus();
				searchInput.select();
			},
			update(update) {
				if (update.docChanged || update.selectionSet || update.transactions.some(
					(tr) => tr.effects.some((e) => e.is(setSearchQuery))
				)) {
					syncFromState();
					updateMatchCount();
				}
			},
		};
	}

	onMount(() => {
		const state = EditorState.create({
			doc,
			extensions: [
				EditorView.lineWrapping,
				history(),
				lang ??
					markdown({
						codeLanguages: languages,
						extensions: [wikilinkExtension],
					}),
				...(lang
					? [syntaxHighlighting(defaultHighlightStyle, { fallback: true })]
					: [
							EditorView.contentAttributes.of({ spellcheck: "true" }),
							vaultRootFacet.of(vaultRoot),
							inlineRendering(),
							wikilinkCompartment.of(
								wikilinks({
									files: fileStems ?? [],
									onNavigate: (t) => onnavigate?.(t),
								}),
							),
						]),
				search({ top: true, createPanel: createSearchPanel }),
				keymap.of([
					...(lang ? [] : markdownKeymap),
					...defaultKeymap,
					...historyKeymap,
					...searchKeymap,
					{
						key: "Mod-s",
						run: () => {
							onsave?.();
							return true;
						},
					},
					{
						key: "ArrowUp",
						run: (v) => {
							const sel = v.state.selection.main;
							if (!sel.empty || v.composing) return false;
							if (v.state.doc.lineAt(sel.head).number === 1) {
								focusTitleInput();
								return true;
							}
							return false;
						},
					},
				]),
				EditorView.updateListener.of((update) => {
					if (update.docChanged && !applyingExternal) {
						onchange?.(update.state.doc.toString());
					}
				}),
				EditorView.domEventHandlers({
					dragover(event) {
						if (event.dataTransfer?.types.includes("Files")) {
							event.preventDefault();
							event.dataTransfer.dropEffect = "copy";
						}
					},
					drop(event) {
						// Only suppress default for image files; let non-images fall through
						const files = Array.from(event.dataTransfer?.files ?? []);
						const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico"]);
						const hasImages = files.some((f) => {
							const ext = f.name.split(".").pop()?.toLowerCase() ?? "";
							return IMAGE_EXTS.has(ext);
						});
						if (hasImages) {
							event.preventDefault();
						}
					},
				}),
				theme,
			],
		});

		view = new EditorView({ state, parent: container });

		// Inject title into CM scroller (Obsidian-style inline title)
		const scroller = view.scrollDOM;
		const content = view.contentDOM;
		if (scroller && content && !scroller.querySelector(":scope > .cm-title-wrapper")) {
			const wrapper = document.createElement("div");
			wrapper.className = "cm-title-wrapper";

			const input = document.createElement("input");
			input.className = "cm-note-title";
			input.type = "text";
			input.spellcheck = false;
			input.placeholder = "Untitled";
			input.value = title;
			input.disabled = titleDisabled;
			input.addEventListener("input", (e) =>
				ontitlechange?.((e.target as HTMLInputElement).value),
			);
			input.addEventListener("blur", () => ontitleblur?.());
			input.addEventListener("keydown", (e) => ontitlekeydown?.(e));

			const indicator = document.createElement("span");
			indicator.className = "cm-dirty-indicator";
			indicator.textContent = "●";
			indicator.title = "Unsaved changes";
			indicator.style.display = dirty ? "" : "none";
			if (saving) indicator.classList.add("cm-saving");

			wrapper.appendChild(input);
			wrapper.appendChild(indicator);

			const saveErrDiv = document.createElement("div");
			saveErrDiv.className = "cm-title-error cm-save-error";
			saveErrDiv.style.display = saveError ? "" : "none";
			saveErrDiv.textContent = saveError ? `Save failed: ${saveError}` : "";

			const errDiv = document.createElement("div");
			errDiv.className = "cm-title-error";
			errDiv.style.display = renameError ? "" : "none";
			errDiv.textContent = renameError ?? "";

			scroller.insertBefore(saveErrDiv, content);
			scroller.insertBefore(errDiv, saveErrDiv);
			scroller.insertBefore(wrapper, errDiv);

			titleInputEl = input;
			dirtyEl = indicator;
			errorEl = errDiv;
			saveErrorEl = saveErrDiv;
		}

		onready?.({
			focus: () => {
				view?.focus();
				view?.dispatch({ selection: { anchor: 0 } });
			},
			focusTitle: () => focusTitleInput(),
			insertAtCoords: (text: string, coords: { x: number; y: number }) => {
				if (!view) return false;
				const pos = view.posAtCoords(coords);
				if (pos == null) return false;
				view.dispatch({ changes: { from: pos, insert: text } });
				return true;
			},
			insertAtCursor: (text: string) => {
				if (!view) return;
				const pos = view.state.selection.main.head;
				view.dispatch({ changes: { from: pos, insert: text } });
				view.focus();
			},
		});

		return () => {
			titleInputEl = dirtyEl = errorEl = saveErrorEl = null;
			view?.destroy();
		};
	});

	$effect(() => {
		if (titleInputEl && titleInputEl.value !== title) {
			titleInputEl.value = title;
		}
	});

	$effect(() => {
		if (titleInputEl) {
			titleInputEl.disabled = titleDisabled;
		}
	});

	$effect(() => {
		if (dirtyEl) {
			dirtyEl.style.display = dirty ? "" : "none";
			dirtyEl.classList.toggle("cm-saving", saving);
			dirtyEl.title = saving ? "Saving…" : "Unsaved changes";
		}
	});

	$effect(() => {
		if (saveErrorEl) {
			saveErrorEl.style.display = saveError ? "" : "none";
			saveErrorEl.textContent = saveError ? `Save failed: ${saveError}` : "";
		}
	});

	$effect(() => {
		if (errorEl) {
			errorEl.style.display = renameError ? "" : "none";
			errorEl.textContent = renameError ?? "";
		}
	});

	$effect(() => {
		if (view && doc !== view.state.doc.toString()) {
			applyingExternal = true;
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: doc },
			});
			applyingExternal = false;
		}
	});

	$effect(() => {
		if (view && !lang && fileStems) {
			view.dispatch({
				effects: wikilinkCompartment.reconfigure(
					wikilinks({
						files: fileStems,
						onNavigate: (t) => onnavigate?.(t),
					}),
				),
			});
		}
	});
</script>

<div bind:this={container} class="editor-container"></div>

<style>
	.editor-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.editor-container :global(.cm-editor) {
		flex: 1;
	}

	.editor-container :global(.cm-title-wrapper) {
		position: relative;
		flex: 0 0 auto;
		box-sizing: border-box;
		max-width: var(--content-max-width);
		width: 100%;
		margin: 0 auto;
		padding: 2em 2em 0;
	}

	.editor-container :global(.cm-note-title) {
		display: block;
		width: 100%;
		font: inherit;
		font-size: 2.488em;
		font-weight: 700;
		line-height: 1.1;
		letter-spacing: -0.025em;
		color: var(--editor-text);
		background: transparent;
		border: none;
		outline: none;
		margin: 0;
		padding: 0 0 0.3em;
	}

	.editor-container :global(.cm-note-title::placeholder) {
		color: var(--editor-text);
		opacity: 0.25;
	}

	.editor-container :global(.cm-note-title:disabled) {
		opacity: 1;
	}

	.editor-container :global(.cm-dirty-indicator) {
		position: absolute;
		right: 2em;
		top: 2.5em;
		font-size: 0.5em;
		color: var(--dirty-color);
		transition: opacity 0.15s;
	}

	.editor-container :global(.cm-dirty-indicator.cm-saving) {
		animation: pulse 1s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}

	.editor-container :global(.cm-title-error) {
		max-width: var(--content-max-width);
		margin: 0 auto;
		padding: 0 2em 0.5em;
		font-size: 0.8em;
		color: var(--error-color, #d14343);
	}

	/* Custom search bar */
	.editor-container :global(.cm-search-bar) {
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
		font-size: 0.8125rem;
		background: color-mix(in srgb, var(--editor-bg) 95%, var(--editor-text));
		border-bottom: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
		padding: 5px 8px;
		display: flex;
		align-items: flex-start;
		gap: 0;
	}

	.editor-container :global(.cm-search-bar-toggle-replace) {
		flex: 0 0 auto;
		background: none;
		border: none;
		color: var(--editor-text);
		opacity: 0.45;
		font-size: 0.6rem;
		cursor: pointer;
		padding: 6px 4px 6px 2px;
		line-height: 1;
	}

	.editor-container :global(.cm-search-bar-toggle-replace:hover) {
		opacity: 0.8;
	}

	.editor-container :global(.cm-search-bar-rows) {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		gap: 4px;
	}

	.editor-container :global(.cm-search-bar-row) {
		display: flex;
		align-items: center;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}

	.editor-container :global(.cm-search-bar-replace-row) {
		padding-left: 0;
	}

	.editor-container :global(.cm-search-bar-input) {
		flex: 1;
		min-width: 80px;
		font-family: inherit;
		font-size: 0.8125rem;
		color: var(--editor-text);
		background: var(--editor-bg);
		border: 1px solid color-mix(in srgb, var(--editor-text) 15%, transparent);
		border-radius: 5px;
		padding: 3px 8px;
		outline: none;
	}

	.editor-container :global(.cm-search-bar-input:focus) {
		border-color: var(--accent-color);
	}

	.editor-container :global(.cm-search-bar-input::placeholder) {
		color: var(--editor-text);
		opacity: 0.35;
	}

	.editor-container :global(.cm-search-bar-group) {
		display: flex;
		align-items: center;
		gap: 2px;
		flex: 0 0 auto;
	}

	.editor-container :global(.cm-search-bar-btn) {
		font-family: inherit;
		font-size: 0.75rem;
		color: var(--editor-text);
		background: color-mix(in srgb, var(--editor-text) 6%, transparent);
		border: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
		border-radius: 5px;
		padding: 2px 8px;
		cursor: pointer;
		line-height: 1.4;
		white-space: nowrap;
	}

	.editor-container :global(.cm-search-bar-btn:hover) {
		background: color-mix(in srgb, var(--editor-text) 12%, transparent);
	}

	.editor-container :global(.cm-search-bar-btn:active) {
		background: color-mix(in srgb, var(--editor-text) 18%, transparent);
	}

	.editor-container :global(.cm-search-bar-toggle) {
		font-weight: 600;
		min-width: 26px;
		text-align: center;
		opacity: 0.5;
	}

	.editor-container :global(.cm-search-bar-toggle.active) {
		opacity: 1;
		background: var(--accent-color);
		color: white;
		border-color: var(--accent-color);
	}

	.editor-container :global(.cm-search-bar-nav) {
		font-size: 1rem;
		font-weight: 300;
		padding: 0 6px;
		line-height: 1.2;
	}

	.editor-container :global(.cm-search-bar-sep) {
		width: 1px;
		height: 16px;
		background: color-mix(in srgb, var(--editor-text) 12%, transparent);
		margin: 0 4px;
		flex: 0 0 auto;
	}

	.editor-container :global(.cm-search-bar-count) {
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		color: var(--editor-text);
		opacity: 0.5;
		min-width: 32px;
		text-align: center;
		flex: 0 0 auto;
	}

	.editor-container :global(.cm-search-bar-close) {
		font-size: 1rem;
		padding: 0 4px;
		line-height: 1.2;
		opacity: 0.5;
	}

	.editor-container :global(.cm-search-bar-close:hover) {
		opacity: 1;
	}

	.editor-container :global(.cm-selectionMatch) {
		background: color-mix(in srgb, var(--accent-color) 20%, transparent);
	}

	.editor-container :global(.cm-searchMatch) {
		background: color-mix(in srgb, var(--dirty-color) 30%, transparent);
		border-radius: 2px;
	}

	.editor-container :global(.cm-searchMatch-selected) {
		background: color-mix(in srgb, var(--dirty-color) 55%, transparent);
	}
</style>
