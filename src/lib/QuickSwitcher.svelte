<script lang="ts">
  import { fuzzyScore } from "$lib/fuzzy";
  import { listTags, searchQuery } from "$lib/tauri/search";
  import type { SearchHit } from "$lib/tauri/search";
  import { cleanSnippet } from "$lib/snippets";
  import FileText from "phosphor-svelte/lib/FileText";
  import Calendar from "phosphor-svelte/lib/Calendar";
  import Plus from "phosphor-svelte/lib/Plus";
  import Trash from "phosphor-svelte/lib/Trash";
  import Hash from "phosphor-svelte/lib/Hash";
  import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";

  interface FileEntry {
    path: string;
    stem: string;
  }

  interface Props {
    files: FileEntry[];
    recents: string[];
    currentPath: string | null;
    dailyDir: string;
    onselect: (path: string) => void;
    oncreate: (name: string) => void;
    ondelete: () => void;
    onsearch: (query: string) => void;
    onclose: () => void;
  }

  let { files, recents, currentPath, dailyDir, onselect, oncreate, ondelete, onsearch, onclose }: Props = $props();

  let query = $state("");
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();

  interface ScoredEntry {
    path: string;
    stem: string;
    score: number;
  }

  const MAX_RESULTS = 50;
  const MAX_CONTENT_RESULTS = 8;

  // Tag browsing state
  let allTags = $state<string[]>([]);
  let tagsLoaded = $state(false);

  // Inline content search state
  let contentHits = $state<SearchHit[]>([]);
  let contentSearchSeq = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  let isTagMode = $derived(query.startsWith("#"));
  let isSearchMode = $derived(query.startsWith("/"));

  // The actual text to search with, stripping the / prefix in search mode
  let searchText = $derived(isSearchMode ? query.slice(1) : query);

  // Load tags when entering tag mode
  $effect(() => {
    if (isTagMode && !tagsLoaded) {
      listTags().then((tags) => {
        allTags = tags;
        tagsLoaded = true;
      }).catch(() => {
        allTags = [];
        tagsLoaded = true;
      });
    }
  });

  // Debounced content search when there's a query
  $effect(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = searchText.trim();
    if (!q || isTagMode) {
      contentHits = [];
      return;
    }
    debounceTimer = setTimeout(() => {
      const seq = ++contentSearchSeq;
      searchQuery(q, MAX_CONTENT_RESULTS).then((res) => {
        if (seq !== contentSearchSeq) return;
        contentHits = res.hits;
      }).catch(() => {
        if (seq !== contentSearchSeq) return;
        contentHits = [];
      });
    }, 150);
  });

  let tagResults = $derived.by(() => {
    if (!isTagMode) return [];
    const filter = query.slice(1).toLowerCase();
    const filtered = filter
      ? allTags.filter((t) => t.toLowerCase().includes(filter))
      : allTags;
    return filtered.slice(0, MAX_RESULTS);
  });

  let results = $derived.by(() => {
    if (isTagMode || isSearchMode) return [];
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

  // Filter content hits to exclude files already shown in filename results
  let filteredContentHits = $derived.by(() => {
    if (!searchText.trim() || isTagMode) return [];
    const filenamePaths = new Set(results.map((r) => r.path));
    return contentHits.filter((h) => !filenamePaths.has(h.path));
  });

  let showCreate = $derived(
    !isTagMode && !isSearchMode && query.trim().length > 0 && results.length === 0,
  );

  let currentStem = $derived(
    currentPath
      ? currentPath.replace(/\.md$/, "").split("/").pop()!.replaceAll("_", " ")
      : null,
  );

  let showDelete = $derived(
    !isTagMode && !isSearchMode && currentPath !== null && query.trim().toLowerCase().startsWith("delete"),
  );

  // Total number of actionable items in the list
  let totalItems = $derived.by(() => {
    if (isTagMode) return tagResults.length;
    return results.length + (showCreate ? 1 : 0) + (showDelete ? 1 : 0) + filteredContentHits.length;
  });

  $effect(() => {
    // Reset selection when filename results change
    void results;
    void tagResults;
    selectedIndex = 0;
  });

  $effect(() => {
    inputEl?.focus();
  });

  function stemDisplay(stem: string): string {
    return stem.replaceAll("_", " ");
  }

  function isDailyNote(path: string): boolean {
    return path.startsWith(dailyDir + "/");
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(totalItems, 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + Math.max(totalItems, 1)) % Math.max(totalItems, 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (isTagMode) {
        if (tagResults[selectedIndex]) {
          onsearch(`#${tagResults[selectedIndex]}`);
        }
      } else if (selectedIndex < results.length) {
        onselect(results[selectedIndex].path);
      } else {
        const afterFiles = selectedIndex - results.length;
        let offset = afterFiles;
        if (showCreate) {
          if (offset === 0) { oncreate(query.trim()); return; }
          offset--;
        }
        if (showDelete) {
          if (offset === 0) { ondelete(); return; }
          offset--;
        }
        if (filteredContentHits[offset]) {
          onselect(filteredContentHits[offset].path);
        }
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

  let modeChip = $derived.by(() => {
    if (isTagMode) return "Tags";
    if (isSearchMode) return "Search";
    return null;
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="switcher-backdrop" onclick={handleBackdropClick}>
  <div class="switcher-panel">
    <div class="input-row">
      {#if modeChip}
        <span class="mode-chip">{modeChip}</span>
      {/if}
      <input
        bind:this={inputEl}
        bind:value={query}
        onkeydown={handleKeydown}
        class="switcher-input"
        type="text"
        placeholder="Open or create a note, # tags, / search…"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    {#if isTagMode}
      <div class="switcher-results" role="listbox">
        {#if tagResults.length > 0}
          {#each tagResults as tag, i}
            <button
              class="switcher-item"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onclick={() => onsearch(`#${tag}`)}
              onpointerenter={() => (selectedIndex = i)}
            >
              <span class="item-icon"><Hash size={16} weight="light" /></span>
              <span class="item-tag">#{tag}</span>
            </button>
          {/each}
        {:else if tagsLoaded}
          <div class="switcher-empty">No tags found</div>
        {:else}
          <div class="switcher-empty">Loading tags…</div>
        {/if}
      </div>
    {:else if results.length > 0 || showCreate || showDelete || filteredContentHits.length > 0}
      <div class="switcher-results" role="listbox">
        {#if results.length > 0}
          {#each results as result, i}
            <button
              class="switcher-item"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onclick={() => onselect(result.path)}
              onpointerenter={() => (selectedIndex = i)}
            >
              <span class="item-icon">
                {#if isDailyNote(result.path)}
                  <Calendar size={16} weight="light" />
                {:else}
                  <FileText size={16} weight="light" />
                {/if}
              </span>
              <span class="item-stem">{result.stem}</span>
              {#if result.path.includes("/")}
                <span class="item-dir">{result.path.replace(/\/[^/]+$/, "")}</span>
              {/if}
            </button>
          {/each}
        {/if}

        {#if showCreate}
          {@const createIndex = results.length}
          <div class="section-divider">Note</div>
          <button
            class="switcher-item create-item"
            class:selected={selectedIndex === createIndex}
            role="option"
            aria-selected={selectedIndex === createIndex}
            onclick={() => oncreate(query.trim())}
            onpointerenter={() => (selectedIndex = createIndex)}
          >
            <span class="item-icon create-icon"><Plus size={16} weight="light" /></span>
            <span class="create-text">Create "{query.trim()}"</span>
          </button>
        {/if}

        {#if showDelete}
          {@const deleteIndex = results.length + (showCreate ? 1 : 0)}
          <div class="section-divider">Action</div>
          <button
            class="switcher-item delete-item"
            class:selected={selectedIndex === deleteIndex}
            role="option"
            aria-selected={selectedIndex === deleteIndex}
            onclick={() => ondelete()}
            onpointerenter={() => (selectedIndex = deleteIndex)}
          >
            <span class="item-icon delete-icon"><Trash size={16} weight="light" /></span>
            <span class="delete-text">Delete "{currentStem}"</span>
            <kbd class="shortcut-badge">{"\u2318\u232B"}</kbd>
          </button>
        {/if}

        {#if filteredContentHits.length > 0}
          <div class="section-divider">{isSearchMode ? `Search for "${searchText.trim()}"` : "In notes"}</div>
          {#each filteredContentHits as hit, i}
            {@const flatIndex = results.length + (showCreate ? 1 : 0) + (showDelete ? 1 : 0) + i}
            <button
              class="switcher-item content-item"
              class:selected={selectedIndex === flatIndex}
              role="option"
              aria-selected={selectedIndex === flatIndex}
              onclick={() => onselect(hit.path)}
              onpointerenter={() => (selectedIndex = flatIndex)}
            >
              <span class="item-icon"><MagnifyingGlass size={16} weight="light" /></span>
              <div class="content-hit">
                <span class="content-stem">{stemDisplay(hit.filenameStem)}</span>
                {#if hit.snippet}
                  <span class="content-snippet">{@html cleanSnippet(hit.snippet)}</span>
                {/if}
              </div>
            </button>
          {/each}
        {/if}
      </div>
    {:else if isSearchMode && searchText.trim()}
      <div class="switcher-empty">No results</div>
    {/if}

    <div class="switcher-footer">
      <div class="footer-group">
        <kbd class="footer-key">&uarr;&darr;</kbd>
        <span class="footer-label">navigate</span>
      </div>
      <div class="footer-group">
        <kbd class="footer-key">&crarr;</kbd>
        <span class="footer-label">open</span>
      </div>
      <div class="footer-group">
        <kbd class="footer-key">esc</kbd>
        <span class="footer-label">close</span>
      </div>
    </div>
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
    background: var(--backdrop-color, rgba(0, 0, 0, 0.18));
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

  .input-row {
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--editor-text) 8%, transparent);
    padding-left: 18px;
  }

  .mode-chip {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent-color) 15%, transparent);
    color: var(--accent-color);
    line-height: 1.4;
    letter-spacing: 0.02em;
  }

  .switcher-input {
    flex: 1;
    min-width: 0;
    padding: 14px 18px 14px 0;
    font-family: inherit;
    font-size: 1.0625rem;
    font-weight: 500;
    letter-spacing: -0.01em;
    color: var(--editor-text);
    background: transparent;
    border: none;
    outline: none;
  }

  .input-row:has(.mode-chip) .switcher-input {
    padding-left: 0;
  }

  .switcher-input::placeholder {
    color: var(--editor-text);
    opacity: 0.3;
    font-weight: 400;
  }

  .switcher-results {
    overflow-y: auto;
    padding: 6px;
  }

  .switcher-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
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

  .switcher-item:active {
    background: color-mix(in srgb, var(--editor-text) 12%, transparent);
  }

  .item-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    opacity: 0.4;
    color: var(--editor-text);
  }

  .item-stem {
    flex: 1;
    font-weight: 500;
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

  .item-tag {
    flex: 1;
    opacity: 0.7;
  }

  .shortcut-badge {
    flex-shrink: 0;
    font-family: inherit;
    font-size: 11px;
    font-weight: 400;
    padding: 1px 6px;
    border: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
    border-radius: 4px;
    opacity: 0.35;
    background: transparent;
    color: var(--editor-text);
    line-height: 1.4;
  }

  .section-divider {
    padding: 10px 12px 4px;
    font-size: 11px;
    font-weight: 400;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.35;
    border-top: 1px solid color-mix(in srgb, var(--editor-text) 8%, transparent);
    margin-top: 10px;
  }

  .create-item {
    gap: 8px;
  }

  .create-icon {
    opacity: 0.4;
  }

  .create-text {
    opacity: 0.65;
  }

  .delete-item {
    gap: 8px;
  }

  .delete-icon {
    opacity: 0.5;
    color: var(--error-color);
  }

  .delete-text {
    flex: 1;
    opacity: 0.65;
    color: var(--error-color);
  }

  .content-item {
    align-items: flex-start;
  }

  .content-hit {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .content-stem {
    font-size: 0.875rem;
    font-weight: 500;
    letter-spacing: -0.006em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .content-snippet {
    font-size: 0.75rem;
    line-height: 1.4;
    opacity: 0.45;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
  }

  .content-snippet :global(b) {
    font-weight: 600;
    opacity: 1;
    color: var(--editor-text);
  }

  .switcher-empty {
    padding: 20px 18px;
    text-align: center;
    font-size: 0.8125rem;
    opacity: 0.35;
  }

  .switcher-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 12px;
    border-top: 1px solid color-mix(in srgb, var(--editor-text) 8%, transparent);
    font-size: 11px;
    opacity: 0.3;
  }

  .footer-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .footer-key {
    font-family: inherit;
    font-size: 11px;
    font-weight: 400;
    padding: 1px 4px;
    border: 1px solid color-mix(in srgb, var(--editor-text) 10%, transparent);
    border-radius: 4px;
    background: transparent;
    color: var(--editor-text);
    line-height: 1.4;
  }

  .footer-label {
    color: var(--editor-text);
  }
</style>
