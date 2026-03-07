# Pithy — Agent Context

## What Is This?

Pithy is a fast, focused markdown notes app for macOS desktop. The core philosophy: **get out of the way and let users write**. Speed, simplicity, no plugin sprawl.

## Tech Stack

| Layer | Technology | Notes |
|---|---|---|
| Desktop framework | **Tauri 2** | Rust backend, native webview. No Electron. |
| Frontend framework | **Svelte 5** | Uses runes (`$state`, `$derived`, etc.). SvelteKit with `adapter-static` for SSG. |
| Editor | **CodeMirror 6** | Integrated. Markdown highlighting, line wrapping, autosave. |
| Search | **Tantivy** | Integrated — Rust full-text search, index in `.pithy/` vault dotfolder. |
| Language (UI) | **TypeScript** | Strict mode enabled. |
| Language (backend) | **Rust** | Via Tauri commands. |
| Package manager | **pnpm** | |
| Build tool | **Vite 6** | Dev server on port 1420. |

## Project Structure

```
pithy/
├── src/                    # Frontend (Svelte/TypeScript)
│   ├── lib/
│   │   ├── autosave.ts      # AutoSaveController — debounced single-writer autosave
│   │   ├── daily.ts         # formatDailyName() — date-based note name formatting
│   │   ├── fuzzy.ts         # fuzzyScore() — subsequence matching for filename stems
│   │   ├── snippets.ts      # cleanSnippet(), stripMarkdown() — sanitize Tantivy search snippets
│   │   ├── BacklinksPopover.svelte  # Backlinks popover — click info bar count to see linking notes
│   │   ├── DeleteConfirmDialog.svelte  # Delete confirmation — shows broken backlink warnings
│   │   ├── WikilinkUpdateDialog.svelte  # Rename confirmation — offers bulk rewrite of wikilink refs
│   │   ├── QuickSwitcher.svelte  # Cmd+K modal — file nav, fuzzy search, create-on-enter, delete
│   │   ├── SearchPanel.svelte    # Cmd+Shift+F modal — full-text search via Tantivy
│   │   ├── InfoBar.svelte        # Status bar — word count + clickable backlinks count
│   │   ├── SettingsView.svelte   # Settings screen — macOS-style grouped sections, live apply
│   │   ├── editor/
│   │   │   ├── MarkdownEditor.svelte  # CodeMirror 6 wrapper + inline title, formatting shortcuts, find/replace
│   │   │   └── inlineRendering.ts     # Live preview — hides markdown syntax, renders styled output
│   │   └── tauri/
│   │       ├── config.ts    # Typed invoke wrappers for config commands
│   │       ├── fs.ts        # Typed invoke wrappers for Rust commands
│   │       ├── search.ts   # Typed invoke wrappers for Tantivy search commands
│   │       └── window.ts   # Typed invoke wrappers for window commands (titlebar)
│   ├── routes/             # SvelteKit routes
│   │   ├── +layout.ts      # SSR disabled (ssr = false, prerender = true)
│   │   └── +page.svelte    # Main page — title + editor surface, CSS variable definitions
│   └── app.html            # HTML shell
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point — calls pithy_lib::run()
│   │   ├── lib.rs          # Tauri builder, command registration
│   │   ├── config.rs       # Config parsing, resolution, Tauri commands
│   │   ├── fs.rs           # Filesystem commands (list, read, save, rename, sanitize, images)
│   │   └── titlebar.rs     # macOS titlebar opacity control (auto-hide via objc2_app_kit)
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri config (window size, app ID, build commands)
├── docs/                   # Developer documentation
│   ├── adding-config-settings.md
│   ├── configuration.md       # Full user-facing config reference
│   ├── creating-themes.md
│   └── shortcuts.md
├── static/                 # Static assets served at /
├── package.json            # Node dependencies & scripts
├── svelte.config.js        # SvelteKit config with adapter-static
├── vite.config.js          # Vite config (Tauri dev server settings)
└── tsconfig.json           # TypeScript config (strict, bundler resolution)
```

