<script lang="ts">
	import { onMount } from "svelte";
	import type { WikilinkReference } from "$lib/tauri/fs";

	interface Props {
		noteName: string;
		references: WikilinkReference[];
		onconfirm: () => void;
		oncancel: () => void;
	}

	let { noteName, references, onconfirm, oncancel }: Props = $props();

	let totalCount = $derived(references.reduce((sum, r) => sum + r.count, 0));
	let confirmBtn: HTMLButtonElement | undefined = $state();

	onMount(() => {
		confirmBtn?.focus();
	});

	function displayPath(relPath: string): string {
		return relPath.replace(/\.md$/, "").replaceAll("_", " ");
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Escape") {
			e.preventDefault();
			oncancel();
		} else if (e.key === "Enter") {
			e.preventDefault();
			onconfirm();
		}
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_interactive_supports_focus -->
<div class="dialog-backdrop" role="dialog" aria-modal="true" onkeydown={handleKeydown}>
	<div class="dialog-panel">
		<p class="dialog-message">
			Delete <strong>"{noteName}"</strong>? It will be moved to the Trash.
		</p>
		{#if references.length > 0}
			<p class="dialog-warning">
				{totalCount} {totalCount === 1 ? "link" : "links"} in {references.length}
				{references.length === 1 ? "note" : "notes"} will break:
			</p>
			<ul class="dialog-files">
				{#each references as ref}
					<li class="dialog-file">
						<span class="dialog-file-name">{displayPath(ref.relPath)}</span>
						<span class="dialog-file-count">{ref.count}</span>
					</li>
				{/each}
			</ul>
		{/if}
		<div class="dialog-actions">
			<button class="dialog-btn dialog-btn-secondary" onclick={oncancel}>Cancel</button>
			<button bind:this={confirmBtn} class="dialog-btn dialog-btn-destructive" onclick={onconfirm}>Move to Trash</button>
		</div>
	</div>
</div>

<style>
	.dialog-backdrop {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		justify-content: center;
		padding-top: 12vh;
		background: var(--backdrop-color, rgba(0, 0, 0, 0.18));
	}

	.dialog-panel {
		width: 420px;
		max-height: 360px;
		display: flex;
		flex-direction: column;
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
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
		padding: 18px;
	}

	.dialog-message {
		margin: 0 0 14px;
		font-size: 0.875rem;
		line-height: 1.5;
		color: var(--editor-text);
	}

	.dialog-warning {
		margin: 0 0 8px;
		font-size: 0.8125rem;
		line-height: 1.4;
		color: var(--error-color);
	}

	.dialog-files {
		list-style: none;
		margin: 0 0 16px;
		padding: 0;
		overflow-y: auto;
		max-height: 140px;
	}

	.dialog-file {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 5px 8px;
		border-radius: 6px;
		font-size: 0.8125rem;
		color: var(--editor-text);
		opacity: 0.8;
	}

	.dialog-file:nth-child(odd) {
		background: color-mix(in srgb, var(--editor-text) 4%, transparent);
	}

	.dialog-file-count {
		font-size: 0.75rem;
		opacity: 0.5;
	}

	.dialog-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.dialog-btn {
		padding: 6px 14px;
		border-radius: 6px;
		font-size: 0.8125rem;
		font-family: inherit;
		cursor: pointer;
		border: 1px solid color-mix(in srgb, var(--editor-text) 12%, transparent);
		transition: background 0.1s;
	}

	.dialog-btn-secondary {
		background: transparent;
		color: var(--editor-text);
		opacity: 0.7;
	}

	.dialog-btn-secondary:hover {
		opacity: 1;
		background: color-mix(in srgb, var(--editor-text) 8%, transparent);
	}

	.dialog-btn-destructive {
		background: var(--error-color);
		color: white;
		border-color: transparent;
	}

	.dialog-btn-destructive:hover {
		filter: brightness(1.1);
	}
</style>
