<script lang="ts">
	interface Props {
		wordCount: number;
		backlinkCount: number;
		showBacklinks: boolean;
		showWordCount: boolean;
		onbacklinksclick?: () => void;
	}

	let { wordCount, backlinkCount, showBacklinks, showWordCount, onbacklinksclick }: Props = $props();
</script>

<div class="info-bar">
	{#if showWordCount}
		<span class="info-item">{wordCount} {wordCount === 1 ? "word" : "words"}</span>
	{/if}
	{#if showBacklinks}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<span
			class="info-item backlinks"
			class:clickable={backlinkCount > 0}
			onclick={() => { if (backlinkCount > 0) onbacklinksclick?.(); }}
		>
			{backlinkCount} {backlinkCount === 1 ? "backlink" : "backlinks"}
		</span>
	{/if}
</div>

<style>
	.info-bar {
		position: fixed;
		bottom: 0;
		right: 12px;
		display: flex;
		gap: 16px;
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
		font-size: 0.6875rem;
		color: var(--editor-text);
		background-color: var(--editor-bg);
		padding: 4px 10px;
		border-radius: 6px 6px 0 0;
		pointer-events: none;
	}

	.info-item {
		opacity: 0.4;
	}

	.backlinks.clickable {
		pointer-events: auto;
		cursor: pointer;
		transition: opacity 0.12s ease;
	}

	.backlinks.clickable:hover {
		opacity: 0.6;
	}
</style>