## Running the App

```bash
pnpm tauri dev     # Start Tauri dev mode (launches Vite + native window)
pnpm tauri build   # Build for production
pnpm dev           # Vite dev server only (no Tauri window)
pnpm check         # TypeScript/Svelte type checking
```

## Architecture Boundary Rule

**If it touches the filesystem or needs to be fast at scale, it's Rust. Everything else is TypeScript.**

Rust (Tauri commands) handles: file I/O, atomic writes, file watching (`notify` crate), search indexing (Tantivy), filename sanitization, global hotkeys.

TypeScript/Svelte handles: UI rendering, editor state, CodeMirror extensions, keybinding dispatch, theme application, autosave scheduling/debouncing.

Keep the IPC surface small — well-defined Tauri commands.

### Current Tauri Commands (fs.rs)
- `list_files() -> Vec<String>` — walks vault, returns relative `.md` paths, seeds `welcome.md` on empty vault.
- `read_file(rel_path) -> String` — reads file contents.
- `save_file(rel_path, contents)` — atomic write (temp → fsync → rename → fsync dir).
- `delete_file(rel_path)` — moves file to system Trash (via `trash` crate), removes from search index.
- `rename_file(old_rel_path, new_rel_path)` — renames, fails if destination exists.
- `sanitize_filename(name) -> String` — deterministic sanitization (strip illegal chars, collapse whitespace/dashes, preserve user input).
- `find_wikilink_references(old_stem) -> Vec<WikilinkReference>` — scans vault for files containing `[[wikilinks]]` referencing the given stem. Returns `{relPath, count}` objects. Used for backlink counts and rename/delete dialogs.
- `update_wikilink_references(old_stem, new_stem) -> Vec<String>` — bulk-rewrites all `[[wikilinks]]` targeting `old_stem` to `new_stem`. Computes all rewrites in memory first, then writes atomically. Preserves `|alias` syntax. Returns modified file paths.
- `copy_image_to_assets(source_path, filename) -> String` — copies an image into `_assets/` inside the vault. Validates extension against allowlist, enforces 200 MB size limit, sanitizes filename (lowercase stem, dash-separated), deduplicates with counter suffix. Returns vault-relative path like `_assets/my-screenshot.png`.

All paths are relative to vault root. `resolve_path()` rejects `..`, absolute paths, and other traversal via `Path::components()` checking. Tauri 2 auto-converts camelCase JS args to snake_case Rust params.

### Current Tauri Commands (titlebar.rs)
- `set_titlebar_opacity(opacity: f64)` — sets macOS title bar container alpha via `objc2_app_kit`. No-op on non-macOS. Used by the auto-hide titlebar feature.

### Current Tauri Commands (search.rs)
- `search_query(query) -> Vec<SearchResult>` — full-text search via Tantivy index.
- `search_status() -> SearchStatus` — returns index health/readiness.
- `search_rebuild()` — rebuilds the entire Tantivy index from scratch.
- `list_tags() -> Vec<String>` — returns all indexed `#tag` values.

### Current Tauri Commands (config.rs)
- `get_config_info() -> ConfigInfo` — returns resolved config snapshot (paths, editor settings, warnings). Called on startup.
- `update_config(updates: ConfigUpdates) -> ConfigInfo` — applies partial config updates via `toml_edit` (preserves comments), atomic writes TOML, re-resolves config in `AppState`, returns fresh `ConfigInfo`.
- `list_themes() -> Vec<String>` — scans `~/.config/pithy/themes/` for `.css` files, returns names plus built-in `default-light` and `default-dark`, sorted.
- `read_config_file() -> String` — returns raw TOML contents (for external tooling).
- `write_config_file(contents)` — atomic write of config TOML back to disk.

## Core Design Decisions

