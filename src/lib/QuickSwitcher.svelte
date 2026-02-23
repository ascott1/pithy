<script lang="ts">
  import { fuzzyScore } from "$lib/fuzzy";

  interface FileEntry {
    path: string;
    stem: string;
  }

  interface Props {
    files: FileEntry[];
    recents: string[];
    onselect: (path: string) => void;
    oncreate: (name: string) => void;
    onclose: () => void;
  }

  let { files, recents, onselect, oncreate, onclose }: Props = $props();

  let query = $state("");
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();

  interface ScoredEntry {
    path: string;
    stem: string;
    score: number;
  }

  const MAX_RESULTS = 50;

  let results = $derived.by(() => {
    const q = query.trim();
    if (!q) {
      const recentSet = new Set(recents);
      const recentEntries: FileEntry[] = [];
      for (const r of recents) {
        const entry = files.find((f) => f.path === r);
        if (entry) recentEntries.push(entry);
      }
      const rest = files
        .filter((f) => !recentSet.has(f.path))
        .slice(0, MAX_RESULTS - recentEntries.length);
      return [...recentEntries, ...rest].map((f) => ({
        path: f.path,
        stem: f.stem,
        score: 0,
      }));
    }

    const scored: ScoredEntry[] = [];
    for (const f of files) {
      const s = fuzzyScore(q, f.stem);
      if (s !== null) {
        scored.push({ path: f.path, stem: f.stem, score: s });
      }
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, MAX_RESULTS);
  });

  let showCreate = $derived(
    query.trim().length > 0 && results.length === 0,
  );

  $effect(() => {
    void results;
    selectedIndex = 0;
  });

  $effect(() => {
    inputEl?.focus();
  });

  function handleKeydown(e: KeyboardEvent) {
    const total = results.length + (showCreate ? 1 : 0);

    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(total, 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + Math.max(total, 1)) % Math.max(total, 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (showCreate && selectedIndex === results.length) {
        oncreate(query.trim());
      } else if (results[selectedIndex]) {
        onselect(results[selectedIndex].path);
      } else if (showCreate) {
        oncreate(query.trim());
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
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="switcher-backdrop" onclick={handleBackdropClick}>
  <div class="switcher-panel">
    <input
      bind:this={inputEl}
      bind:value={query}
      onkeydown={handleKeydown}
      class="switcher-input"
      type="text"
      placeholder="Open or create a note…"
      spellcheck="false"
      autocomplete="off"
    />

    {#if results.length > 0 || showCreate}
      <div class="switcher-results" role="listbox">
        {#each results as result, i}
          <button
            class="switcher-item"
            class:selected={i === selectedIndex}
            role="option"
            aria-selected={i === selectedIndex}
            onclick={() => onselect(result.path)}
            onpointerenter={() => (selectedIndex = i)}
          >
            <span class="item-stem">{result.stem}</span>
            {#if result.path.includes("/")}
              <span class="item-dir">{result.path.replace(/\/[^/]+$/, "")}</span>
            {/if}
          </button>
        {/each}

        {#if showCreate}
          <button
            class="switcher-item create-item"
            class:selected={selectedIndex === results.length}
            role="option"
            aria-selected={selectedIndex === results.length}
            onclick={() => oncreate(query.trim())}
            onpointerenter={() => (selectedIndex = results.length)}
          >
            <span class="create-label">Create</span>
            <span class="create-name">"{query.trim()}"</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .switcher-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(0, 0, 0, 0.18);
  }

  @media (prefers-color-scheme: dark) {
    .switcher-backdrop {
      background: rgba(0, 0, 0, 0.45);
    }
  }

  .switcher-panel {
    width: 540px;
    max-height: 420px;
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

  .switcher-input {
    width: 100%;
    padding: 14px 18px;
    font-family: inherit;
    font-size: 1.0625rem;
    font-weight: 400;
    letter-spacing: -0.01em;
    color: var(--editor-text);
    background: transparent;
    border: none;
    border-bottom: 1px solid color-mix(in srgb, var(--editor-text) 8%, transparent);
    outline: none;
  }

  .switcher-input::placeholder {
    color: var(--editor-text);
    opacity: 0.3;
  }

  .switcher-results {
    overflow-y: auto;
    padding: 6px;
  }

  .switcher-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    font-family: inherit;
    font-size: 0.875rem;
    font-weight: 400;
    letter-spacing: -0.006em;
    color: var(--editor-text);
    background: transparent;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: background 0.08s ease;
  }

  .switcher-item.selected {
    background: color-mix(in srgb, var(--editor-text) 7%, transparent);
  }

  @media (prefers-color-scheme: dark) {
    .switcher-item.selected {
      background: color-mix(in srgb, var(--editor-text) 10%, transparent);
    }
  }

  .switcher-item:active {
    background: color-mix(in srgb, var(--editor-text) 12%, transparent);
  }

  .item-stem {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-dir {
    flex-shrink: 0;
    font-size: 0.75rem;
    opacity: 0.35;
    font-weight: 400;
  }

  .create-item {
    gap: 8px;
  }

  .create-label {
    flex-shrink: 0;
    font-size: 0.6875rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.45;
  }

  .create-name {
    font-style: italic;
    opacity: 0.65;
  }
</style>
