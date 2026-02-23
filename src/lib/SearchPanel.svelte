<script lang="ts">
	import { onMount } from "svelte";
	import { searchQuery, searchStatus, searchRebuild } from "$lib/tauri/search";
	import type { SearchHit } from "$lib/tauri/search";
	import { cleanSnippet } from "$lib/snippets";

	interface Props {
		initialQuery?: string;
		onselect: (path: string) => void;
		onclose: () => void;
	}

	let { initialQuery = "", onselect, onclose }: Props = $props();

	let query = $state(initialQuery);
	let hits = $state<SearchHit[]>([]);
	let status = $state<string>("ready");
	let isSearching = $state(false);
	let selectedIndex = $state(0);
	let inputEl: HTMLInputElement | undefined = $state();
	let searchSeq = 0;
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		inputEl?.focus();
		void checkStatus();
		if (query.trim()) {
			void doSearch(query.trim());
		}
		return () => {
			if (debounceTimer) clearTimeout(debounceTimer);
		};
	});

	async function checkStatus() {
		try {
			status = await searchStatus();
		} catch {
			status = "error";
		}
	}

	function onInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		const q = query.trim();
		if (!q) {
			hits = [];
			isSearching = false;
			return;
		}
		debounceTimer = setTimeout(() => void doSearch(q), 150);
	}

	async function doSearch(q: string) {
		const seq = ++searchSeq;
		isSearching = true;
		try {
			const res = await searchQuery(q, 50);
			if (seq !== searchSeq) return;
			hits = res.hits;
			selectedIndex = 0;
		} catch {
			if (seq !== searchSeq) return;
			hits = [];
		} finally {
			if (seq === searchSeq) isSearching = false;
		}
	}

	async function handleRebuild() {
		try {
			await searchRebuild();
			status = "building";
			const poll = setInterval(async () => {
				const s = await searchStatus();
				status = s;
				if (s !== "building") clearInterval(poll);
			}, 500);
		} catch {
			status = "error";
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "ArrowDown") {
			e.preventDefault();
			if (hits.length > 0) {
				selectedIndex = (selectedIndex + 1) % hits.length;
			}
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			if (hits.length > 0) {
				selectedIndex = (selectedIndex - 1 + hits.length) % hits.length;
			}
		} else if (e.key === "Enter") {
			e.preventDefault();
			if (hits[selectedIndex]) {
				onselect(hits[selectedIndex].path);
			}
		} else if (e.key === "Escape") {
			e.preventDefault();
			onclose();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onclose();
		}
	}

	function stemDisplay(stem: string): string {
		return stem.replaceAll("-", " ").replaceAll("_", " ");
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="search-backdrop" onclick={handleBackdropClick}>
	<div class="search-panel">
		<div class="search-header">
			<input
				bind:this={inputEl}
				bind:value={query}
				oninput={onInput}
				onkeydown={handleKeydown}
				class="search-input"
				type="text"
				placeholder="Search notes…"
				spellcheck="false"
				autocomplete="off"
			/>
			{#if status === "building"}
				<span class="search-status building">Indexing…</span>
			{:else if status === "error"}
				<button class="search-status error" onclick={handleRebuild}>Rebuild index</button>
			{/if}
		</div>

		{#if isSearching}
			<div class="search-loading">Searching…</div>
		{:else if hits.length > 0}
			<div class="search-results" role="listbox">
				{#each hits as hit, i}
					<button
						class="search-hit"
						class:selected={i === selectedIndex}
						role="option"
						aria-selected={i === selectedIndex}
						onclick={() => onselect(hit.path)}
						onpointerenter={() => (selectedIndex = i)}
					>
						<div class="hit-header">
							<span class="hit-stem">{stemDisplay(hit.filenameStem)}</span>
							{#if hit.tags.length > 0}
								<span class="hit-tags">
									{#each hit.tags as tag}
										<span class="hit-tag">#{tag}</span>
									{/each}
								</span>
							{/if}
						</div>
						{#if hit.snippet}
							<div class="hit-snippet">{@html cleanSnippet(hit.snippet)}</div>
						{/if}
					</button>
				{/each}
			</div>
		{:else if query.trim() && !isSearching}
			<div class="search-empty">No results</div>
		{/if}
	</div>
</div>

<style>
	.search-backdrop {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		justify-content: center;
		padding-top: 10vh;
		background: rgba(0, 0, 0, 0.18);
	}

	@media (prefers-color-scheme: dark) {
		.search-backdrop {
			background: rgba(0, 0, 0, 0.45);
		}
	}

	.search-panel {
		width: 580px;
		max-height: 70vh;
		display: flex;
		flex-direction: column;
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", system-ui, sans-serif;
		background: color-mix(in srgb, var(--editor-bg) 92%, transparent);
		-webkit-backdrop-filter: blur(40px) saturate(180%);
		border: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
		border-radius: 12px;
		box-shadow:
			0 0 0 0.5px color-mix(in srgb, var(--editor-text) 6%, transparent),
			0 24px 60px rgba(0, 0, 0, 0.22),
			0 8px 20px rgba(0, 0, 0, 0.08);
		overflow: hidden;
		align-self: flex-start;
	}

	.search-header {
		display: flex;
		align-items: center;
		border-bottom: 1px solid color-mix(in srgb, var(--editor-text) 8%, transparent);
	}

	.search-input {
		flex: 1;
		padding: 14px 18px;
		font-family: inherit;
		font-size: 1.0625rem;
		font-weight: 400;
		letter-spacing: -0.01em;
		color: var(--editor-text);
		background: transparent;
		border: none;
		outline: none;
	}

	.search-input::placeholder {
		color: var(--editor-text);
		opacity: 0.3;
	}

	.search-status {
		flex-shrink: 0;
		margin-right: 14px;
		font-family: inherit;
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-weight: 500;
	}

	.search-status.building {
		opacity: 0.45;
		animation: pulse 1.5s ease-in-out infinite;
	}

	.search-status.error {
		color: #d14343;
		background: none;
		border: 1px solid color-mix(in srgb, #d14343 50%, transparent);
		border-radius: 5px;
		padding: 3px 10px;
		cursor: pointer;
		font-family: inherit;
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-weight: 500;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.45;
		}
		50% {
			opacity: 0.15;
		}
	}

	.search-results {
		overflow-y: auto;
		padding: 6px;
	}

	.search-hit {
		display: flex;
		flex-direction: column;
		gap: 3px;
		width: 100%;
		padding: 10px 12px;
		font-family: inherit;
		color: var(--editor-text);
		background: transparent;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		text-align: left;
		transition: background 0.08s ease;
	}

	.search-hit.selected {
		background: color-mix(in srgb, var(--editor-text) 7%, transparent);
	}

	@media (prefers-color-scheme: dark) {
		.search-hit.selected {
			background: color-mix(in srgb, var(--editor-text) 10%, transparent);
		}
	}

	.search-hit:active {
		background: color-mix(in srgb, var(--editor-text) 12%, transparent);
	}

	.hit-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.hit-stem {
		font-size: 0.875rem;
		font-weight: 500;
		letter-spacing: -0.006em;
	}

	.hit-tags {
		display: flex;
		gap: 4px;
	}

	.hit-tag {
		font-size: 0.6875rem;
		opacity: 0.35;
		font-weight: 400;
	}

	.hit-snippet {
		font-size: 0.8125rem;
		line-height: 1.45;
		opacity: 0.55;
		letter-spacing: -0.006em;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
	}

	.hit-snippet :global(b) {
		font-weight: 600;
		opacity: 1;
		color: var(--editor-text);
	}

	.search-loading,
	.search-empty {
		padding: 28px 18px;
		text-align: center;
		font-size: 0.8125rem;
		opacity: 0.35;
		font-weight: 400;
	}
</style>
