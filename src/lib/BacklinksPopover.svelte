<script lang="ts">
	import type { WikilinkReference } from "$lib/tauri/fs";

	interface Props {
		references: WikilinkReference[];
		onselect: (path: string) => void;
		onclose: () => void;
	}

	let { references, onselect, onclose }: Props = $props();
	let selectedIndex = $state(0);
	let containerEl: HTMLDivElement | undefined = $state();

	$effect(() => {
		containerEl?.focus();
	});

	$effect(() => {
		function handleClick(e: MouseEvent) {
			if (containerEl && !containerEl.contains(e.target as Node)) {
				onclose();
			}
		}
		window.addEventListener("pointerdown", handleClick);
		return () => window.removeEventListener("pointerdown", handleClick);
	});

	function displayName(relPath: string): string {
		return relPath
			.replace(/\.md$/, "")
			.split("/")
			.pop()!
			.replaceAll("-", " ")
			.replaceAll("_", " ");
	}

	function dirHint(relPath: string): string {
		const parts = relPath.split("/");
		if (parts.length <= 1) return "";
		parts.pop();
		return parts.join("/");
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "ArrowDown") {
			e.preventDefault();
			selectedIndex = (selectedIndex + 1) % references.length;
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			selectedIndex = (selectedIndex - 1 + references.length) % references.length;
		} else if (e.key === "Enter") {
			e.preventDefault();
			if (references[selectedIndex]) {
				onselect(references[selectedIndex].relPath);
			}
		} else if (e.key === "Escape") {
			e.preventDefault();
			onclose();
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="backlinks-popover"
	bind:this={containerEl}
	onkeydown={handleKeydown}
	tabindex="-1"
	role="listbox"
>
	<div class="backlinks-header">
		{references.length} {references.length === 1 ? "backlink" : "backlinks"}
	</div>
	<div class="backlinks-list">
		{#each references as ref, i}
			<button
				class="backlink-item"
				class:selected={i === selectedIndex}
				role="option"
				aria-selected={i === selectedIndex}
				onclick={() => onselect(ref.relPath)}
				onpointerenter={() => (selectedIndex = i)}
			>
				<span class="backlink-name">{displayName(ref.relPath)}</span>
				{#if dirHint(ref.relPath)}
					<span class="backlink-dir">{dirHint(ref.relPath)}</span>
				{/if}
				{#if ref.count > 1}
					<span class="backlink-count">{ref.count} references</span>
				{/if}
			</button>
		{/each}
		{#if references.length === 0}
			<div class="backlinks-empty">No backlinks</div>
		{/if}
	</div>
</div>

<style>
	.backlinks-popover {
		position: fixed;
		bottom: 32px;
		right: 12px;
		width: 280px;
		max-height: 320px;
		display: flex;
		flex-direction: column;
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", system-ui, sans-serif;
		background: color-mix(in srgb, var(--editor-bg) 92%, transparent);
		-webkit-backdrop-filter: blur(40px) saturate(180%);
		border: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
		border-radius: 10px;
		box-shadow:
			0 0 0 0.5px color-mix(in srgb, var(--editor-text) 6%, transparent),
			0 24px 60px rgba(0, 0, 0, 0.22),
			0 8px 20px rgba(0, 0, 0, 0.08);
		overflow: hidden;
		z-index: 100;
		outline: none;
	}

	.backlinks-header {
		padding: 10px 14px 6px;
		font-size: 0.75rem;
		font-weight: 400;
		letter-spacing: -0.006em;
		opacity: 0.4;
	}

	.backlinks-list {
		overflow-y: auto;
		padding: 0 6px 6px;
	}

	.backlink-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 8px 10px;
		font-family: inherit;
		font-size: 0.8125rem;
		font-weight: 400;
		letter-spacing: -0.006em;
		color: var(--editor-text);
		background: transparent;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		text-align: left;
		transition: background 0.08s ease;
	}

	.backlink-item.selected {
		background: color-mix(in srgb, var(--editor-text) 7%, transparent);
	}

	.backlink-item:active {
		background: color-mix(in srgb, var(--editor-text) 12%, transparent);
	}

	.backlink-name {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.backlink-dir {
		flex-shrink: 0;
		font-size: 0.6875rem;
		opacity: 0.35;
	}

	.backlink-count {
		flex-shrink: 0;
		font-size: 0.6875rem;
		opacity: 0.35;
	}

	.backlinks-empty {
		padding: 16px 14px;
		text-align: center;
		font-size: 0.8125rem;
		opacity: 0.35;
	}
</style>
