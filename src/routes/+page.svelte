<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import MarkdownEditor from "$lib/editor/MarkdownEditor.svelte";
	import QuickSwitcher from "$lib/QuickSwitcher.svelte";
	import SearchPanel from "$lib/SearchPanel.svelte";
	import InfoBar from "$lib/InfoBar.svelte";
	import BacklinksPopover from "$lib/BacklinksPopover.svelte";
	import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
	import {
		listFiles,
		readFile,
		saveFile,
		deleteFile,
		renameFile,
		sanitizeFilename,
		findWikilinkReferences,
		updateWikilinkReferences,
		copyImageToAssets,
		type WikilinkReference,
	} from "$lib/tauri/fs";
	import {
		readConfigFile,
		writeConfigFile,
		getConfigInfo,
	} from "$lib/tauri/config";
	import { AutoSaveController, type SaveState } from "$lib/autosave";
	import { resolveWikilink } from "$lib/editor/wikilink";
	import { formatDailyName } from "$lib/daily";
	import WikilinkUpdateDialog from "$lib/WikilinkUpdateDialog.svelte";
	import DeleteConfirmDialog from "$lib/DeleteConfirmDialog.svelte";
	import type { DailyConfigInfo, StatusBarConfigInfo } from "$lib/tauri/config";
	import { StreamLanguage } from "@codemirror/language";
	import { toml } from "@codemirror/legacy-modes/mode/toml";

	const tomlLang = StreamLanguage.define(toml);

	let mode = $state<"vault" | "config">("vault");
	let currentPath = $state<string | null>(null);
	let doc = $state("");
	let configWarning = $state<string | null>(null);
	let vaultDir = $state("");

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

	const configAutosave = new AutoSaveController(350, (_path, contents) =>
		writeConfigFile(contents),
	);
	configAutosave.onState = (s, dirty, err) => {
		if (mode === "config") {
			saveState = s;
			saveDirty = dirty;
			saveError = err;
		}
	};

	let showSwitcher = $state(false);
	let showSearch = $state(false);
	let searchInitialQuery = $state("");

	interface FileEntry {
		path: string;
		stem: string;
	}

	let fileEntries = $state<FileEntry[]>([]);
	let recentPaths = $state<string[]>([]);

	let autoUpdateLinks = $state(true);
	let dailyConfig = $state<DailyConfigInfo>({ dir: "daily", format: "YYYY-MM-DD" });
	let statusBarConfig = $state<StatusBarConfigInfo>({ show: true, showBacklinks: true, showWordCount: true });
	let backlinkCount = $state(0);
	let backlinkRefs = $state<WikilinkReference[]>([]);
	let showBacklinksPopover = $state(false);
	let wordCount = $derived(doc.trim() ? doc.trim().split(/\s+/).length : 0);

	let titleDraft = $state("");
	let isRenaming = $state(false);
	let renameError = $state<string | null>(null);

	let wikilinkDialog = $state<{
		oldName: string;
		newName: string;
		oldStem: string;
		newStem: string;
		newPath: string;
		references: WikilinkReference[];
	} | null>(null);
	let deleteDialog = $state<{
		noteName: string;
		path: string;
		references: WikilinkReference[];
	} | null>(null);
	let editorApi: { focus: () => void; focusTitle: () => void; insertAtCoords: (text: string, coords: { x: number; y: number }) => boolean; insertAtCursor: (text: string) => void } | null = null;

	let openSeq = 0;
	let renameSeq = 0;
	let unlistenConfig: UnlistenFn | null = null;
	let unlistenDragDrop: UnlistenFn | null = null;

	const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico"]);

	function isImageFile(path: string): boolean {
		const ext = path.split(".").pop()?.toLowerCase() ?? "";
		return IMAGE_EXTENSIONS.has(ext);
	}

	async function handleImageDrop(paths: string[], position: { x: number; y: number }) {
		if (mode !== "vault" || !currentPath || !editorApi) return;

		const imagePaths = paths.filter(isImageFile);
		if (imagePaths.length === 0) return;

		const markdownParts: string[] = [];
		for (const sourcePath of imagePaths) {
			const filename = sourcePath.split("/").pop() ?? "image.png";
			try {
				const relPath = await copyImageToAssets(sourcePath, filename);
				// Derive alt text from the sanitized filename in relPath (not raw filename)
				const sanitizedStem = relPath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "image";
				const alt = sanitizedStem.replaceAll("-", " ");
				markdownParts.push(`![${alt}](${relPath})`);
			} catch (e) {
				console.error("Failed to copy image:", e);
			}
		}

		if (markdownParts.length > 0) {
			const text = markdownParts.join("\n");
			// Try inserting at drop coords; fall back to cursor position
			if (!editorApi.insertAtCoords(text, position)) {
				editorApi.insertAtCursor(text);
			}
		}
	}

	onMount(async () => {
		const info = await getConfigInfo();
		if (info.warning) configWarning = info.warning;
		vaultDir = info.vaultDir;

		autoUpdateLinks = info.autoUpdateLinks;
		dailyConfig = info.daily;
		statusBarConfig = info.statusBar;
		document.documentElement.style.setProperty("--editor-font-size", `${info.editor.fontSize}px`);
		document.documentElement.style.setProperty("--editor-font-family", info.editor.fontFamily);
		document.documentElement.style.setProperty("--editor-line-height", `${info.editor.lineHeight}`);

		// Inject theme CSS
		const themeStyle = document.createElement("style");
		themeStyle.id = "pithy-theme";
		if (info.theme.mode === "light") {
			themeStyle.textContent = info.theme.lightCss;
		} else if (info.theme.mode === "dark") {
			themeStyle.textContent = info.theme.darkCss;
		} else {
			themeStyle.textContent =
				`@media (prefers-color-scheme: light) { ${info.theme.lightCss} }\n` +
				`@media (prefers-color-scheme: dark) { ${info.theme.darkCss} }`;
		}
		document.head.appendChild(themeStyle);

		unlistenConfig = await listen("open-config", () => {
			void openConfig();
		});

		const appWindow = getCurrentWebviewWindow();
		unlistenDragDrop = await appWindow.onDragDropEvent((event) => {
			if (event.payload.type === "drop") {
				void handleImageDrop(event.payload.paths, event.payload.position);
			}
		});

		const files = await listFiles();
		fileEntries = buildFileEntries(files);
		if (files.length > 0) {
			await openFile(files[0]);
			addRecent(files[0]);
		}
	});

	onDestroy(() => {
		unlistenConfig?.();
		unlistenDragDrop?.();
	});

	function handleGlobalKeydown(e: KeyboardEvent) {
		if (e.isComposing) return;

		if (e.metaKey && e.key === "k") {
			e.preventDefault();
			showSearch = false;
			showBacklinksPopover = false;
			if (!showSwitcher) {
				listFiles().then((files) => {
					fileEntries = buildFileEntries(files);
				});
			}
			showSwitcher = !showSwitcher;
		} else if (e.metaKey && e.key === "d") {
			e.preventDefault();
			void openOrCreateDailyNote();
		} else if (e.metaKey && e.key === "Backspace" && mode === "vault" && currentPath) {
			e.preventDefault();
			void deleteCurrentNote();
		} else if (e.metaKey && e.shiftKey && e.key.toLowerCase() === "f") {
			e.preventDefault();
			showSwitcher = false;
			showBacklinksPopover = false;
			searchInitialQuery = "";
			showSearch = !showSearch;
		}
	}

	$effect(() => {
		window.addEventListener("keydown", handleGlobalKeydown);
		return () => window.removeEventListener("keydown", handleGlobalKeydown);
	});

	function displayName(path: string): string {
		return path
			.replace(/\.md$/, "")
			.split("/")
			.pop()!
			.replaceAll("_", " ");
	}

	function dirname(relPath: string): string {
		const parts = relPath.split("/");
		parts.pop();
		return parts.join("/");
	}

	function buildFileEntries(paths: string[]): FileEntry[] {
		return paths.map((p) => ({
			path: p,
			stem: displayName(p),
		}));
	}

	function addRecent(path: string) {
		recentPaths = [path, ...recentPaths.filter((r) => r !== path)].slice(0, 20);
	}

	async function openNote(path: string) {
		showSwitcher = false;
		showSearch = false;
		if (mode === "config") {
			await configAutosave.flushAndWait();
			mode = "vault";
		}
		await openFile(path);
		addRecent(path);
	}

	async function createNote(name: string) {
		const sanitized = await sanitizeFilename(name);
		const relPath = `${sanitized}.md`;

		if (fileEntries.some((f) => f.path === relPath)) {
			await openNote(relPath);
			return;
		}

		await saveFile(relPath, "");
		fileEntries = [...fileEntries, { path: relPath, stem: displayName(relPath) }];
		await openNote(relPath);
	}

	async function openOrCreateDailyNote() {
		const name = formatDailyName(dailyConfig.format);
		const relPath = `${dailyConfig.dir}/${name}.md`;

		if (fileEntries.some((f) => f.path === relPath)) {
			await openNote(relPath);
			return;
		}

		await saveFile(relPath, "");
		fileEntries = [...fileEntries, { path: relPath, stem: displayName(relPath) }];
		await openNote(relPath);
	}

	async function handleWikilinkNavigate(target: string) {
		if (!target.trim()) return;
		const resolved = resolveWikilink(target, fileEntries);
		if (resolved) {
			await openNote(resolved);
		} else {
			await createNote(target);
		}
	}

	async function deleteCurrentNote() {
		if (!currentPath || mode !== "vault") return;
		await autosave.flushAndWait();

		const stem = currentPath.replace(/\.md$/, "").split("/").pop()!;
		const refs = await findWikilinkReferences(stem);
		deleteDialog = {
			noteName: displayName(currentPath),
			path: currentPath,
			references: refs,
		};
	}

	async function confirmDelete() {
		if (!deleteDialog) return;
		const pathToDelete = deleteDialog.path;
		deleteDialog = null;

		try {
			await deleteFile(pathToDelete);
		} catch (e) {
			console.error("Failed to delete:", e);
			return;
		}

		fileEntries = fileEntries.filter((f) => f.path !== pathToDelete);

		// Open next note: most recent (skip deleted), or first available, or empty state
		const nextPath = recentPaths.find((p) => p !== pathToDelete && fileEntries.some((f) => f.path === p))
			?? fileEntries[0]?.path
			?? null;

		recentPaths = recentPaths.filter((p) => p !== pathToDelete);

		if (nextPath) {
			await openFile(nextPath);
			addRecent(nextPath);
		} else {
			currentPath = null;
			doc = "";
		}
	}

	async function openConfig() {
		if (mode === "config") return;
		await autosave.flushAndWait();

		const contents = await readConfigFile();
		mode = "config";
		currentPath = "config.toml";
		titleDraft = "config";
		renameError = null;
		doc = contents;
		configAutosave.setOpenedFile("config.toml", contents);
	}

	async function closeConfig() {
		await configAutosave.flushAndWait();
		mode = "vault";

		const files = await listFiles();
		if (files.length > 0) {
			await openFile(files[0]);
		} else {
			currentPath = null;
			doc = "";
		}
	}

	async function openFile(path: string) {
		await autosave.flushAndWait();

		const seq = ++openSeq;
		currentPath = path;
		titleDraft = displayName(path);
		renameError = null;
		backlinkCount = 0;
		backlinkRefs = [];
		showBacklinksPopover = false;

		const contents = await readFile(path);
		if (seq !== openSeq) return;

		doc = contents;
		autosave.setOpenedFile(path, contents);

		const stem = path.replace(/\.md$/, "").split("/").pop()!;
		const refs = await findWikilinkReferences(stem);
		if (seq !== openSeq) return;
		backlinkRefs = refs;
		backlinkCount = refs.length;
	}

	function onDocChange(d: string) {
		doc = d;
		if (mode === "config") {
			configAutosave.setDoc(d);
		} else {
			autosave.setDoc(d);
		}
	}

	async function save() {
		if (mode === "config") {
			await configAutosave.flushNow();
		} else {
			await autosave.flushNow();
		}
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

			// Re-fetch backlinks for new filename
			const newStemForBacklinks = newPath.replace(/\.md$/, "").split("/").pop()!;
			const newRefs = await findWikilinkReferences(newStemForBacklinks);
			backlinkRefs = newRefs;
			backlinkCount = newRefs.length;

			// Update wikilinks referencing the old name
			const oldStem = oldPath.replace(/\.md$/, "").split("/").pop()!;
			const newStem = newPath.replace(/\.md$/, "").split("/").pop()!;
			const refs = await findWikilinkReferences(oldStem);
			if (refs.length > 0) {
				if (autoUpdateLinks) {
					const modified = await updateWikilinkReferences(oldStem, newStem);
					if (currentPath && modified.includes(currentPath)) {
						doc = await readFile(currentPath);
						autosave.setOpenedFile(currentPath, doc);
					}
				} else {
					wikilinkDialog = {
						oldName: displayName(oldPath),
						newName: displayName(newPath),
						oldStem,
						newStem,
						newPath,
						references: refs,
					};
				}
			}
		} catch (e) {
			renameError = String(e);
			titleDraft = displayName(oldPath);
		} finally {
			isRenaming = false;
		}
	}

	async function handleWikilinkUpdate() {
		if (!wikilinkDialog) return;
		const { oldStem, newStem } = wikilinkDialog;
		wikilinkDialog = null;
		const modified = await updateWikilinkReferences(oldStem, newStem);
		if (currentPath && modified.includes(currentPath)) {
			doc = await readFile(currentPath);
			autosave.setOpenedFile(currentPath, doc);
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
	<div class="drag-region" data-tauri-drag-region></div>
	{#if configWarning}
		<div class="warning-banner">
			<span class="warning-text">{configWarning}</span>
			<button class="warning-dismiss" onclick={() => (configWarning = null)} aria-label="Dismiss">✕</button>
		</div>
	{/if}

	{#if mode === "config"}
		<div class="editor-surface">
			<div class="config-bar">
				<button class="config-back" onclick={() => void closeConfig()}>← Back</button>
				<span class="config-notice">Restart Pithy to apply changes</span>
			</div>
			{#key "config"}
				<MarkdownEditor
					{doc}
					lang={tomlLang}
					title="config"
					titleDisabled={true}
					dirty={saveDirty}
					{saving}
					{saveError}
					renameError={null}
					onchange={onDocChange}
					onsave={save}
					ontitlechange={() => {}}
					ontitleblur={() => {}}
					ontitlekeydown={() => {}}
					onready={(api) => (editorApi = api)}
				/>
			{/key}
		</div>
	{:else if currentPath}
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
					onready={(api) => {
						editorApi = api;
						api.focus();
					}}
					fileStems={fileEntries}
					onnavigate={(t) => void handleWikilinkNavigate(t)}
					vaultRoot={vaultDir}
				/>
			{/key}
		</div>
	{:else}
		<div class="empty">No file open</div>
	{/if}
</div>

{#if mode === "vault" && currentPath && statusBarConfig.show}
	<InfoBar
		{wordCount}
		{backlinkCount}
		showBacklinks={statusBarConfig.showBacklinks}
		showWordCount={statusBarConfig.showWordCount}
		onbacklinksclick={() => (showBacklinksPopover = true)}
	/>
{/if}

{#if showBacklinksPopover && backlinkRefs.length > 0}
	<BacklinksPopover
		references={backlinkRefs}
		onselect={(path) => { showBacklinksPopover = false; void openNote(path); }}
		onclose={() => (showBacklinksPopover = false)}
	/>
{/if}

{#if showSwitcher}
	<QuickSwitcher
		files={fileEntries}
		recents={recentPaths}
		{currentPath}
		onselect={(path) => void openNote(path)}
		oncreate={(name) => void createNote(name)}
		ondelete={() => { showSwitcher = false; void deleteCurrentNote(); }}
		onsearch={(q) => {
			showSwitcher = false;
			searchInitialQuery = q;
			showSearch = true;
		}}
		onclose={() => (showSwitcher = false)}
	/>
{/if}

{#if showSearch}
	<SearchPanel
		initialQuery={searchInitialQuery}
		onselect={(path) => void openNote(path)}
		onclose={() => (showSearch = false)}
	/>
{/if}

{#if wikilinkDialog}
	<WikilinkUpdateDialog
		oldName={wikilinkDialog.oldName}
		newName={wikilinkDialog.newName}
		references={wikilinkDialog.references}
		onupdate={() => void handleWikilinkUpdate()}
		onskip={() => (wikilinkDialog = null)}
	/>
{/if}

{#if deleteDialog}
	<DeleteConfirmDialog
		noteName={deleteDialog.noteName}
		references={deleteDialog.references}
		onconfirm={() => void confirmDelete()}
		oncancel={() => (deleteDialog = null)}
	/>
{/if}

<style>
	:global(:root) {
		--editor-bg: #ffffff;
		--editor-text: #37352f;
		--editor-cursor: #37352f;
		--editor-selection: #d3e0f0;
		--accent-color: #2383e2;
		--dirty-color: #d9730d;
		--link-color: #2383e2;
		--error-color: #c4463a;
		--code-bg: rgba(135, 131, 120, 0.1);
		--code-block-bg: rgba(135, 131, 120, 0.04);
		--border-color: rgba(55, 53, 47, 0.16);
		--backdrop-color: rgba(15, 15, 15, 0.6);
		--shadow-color: rgba(15, 15, 15, 0.1);
		--tag-color: #2383e2;
		--tag-bg: rgba(35, 131, 226, 0.08);
		--content-max-width: 680px;
		--editor-font-size: 15px;
		--editor-font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
		--editor-line-height: 1.7;
	}

	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		margin: 0;
		font-family:
			-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui,
			sans-serif;
		background: var(--editor-bg);
		color: var(--editor-text);
	}

	.drag-region {
		height: 28px;
		flex-shrink: 0;
		-webkit-app-region: drag;
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

	.warning-banner {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 8px 16px 0;
		padding: 6px 10px;
		background: color-mix(in srgb, var(--dirty-color) 15%, transparent);
		border: 1px solid color-mix(in srgb, var(--dirty-color) 40%, transparent);
		border-radius: 6px;
		font-size: 0.75rem;
		color: var(--editor-text);
	}

	.warning-text {
		flex: 1;
		opacity: 0.8;
	}

	.warning-dismiss {
		flex-shrink: 0;
		background: none;
		border: none;
		color: var(--editor-text);
		opacity: 0.4;
		cursor: pointer;
		font-size: 0.75rem;
		padding: 0 2px;
		line-height: 1;
	}

	.warning-dismiss:hover {
		opacity: 0.8;
	}

	.config-bar {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 16px;
	}

	.config-back {
		background: none;
		border: none;
		color: var(--editor-text);
		cursor: pointer;
		font-size: 0.8125rem;
		padding: 2px 6px;
		border-radius: 4px;
		opacity: 0.7;
	}

	.config-back:hover {
		opacity: 1;
		background: color-mix(in srgb, var(--editor-text) 8%, transparent);
	}

	.config-notice {
		font-size: 0.75rem;
		opacity: 0.5;
	}
</style>
