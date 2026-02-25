<script lang="ts">
	interface Props {
		wordCount: number;
		backlinkCount: number;
		showBacklinks: boolean;
		showWordCount: boolean;
	}

	let { wordCount, backlinkCount, showBacklinks, showWordCount }: Props = $props();
</script>

<div class="info-bar">
	{#if showWordCount}
		<span class="info-item">{wordCount} {wordCount === 1 ? "word" : "words"}</span>
	{/if}
	{#if showBacklinks}
		<span class="info-item">{backlinkCount} {backlinkCount === 1 ? "backlink" : "backlinks"}</span>
	{/if}
</div>

<style>
	.info-bar {
		position: fixed;
		bottom: 8px;
		right: 12px;
		display: flex;
		gap: 16px;
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
		font-size: 0.6875rem;
		color: var(--editor-text);
		opacity: 0.4;
		pointer-events: none;
	}
</style>
