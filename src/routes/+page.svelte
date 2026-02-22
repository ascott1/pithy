<script lang="ts">
	import { onMount } from "svelte";
	import MarkdownEditor from "$lib/editor/MarkdownEditor.svelte";
	import {
		listFiles,
		readFile,
		renameFile,
		sanitizeFilename,
	} from "$lib/tauri/fs";
	import { AutoSaveController, type SaveState } from "$lib/autosave";

	let currentPath = $state<string | null>(null);
	let doc = $state("");

	let saveState = $state<SaveState>("idle");
	let saveDirty = $state(false);
	let saveError = $state<string | null>(null);
	let saving = $derived(saveState === "saving");

	const autosave = new AutoSaveController(350);
	autosave.onState = (s, dirty, err) => {
		saveState = s;
		saveDirty = dirty;
		saveError = err;
	};

	let titleDraft = $state("");
	let isRenaming = $state(false);
	let renameError = $state<string | null>(null);
	let editorApi: { focus: () => void; focusTitle: () => void } | null = null;

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
		await autosave.flushAndWait();

		const seq = ++openSeq;
		currentPath = path;
		titleDraft = displayName(path);
		renameError = null;

		const contents = await readFile(path);
		if (seq !== openSeq) return;

		doc = contents;
		autosave.setOpenedFile(path, contents);
	}

	function onDocChange(d: string) {
		doc = d;
		autosave.setDoc(d);
	}

	async function save() {
		await autosave.flushNow();
	}

	async function commitTitleRename() {
		if (!currentPath || isRenaming) return;

		await autosave.flushAndWait();

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
			autosave.setOpenedFile(newPath, doc);
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
</script>

<div class="app">
	{#if currentPath}
		<div class="editor-surface">
			{#key currentPath}
				<MarkdownEditor
					{doc}
					title={titleDraft}
					titleDisabled={isRenaming}
					dirty={saveDirty}
					{saving}
					{saveError}
					{renameError}
					onchange={onDocChange}
					onsave={save}
					ontitlechange={(v) => (titleDraft = v)}
					ontitleblur={() => void commitTitleRename()}
					ontitlekeydown={onTitleKeydown}
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
		--content-max-width: 680px;
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

	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.3;
		font-size: 1.125rem;
	}
</style>