### Title Is the Filename
- No frontmatter, no title field in file contents. The filename stem IS the note's identity.
- The editor shows an **editable title `<input>`** injected into CM6's `.cm-scroller` (Obsidian-style inline title). It scrolls with the document. Arrow keys navigate between title and editor as if they're one surface.
- Display: underscores → spaces (`project_kickoff.md` → "project kickoff"). Dashes are preserved as-is. Display-only; file on disk unchanged.
- Editing the title triggers a file rename on blur/Enter. Escape reverts. Rename fails gracefully if destination exists.
- If wikilinks reference the old name, a confirmation dialog (`WikilinkUpdateDialog`) offers bulk rewrite of all references.

### Filename Sanitization
A single deterministic function (defined in Rust, exposed via Tauri command) used everywhere: strip illegal characters (`/ \ : * ? " < > |`), collapse consecutive whitespace/dashes, preserve spaces and Unicode characters. Filenames use the text the user provides — spaces are not converted to dashes.

### Inline Rendering (Not WYSIWYG)
The editor uses CodeMirror decorations to render markdown inline (bold appears bold, headers resize, links styled) while raw markdown is revealed when the cursor enters an element. Implemented in `src/lib/editor/inlineRendering.ts`.

**Architecture:** Two-layer system — a `StateField` (`inlineDecoField`) for the main decorations and a `ViewPlugin` (`inlineRenderingPlugin`) for tag styling and DOM events.

- **`inlineDecoField` (StateField)** — owns the main body of decorations (headings, bold, italic, links, images, wikilinks, blockquotes, lists, task lists, strikethrough, code, horizontal rules). Operates over the **full document** (not just visible ranges) to prevent invisible-text bugs during height estimation with non-standard font sizes.
- **`inlineRenderingPlugin` (ViewPlugin)** — handles `#hashtag` decorations (visible-range scan via `buildTagDecorations()`) and DOM event handlers (Cmd+click for links/wikilinks, `Meta` key CSS class toggling).
- **`vaultRootFacet`** — a `Facet` that carries the vault root path into editor state. Used by `resolveImageSrc()` to convert relative image paths to loadable `asset://` URLs. `MarkdownEditor` accepts a `vaultRoot` prop and injects it via this facet.

Both layers walk the Lezer markdown syntax tree via `syntaxTree(state).iterate()`. For each node: if the cursor intersects its range → skip decorations (show raw markdown); otherwise → hide delimiters with `Decoration.replace({})` and style content with `Decoration.mark`/`Decoration.line`.

**Key implementation details:**
- **Container/composable nodes** (headings, bold, italic, links, blockquotes) must `return` (not `return false`) from the tree iterator so nested formatting renders correctly (e.g., bold inside headings, italic inside blockquotes).
- **Leaf nodes** (inline code, fenced code, horizontal rules) use `return false` to prevent descending into children.
- **Custom highlight style:** The extension bundles its own `HighlightStyle` (a copy of `defaultHighlightStyle` with `textDecoration: underline` removed from `heading` and `link` tags).
- **IME composition:** Decoration building is skipped entirely when `view.composing` is true.
- **Cursor boundary:** `selectionIntersects()` uses inclusive checks for empty selections so cursor at a delimiter edge counts as "inside".

**Implemented elements:** ATX headings (H1–H6), bold, italic, strikethrough, links (hides `[]()` and URL, Cmd+click to open), wikilinks (hides `[[]]`, Cmd+click to navigate), inline code, fenced code blocks (styled lines, dimmed fences), blockquotes (hides `>`, left border), horizontal rules (replaced with styled line), unordered lists (bullet widget replaces `- `), ordered lists (styled number marker), task lists (interactive checkbox widget, clickable to toggle), inline images (hides syntax, renders `ImageWidget`; supports URLs and vault-relative paths), `#hashtag` tags (accent-colored pill styling).
**Deferred:** tables, footnotes, math blocks, setext headings.

