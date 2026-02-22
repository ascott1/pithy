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
		onchange?: (doc: string) => void;
		onsave?: () => void;
		onfocusup?: () => void;
		onready?: (api: { focus: () => void }) => void;
	}

	let { doc, onchange, onsave, onfocusup, onready }: Props = $props();

	let container: HTMLDivElement;
	let view: EditorView | undefined = $state();
	let applyingExternal = false;

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
			overflow: "auto",
			fontFamily:
				'-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif',
			fontSize: "15px",
			lineHeight: "1.7",
		},
		".cm-content": {
			padding: "0 0 2rem",
			maxWidth: "65ch",
			margin: "0 auto",
			caretColor: "var(--editor-cursor)",
		},
		".cm-line": {
			padding: "0 2rem",
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
							const { head } = v.state.selection.main;
							if (v.state.doc.lineAt(head).number === 1) {
								onfocusup?.();
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

		onready?.({
			focus: () => {
				view?.focus();
				view?.dispatch({ selection: { anchor: 0 } });
			},
		});

		return () => view?.destroy();
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
</style>
