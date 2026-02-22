<script lang="ts">
	import { onMount } from "svelte";
	import MarkdownEditor from "$lib/editor/MarkdownEditor.svelte";
	import {
		listFiles,
		readFile,
		saveFile,
		renameFile,
		sanitizeFilename,
	} from "$lib/tauri/fs";

	let currentPath = $state<string | null>(null);
	let doc = $state("");
	let lastSavedDoc = $state("");
	let dirty = $derived(currentPath !== null && doc !== lastSavedDoc);

	let titleDraft = $state("");
	let isRenaming = $state(false);
	let renameError = $state<string | null>(null);
	let titleInput = $state<HTMLInputElement | null>(null);
	let editorApi: { focus: () => void } | null = null;

	let openSeq = 0;
	let renameSeq = 0;

	onMount(async () => {
		const files = await listFiles();
		if (files.length > 0) {
			await openFile(files[0]);
		}
	});

	function displayName(path: string): string {
		return path
			.replace(/\.md$/, "")
			.split("/")
			.pop()!
			.replaceAll("-", " ")
			.replaceAll("_", " ");
	}

	function dirname(relPath: string): string {
		const parts = relPath.split("/");
		parts.pop();
		return parts.join("/");
	}

	async function openFile(path: string) {
		const seq = ++openSeq;
		currentPath = path;
		titleDraft = displayName(path);
		renameError = null;

		const contents = await readFile(path);
		if (seq !== openSeq) return;

		doc = contents;
		lastSavedDoc = contents;
	}

	async function save() {
		if (currentPath === null || isRenaming) return;
		await saveFile(currentPath, doc);
		lastSavedDoc = doc;
	}

	async function commitTitleRename() {
		if (!currentPath || isRenaming) return;

		const seq = ++renameSeq;
		const oldPath = currentPath;
		renameError = null;

		const sanitizedStem = await sanitizeFilename(titleDraft);
		if (seq !== renameSeq || currentPath !== oldPath) return;

		const dir = dirname(oldPath);
		const newPath = `${dir ? dir + "/" : ""}${sanitizedStem}.md`;

		if (newPath === oldPath) {
			titleDraft = displayName(oldPath);
			return;
		}

		isRenaming = true;
		try {
			await renameFile(oldPath, newPath);
			if (seq !== renameSeq || currentPath !== oldPath) return;

			currentPath = newPath;
			titleDraft = displayName(newPath);
		} catch (e) {
			renameError = String(e);
			titleDraft = displayName(oldPath);
		} finally {
			isRenaming = false;
		}
	}

	function onTitleKeydown(ev: KeyboardEvent) {
		if (ev.key === "Enter" || ev.key === "ArrowDown") {
			ev.preventDefault();
			(ev.target as HTMLInputElement).blur();
			editorApi?.focus();
		} else if (ev.key === "Escape") {
			ev.preventDefault();
			renameError = null;
			if (currentPath) titleDraft = displayName(currentPath);
			(ev.target as HTMLInputElement).blur();
		}
	}

	function focusTitle() {
		titleInput?.focus();
		titleInput?.setSelectionRange(
			titleInput.value.length,
			titleInput.value.length,
		);
	}
</script>

<div class="app">
	{#if currentPath}
		<div class="editor-surface">
			<div class="title-area">
				<input
					bind:this={titleInput}
					class="note-title"
					value={titleDraft}
					disabled={isRenaming}
					spellcheck={false}
					placeholder="Untitled"
					oninput={(e) =>
						(titleDraft = (e.target as HTMLInputElement).value)}
					onkeydown={onTitleKeydown}
					onblur={() => void commitTitleRename()}
				/>
				{#if dirty}
					<span class="dirty-indicator" title="Unsaved changes">●</span
					>
				{/if}
			</div>

			{#if renameError}
				<div class="title-error">{renameError}</div>
			{/if}

			{#key currentPath}
				<MarkdownEditor
					{doc}
					onchange={(d) => (doc = d)}
					onsave={save}
					onfocusup={focusTitle}
					onready={(api) => (editorApi = api)}
				/>
			{/key}
		</div>
	{:else}
		<div class="empty">No file open</div>
	{/if}
</div>

<style>
	:global(:root) {
		--editor-bg: #ffffff;
		--editor-text: #1a1a1a;
		--editor-cursor: #333;
		--editor-selection: #d7e4f2;
		--dirty-color: #f59e0b;
	}

	@media (prefers-color-scheme: dark) {
		:global(:root) {
			--editor-bg: #1a1a1e;
			--editor-text: #d4d4d4;
			--editor-cursor: #c6c6c6;
			--editor-selection: #264f78;
		}
	}

	.app {
		display: flex;
		height: 100vh;
		margin: 0;
		font-family:
			-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui,
			sans-serif;
		background: var(--editor-bg);
		color: var(--editor-text);
	}

	.editor-surface {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--editor-bg);
	}

	.title-area {
		position: relative;
		flex-shrink: 0;
		max-width: 65ch;
		width: 100%;
		margin: 0 auto;
		padding: 2rem 2rem 0;
	}

	.note-title {
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
		padding: 0;
		padding-bottom: 0.5rem;
	}

	.note-title::placeholder {
		color: var(--editor-text);
		opacity: 0.25;
	}

	.note-title:disabled {
		opacity: 0.6;
	}

	.dirty-indicator {
		position: absolute;
		right: 2rem;
		top: 2.5rem;
		font-size: 0.5rem;
		color: var(--dirty-color);
	}

	.title-error {
		max-width: 65ch;
		margin: 0 auto;
		padding: 0 2rem 0.5rem;
		font-size: 0.8rem;
		color: #d14343;
	}

	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.3;
		font-size: 1.125rem;
	}
</style>