### Native Menus
- Standard macOS menu bar: **Pithy | File | Edit | View | Window | Help**.
- **Pithy** submenu: About, Settings (Cmd+,), Services, Hide/Show, Quit.
- **File** submenu: New Note (Cmd+N, emits `create-new-note`), Delete Note (Cmd+Backspace, emits `delete-note`), Close Window (Cmd+W).
- **View** submenu: Enter Full Screen (Ctrl+Cmd+F, toggles fullscreen on main window).
- **Window** submenu: Minimize (Cmd+M).
- **Help** submenu: empty (macOS auto-adds searchable help entry).
- Menu events are handled in `lib.rs` `on_menu_event`. New Note and Delete Note emit Tauri events that the frontend listens for in `+page.svelte`.

### Auto-hiding Titlebar
- macOS traffic-light buttons auto-hide after 1500ms of inactivity.
- Mouse within 40px of window top reveals the titlebar; moving below schedules a 400ms hide.
- Any typing schedules an 800ms hide. Modal open (QuickSwitcher, SearchPanel, etc.) shows titlebar; modal close schedules hide.
- Implementation: `setTitlebarOpacity()` IPC in `+page.svelte`, delegating to `titlebar.rs` which manipulates `NSTitlebarContainerView` alpha via `objc2_app_kit`.

### Image Support
- **Drag-and-drop:** listens for Tauri `dragdrop` window events. Drops image files into `_assets/` via `copyImageToAssets()`, inserts `![alt](_assets/filename.ext)` at drop coordinates (falls back to cursor). Non-image files ignored.
- **Inline rendering:** images render as `ImageWidget` when cursor is away; raw markdown + image preview shown when cursor is on the node. Supports http/https URLs and vault-relative paths (resolved via `convertFileSrc`).
- **Supported formats:** png, jpg, jpeg, gif, webp, svg, bmp, ico.
- **Storage convention:** images live in `_assets/` subdirectory inside the vault root.

### In-editor Find/Replace
- Custom CodeMirror search panel (Cmd+F) for within-document find/replace. Separate from Cmd+Shift+F full-text Tantivy search.
- Find with case-sensitive, whole-word, and regex toggle buttons.
- Match counter (`current/total`), previous/next navigation (Enter/Shift+Enter).
- Expandable replace row with Replace and Replace All buttons.
- Escape closes panel and returns focus to editor.

### Navigation: Cmd+K Is Everything
- Cmd+K is the **unified** quick switcher for navigation AND creation.
- Default state (before typing) shows recent files.
- No sidebar in MVP.

### Info Bar & Backlinks Popover
- Fixed bottom-right status bar (`InfoBar.svelte`) showing word count and backlinks count.
- Visibility of each item controlled by `StatusBarConfigInfo` from config.
- Backlinks count is **clickable** when > 0 — opens a `BacklinksPopover` anchored above the info bar.
- **Popover pattern** (not modal): no backdrop, no dimming, glassmorphic styling matching QuickSwitcher. Dismissed via click-outside (`pointerdown` window listener), Escape, or opening another overlay (QuickSwitcher, SearchPanel).
- Backlink data: `+page.svelte` stores the full `WikilinkReference[]` array (`backlinkRefs`) alongside the count. Both are refreshed on file open and after rename.
- Arrow key navigation in popover with modulo wrapping, Enter to select, `pointerenter` syncs hover with keyboard — same interaction pattern as QuickSwitcher items.

### Wikilinks
- `[[wikilinks]]` resolve against filename stems, case-insensitive.
- Autocomplete triggered on `[[` keystroke, fuzzy-matches all filename stems.
- Following a link to a nonexistent note creates it.
- Disambiguation popup when multiple files share a stem (subdirectories).

### Storage & Autosave
- Plain `.md` files in a vault directory (default: `~/Documents/Pithy`).
- **Autosave:** changes auto-persist ~350ms after typing stops via `AutoSaveController`. No manual save needed — Apple Notes-style "user never thinks about saving". Cmd+S is retained as an immediate flush for muscle memory.
- **AutoSaveController** (`src/lib/autosave.ts`): debounced single-writer with waiter pattern. Uses a while-loop save cycle (not recursive promises) to coalesce rapid changes. Generation counter invalidates stale saves on file switch. `flushAndWait()` returns a promise that resolves only after the full save cycle drains. Always flush before file switch or rename.
- **Atomic writes:** write to temp file, fsync, rename, fsync parent dir. Temp file cleaned up on failure.
- File watcher via `notify` crate for external changes.
- Ignore dotfiles and sync artifacts (`.git/`, `.DS_Store`, `*.icloud`, `*.conflict`).
- Sync is the user's responsibility (iCloud, Dropbox, Git).

