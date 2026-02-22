<script lang="ts">
	import { onMount } from "svelte";
	import { EditorView, keymap } from "@codemirror/view";
	import { EditorState } from "@codemirror/state";
	import {
		defaultKeymap,
		history,
		historyKeymap,
	} from "@codemirror/commands";
	import { markdown } from "@codemirror/lang-markdown";
	import { languages } from "@codemirror/language-data";
	import {
		syntaxHighlighting,
		defaultHighlightStyle,
	} from "@codemirror/language";

	interface Props {
		doc: string;
		title?: string;
		titleDisabled?: boolean;
		dirty?: boolean;
		renameError?: string | null;
		onchange?: (doc: string) => void;
		onsave?: () => void;
		onready?: (api: { focus: () => void; focusTitle: () => void }) => void;
		ontitlechange?: (value: string) => void;
		ontitleblur?: () => void;
		ontitlekeydown?: (e: KeyboardEvent) => void;
	}

	let {
		doc,
		title = "",
		titleDisabled = false,
		dirty = false,
		renameError = null,
		onchange,
		onsave,
		onready,
		ontitlechange,
		ontitleblur,
		ontitlekeydown,
	}: Props = $props();

	let container: HTMLDivElement;
	let view: EditorView | undefined = $state();
	let applyingExternal = false;
	let titleInputEl: HTMLInputElement | null = null;
	let dirtyEl: HTMLSpanElement | null = null;
	let errorEl: HTMLDivElement | null = null;

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
			overflow: "auto",
			flexWrap: "wrap",
			alignContent: "flex-start",
			fontFamily:
				'-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif',
			fontSize: "15px",
			lineHeight: "1.7",
		},
		".cm-content": {
			boxSizing: "border-box",
			maxWidth: "var(--content-max-width)",
			width: "100%",
			margin: "0 auto",
			padding: "0 2rem 2rem",
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

	onMount(() => {
		const state = EditorState.create({
			doc,
			extensions: [
				EditorView.lineWrapping,
				history(),
				markdown({ codeLanguages: languages }),
				syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
				keymap.of([
					...defaultKeymap,
					...historyKeymap,
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

			wrapper.appendChild(input);
			wrapper.appendChild(indicator);

			const errDiv = document.createElement("div");
			errDiv.className = "cm-title-error";
			errDiv.style.display = renameError ? "" : "none";
			errDiv.textContent = renameError ?? "";

			scroller.insertBefore(errDiv, content);
			scroller.insertBefore(wrapper, errDiv);

			titleInputEl = input;
			dirtyEl = indicator;
			errorEl = errDiv;
		}

		onready?.({
			focus: () => {
				view?.focus();
				view?.dispatch({ selection: { anchor: 0 } });
			},
			focusTitle: () => focusTitleInput(),
		});

		return () => {
			titleInputEl = dirtyEl = errorEl = null;
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
		flex: 0 0 100%;
		box-sizing: border-box;
		max-width: var(--content-max-width);
		width: 100%;
		margin: 0 auto;
		padding: 2rem 2rem 0;
	}

	.editor-container :global(.cm-note-title) {
		display: block;
		width: 100%;
		font: inherit;
		font-size: 1.8rem;
		font-weight: 700;
		line-height: 1.2;
		color: var(--editor-text);
		background: transparent;
		border: none;
		outline: none;
		margin: 0;
		padding: 0 0 0.5rem;
	}

	.editor-container :global(.cm-note-title::placeholder) {
		color: var(--editor-text);
		opacity: 0.25;
	}

	.editor-container :global(.cm-note-title:disabled) {
		opacity: 0.6;
	}

	.editor-container :global(.cm-dirty-indicator) {
		position: absolute;
		right: 2rem;
		top: 2.5rem;
		font-size: 0.5rem;
		color: var(--dirty-color);
	}

	.editor-container :global(.cm-title-error) {
		max-width: var(--content-max-width);
		margin: 0 auto;
		padding: 0 2rem 0.5rem;
		font-size: 0.8rem;
		color: #d14343;
	}
</style>