### Settings UI
- Cmd+, opens a **Settings screen** (`SettingsView.svelte`) — macOS-style grouped sections with rounded cards.
- Settings changes **apply live** (no restart needed). Each control change calls `updateConfig()`, which writes TOML via `toml_edit` (preserves comments), re-resolves config, and returns fresh `ConfigInfo`. The frontend applies CSS vars and theme CSS immediately.
- The TOML config file (`~/.config/pithy/config.toml`) remains the single source of truth. Power users can edit it directly outside the app.
- On first launch, `load_or_create()` writes a commented default template.
- If config TOML is malformed, app falls back to defaults and shows a warning banner.

### Config Architecture
- **Structs pipeline:** `Config` (TOML parse) → `ResolvedConfig` (validated, held in `AppState`) → `ConfigInfo`/`EditorConfigInfo` (JSON to frontend).
- **`Config`** (`config.rs`): serde struct matching the TOML shape. Has `vault: VaultConfig` and `editor: EditorConfig`. All fields have `#[serde(default)]` so partial configs work.
- **`EditorConfig`**: uses `#[serde(rename_all = "kebab-case")]` for TOML keys. Embedded directly in `ResolvedConfig` (no field flattening).
- **`EditorConfigInfo`**: separate struct with `#[serde(rename_all = "camelCase")]` for JSON serialization to the frontend. Maps from `EditorConfig` in `get_config_info`.
- **`AppState`**: holds `Arc<RwLock<ResolvedConfig>>` + `RwLock<Option<String>>` warning. `update_config` acquires a write lock to swap in new resolved config after TOML changes.
- **Frontend**: `getConfigInfo()` is called on startup; `applyConfig()` sets CSS custom properties and injects theme CSS. `updateConfig()` returns fresh `ConfigInfo` for live apply without a second IPC call. CodeMirror reads CSS vars.
- **Theme/StatusBar structs** follow the same pattern: `ThemeConfig` → `ThemeConfigInfo`, `StatusBarConfig` → `StatusBarConfigInfo`. All config info structs use `camelCase` JSON serialization for the frontend.
- **Adding a new setting** requires 5 touch-points: `EditorConfig` + `EditorConfigInfo` + `ConfigUpdates` (Rust), TS interfaces, CSS var in `applyConfig()`, control in `SettingsView.svelte`. See `docs/adding-config-settings.md`.

### Theme System
- Themes are CSS files defining `:root {}` blocks with CSS custom properties.
- **Config:** `[theme]` section in TOML with `mode` ("auto"/"light"/"dark"), `light` (theme name), `dark` (theme name).
- **Built-in themes:** `default-light` and `default-dark` are embedded as Rust string constants (`DEFAULT_LIGHT_CSS`, `DEFAULT_DARK_CSS` in `config.rs`).
- **Custom themes:** `.css` files in `~/.config/pithy/themes/`. Referenced by name (with or without `.css` extension). Missing themes fall back to built-in defaults with a warning.
- **Resolution pipeline:** `ThemeConfig` (TOML) → `resolve_theme_css()` loads CSS content → `ResolvedConfig` holds CSS strings → `ThemeConfigInfo` (JSON to frontend with `mode`, `lightCss`, `darkCss`).
- **Frontend injection:** `+page.svelte` creates a `<style id="pithy-theme">` element in `<head>` on mount. For `mode: "auto"`, wraps each theme's CSS in `@media (prefers-color-scheme: light/dark)`. For forced mode, injects the chosen CSS directly.
- **Theme CSS variables:** `--editor-bg`, `--editor-text`, `--editor-cursor`, `--editor-selection`, `--accent-color`, `--link-color`, `--dirty-color`, `--error-color`, `--code-bg`, `--code-block-bg`, `--border-color`, `--backdrop-color`, `--shadow-color`, `--checkbox-color`, `--checkbox-checked-bg` (falls back to `--accent-color`), `--checkbox-check-color` (falls back to `white`), `--tag-color` (falls back to `--accent-color`), `--tag-bg`. Font settings (`--editor-font-size`, `--editor-font-family`, `--editor-line-height`) are controlled by `[editor]` config, not themes.
- **Hardcoded CSS fallback:** `+page.svelte` still defines light defaults in `:global(:root)` as a baseline before theme CSS loads.
- See `docs/creating-themes.md` for the full user-facing guide.

### Search (Tantivy)
- Full-text search via Tantivy (Rust).
- Index stored in `.pithy/` dotfolder alongside the vault.
- Index fields: `filename_stem`, `body`, `tags` (structured), `path`, `modified_date`.
- Incremental re-index on file change events.
- Tags (`#tag`) indexed as structured metadata; tags inside code blocks/URLs excluded.

### Daily Notes
- Cmd+D opens or creates today's daily note.
- Filename format is configurable via `[daily]` section in config TOML (`format` key, default `"YYYY-MM-DD"`).
- Subfolder configurable via `folder` key (e.g., `folder = "daily"`).
- Implementation: `formatDailyName()` in `src/lib/daily.ts` handles date formatting. Config structs follow the standard pipeline (`DailyConfig` → `DailyConfigInfo`).

### Deleting Notes
- Cmd+Backspace triggers delete for the current note (vault mode only).
- Also available as a "Delete" action in the QuickSwitcher when a note is open.
- Shows `DeleteConfirmDialog` with the note name and a list of backlinks that will break.
- Files are moved to system Trash via the `trash` crate (not permanently deleted).
- After deletion, the search index is updated and the app opens the next available note.

## Performance Targets

| Metric | Target |
|---|---|
| Cold start to editable buffer | < 300ms |
| Keystroke-to-render latency | < 16ms (60fps) |
| Cmd+K open to interactive | < 50ms |
| Fuzzy search (10k notes) | < 50ms |
| Full-text search (10k notes) | < 200ms first page |
| File save (atomic write) | < 10ms |
| File rename + link rewrite | < 500ms for 100 files |
| Memory (100 notes indexed) | < 80MB RSS |

## Key Keybindings (macOS)

| Action | Shortcut |
|---|---|
| New note (untitled) | Cmd+N |
| Quick switcher (nav + create) | Cmd+K |
| Full-text search | Cmd+Shift+F |
| Find/replace (in document) | Cmd+F |
| Daily note | Cmd+D |
| Open Settings | Cmd+, |
| Delete current note | Cmd+Backspace |
| Enter Full Screen | Ctrl+Cmd+F |
| Immediate save (flush) | Cmd+S |
| Bold | Cmd+B |
| Italic | Cmd+I |
| Inline code | Cmd+E |
| Strikethrough | Cmd+Shift+X |
| Code block | Cmd+Shift+C |
| Indent/unindent | Tab / Shift+Tab |
| Quick capture (global) | Configurable |

See `docs/shortcuts.md` for the full shortcuts reference.

## Testing

### Running Tests

```bash
pnpm test          # Run all tests (TypeScript + Rust)
pnpm test:ts       # TypeScript/Svelte tests only (Vitest)
pnpm test:watch    # Vitest in watch mode
pnpm test:rust     # Rust unit tests only (cargo test)
```

### Testing Stack

| Layer | Framework | Environment |
|---|---|---|
| TypeScript/Svelte | **Vitest** | jsdom |
| Component testing | **@testing-library/svelte** | jsdom |
| Assertions (DOM) | **@testing-library/jest-dom** | — |
| Rust | **cargo test** + `tempfile` | native |

### Test File Conventions

- **Co-locate tests** next to the source file: `autosave.ts` → `autosave.test.ts`.
- **Rust tests** go in `#[cfg(test)] mod tests` at the bottom of the source file (e.g., `fs.rs`).
- Test files match the glob `src/**/*.{test,spec}.{ts,js}`.

### Mocking Tauri IPC

Frontend code depends on Tauri's `invoke()` which is unavailable in jsdom. Two mocking strategies:

1. **Mock the wrapper module** (`$lib/tauri/fs`) — preferred when testing logic that calls IPC (e.g., `AutoSaveController`):
   ```ts
   vi.mock("$lib/tauri/fs", () => ({
     saveFile: vi.fn().mockResolvedValue(undefined),
   }));
   ```

2. **Mock `@tauri-apps/api/core`** — use when testing the wrappers themselves:
   ```ts
   vi.mock("@tauri-apps/api/core", () => ({
     invoke: vi.fn(),
   }));
   ```

### Testing Patterns

- **Fake timers** for debounce/timing logic (`vi.useFakeTimers()`, `vi.advanceTimersByTimeAsync()`).
- **Rust filesystem tests** use `tempfile::tempdir()` for isolated temp directories — no real vault interaction.
- **Private Rust functions** (e.g., `sanitize_filename_impl`, `resolve_path`, `atomic_write`) are testable via in-module `#[cfg(test)]` blocks.
- Always run `pnpm test` before declaring work complete.

## What's Out of Scope for MVP

Graph view, block references/transclusion, frontmatter parsing, PDF/media, export, plugin system, mobile, GUI settings, sidebar, folder tree, collaboration, E2E encrypted sync.

## Conventions to Follow

- **Svelte 5 runes** — use `$state`, `$derived`, `$effect`, not legacy `let` reactivity.
- **TypeScript strict mode** — no `any` unless absolutely necessary.
- **Rust Tauri commands** — keep IPC surface minimal. Each command does one well-defined thing.
- **Atomic file writes** — always write-to-temp-then-rename for any file mutation.
- **macOS-first** — follow platform conventions (Cmd shortcuts, system fonts, accent colors). Linux/Windows deferred.
- **No unnecessary abstractions** — build what's needed now. No plugin architecture, no extension points, no premature generalization.
- **CSS variables** — define on `:global(:root)` (not scoped `:root`) so they reach CodeMirror's shadow styles. Dark mode is handled by the theme system (see "Theme System" section), not per-component `@media` queries.
- **Async race guards** — use sequence counters (`openSeq`, `renameSeq`) for any async operation that sets state; check the counter after `await` to discard stale results.
- **Editor remounting** — wrap `MarkdownEditor` in `{#key currentPath}` so each file gets a fresh CodeMirror instance with clean undo history.
- **Config identifier:** `com.writepithy.app`
- **Autosave flush-before-switch** — always `await autosave.flushAndWait()` before opening a different file or renaming. After rename, call `autosave.setOpenedFile(newPath, doc)` to reset the baseline.
- **CSS variables** — define on `:global(:root)` (not scoped `:root`) so they reach CodeMirror's shadow styles. Theme-controlled color vars: `--editor-bg`, `--editor-text`, `--editor-cursor`, `--editor-selection`, `--accent-color`, `--link-color`, `--dirty-color`, `--error-color`, `--code-bg`, `--code-block-bg`, `--border-color`, `--backdrop-color`, `--shadow-color`, `--checkbox-color`, `--checkbox-checked-bg`, `--checkbox-check-color`, `--tag-color`, `--tag-bg`. Layout/font vars (set from `[editor]` config): `--content-max-width`, `--editor-font-size`, `--editor-font-family`, `--editor-line-height`. Dark mode is handled by the theme system, not inline `@media` queries in component styles.
- **Keyboard shortcuts** — when adding or changing a keyboard shortcut, always update `docs/shortcuts.md` to keep the shortcuts reference in sync.
