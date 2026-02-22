# Pithy — Agent Context

## What Is This?

Pithy is a fast, focused markdown notes app for macOS desktop. The core philosophy: **get out of the way and let users write**. Speed, simplicity, no plugin sprawl.

## Tech Stack

| Layer | Technology | Notes |
|---|---|---|
| Desktop framework | **Tauri 2** | Rust backend, native webview. No Electron. |
| Frontend framework | **Svelte 5** | Uses runes (`$state`, `$derived`, etc.). SvelteKit with `adapter-static` for SSG. |
| Editor | **CodeMirror 6** | Integrated. Markdown highlighting, line wrapping, autosave. |
| Search | **Tantivy** | Not yet integrated — Rust full-text search library. |
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
│   │   ├── editor/
│   │   │   └── MarkdownEditor.svelte  # CodeMirror 6 wrapper + inline title (injected into CM scroller)
│   │   └── tauri/
│   │       ├── config.ts    # Typed invoke wrappers for config commands
│   │       └── fs.ts        # Typed invoke wrappers for Rust commands
│   ├── routes/             # SvelteKit routes
│   │   ├── +layout.ts      # SSR disabled (ssr = false, prerender = true)
│   │   └── +page.svelte    # Main page — title + editor surface
│   └── app.html            # HTML shell
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point — calls pithy_lib::run()
│   │   ├── lib.rs          # Tauri builder, command registration
│   │   ├── config.rs       # Config parsing, resolution, Tauri commands
│   │   └── fs.rs           # Filesystem commands (list, read, save, rename, sanitize)
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri config (window size, app ID, build commands)
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
- `rename_file(old_rel_path, new_rel_path)` — renames, fails if destination exists.
- `sanitize_filename(name) -> String` — deterministic sanitization (lowercase, spaces→dashes, strip illegal chars).

All paths are relative to vault root. `resolve_path()` rejects `..`, absolute paths, and other traversal via `Path::components()` checking. Tauri 2 auto-converts camelCase JS args to snake_case Rust params.

### Current Tauri Commands (config.rs)
- `get_config_info() -> ConfigInfo` — returns resolved config snapshot (paths, editor settings, warnings). Called once on startup.
- `read_config_file() -> String` — returns raw TOML contents for in-app editing.
- `write_config_file(contents)` — atomic write of edited config TOML back to disk.

## Core Design Decisions

### Title Is the Filename
- No frontmatter, no title field in file contents. The filename stem IS the note's identity.
- The editor shows an **editable title `<input>`** injected into CM6's `.cm-scroller` (Obsidian-style inline title). It scrolls with the document. Arrow keys navigate between title and editor as if they're one surface.
- Display: dashes/underscores → spaces (`project-kickoff.md` → "project kickoff"). Display-only; file on disk unchanged.
- Editing the title triggers a file rename on blur/Enter. Escape reverts. Rename fails gracefully if destination exists.
- If wikilinks reference the old name, show a confirmation dialog for bulk rewrite (not yet implemented).

### Filename Sanitization
A single deterministic function (defined in Rust, exposed via Tauri command) used everywhere: spaces → dashes, strip illegal characters (`/ \ : * ? " < > |`), lowercase everything.

### Inline Rendering (Not WYSIWYG)
The editor uses CodeMirror decorations to render markdown inline (bold appears bold, headers resize, links clickable) while raw markdown is accessible on cursor focus.

**MVP inline rendering:** headers, bold, italic, links, inline code, code blocks, blockquotes, horizontal rules.
**Deferred:** tables, footnotes, math blocks, embedded images.

### Navigation: Cmd+K Is Everything
- Cmd+K is the **unified** quick switcher for navigation AND creation.
- Default state (before typing) shows recent files.
- No sidebar in MVP.

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

### Config Over GUI
- All settings in a TOML config file (`~/.config/pithy/config.toml`). No settings panel.
- Cmd+, opens the config file in the app itself (reuses `MarkdownEditor` in "config" mode with a dedicated `configAutosave` instance).
- Config is self-documenting with comments. TOML keys use kebab-case (e.g., `font-size`, `font-family`).
- Config changes require app restart (MVP). The config bar shows a notice.
- On first launch, `load_or_create()` writes a commented default template.
- If config TOML is malformed, app falls back to defaults and shows a warning banner.

### Config Architecture
- **Structs pipeline:** `Config` (TOML parse) → `ResolvedConfig` (validated, held in `AppState`) → `ConfigInfo`/`EditorConfigInfo` (JSON to frontend).
- **`Config`** (`config.rs`): serde struct matching the TOML shape. Has `vault: VaultConfig` and `editor: EditorConfig`. All fields have `#[serde(default)]` so partial configs work.
- **`EditorConfig`**: uses `#[serde(rename_all = "kebab-case")]` for TOML keys. Embedded directly in `ResolvedConfig` (no field flattening).
- **`EditorConfigInfo`**: separate struct with `#[serde(rename_all = "camelCase")]` for JSON serialization to the frontend. Maps from `EditorConfig` in `get_config_info`.
- **`AppState`**: holds `Arc<ResolvedConfig>` + optional warning string. Managed as Tauri state.
- **Frontend**: `getConfigInfo()` is called once in `onMount`. Editor settings are applied as CSS custom properties (`--editor-font-size`, `--editor-font-family`, `--editor-line-height`) on `document.documentElement`. CodeMirror theme reads these vars.
- **Adding a new setting** requires 3 touch-points: `EditorConfig` + `EditorConfigInfo` (Rust), TS `EditorConfigInfo` interface, CSS var injection line. See `docs/adding-config-settings.md`.

### Search (Tantivy)
- Full-text search via Tantivy (Rust).
- Index stored in `.pithy/` dotfolder alongside the vault.
- Index fields: `filename_stem`, `body`, `tags` (structured), `path`, `modified_date`.
- Incremental re-index on file change events.
- Tags (`#tag`) indexed as structured metadata; tags inside code blocks/URLs excluded.

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
| Quick switcher (nav + create) | Cmd+K |
| Full-text search | Cmd+Shift+F |
| Daily note | Cmd+D |
| Open config | Cmd+, |
| Immediate save (flush) | Cmd+S |
| Quick capture (global) | Configurable |

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
- **CSS variables** — define on `:global(:root)` (not scoped `:root`) so they reach CodeMirror's shadow styles. Use `prefers-color-scheme: dark` media query for dark mode.
- **Async race guards** — use sequence counters (`openSeq`, `renameSeq`) for any async operation that sets state; check the counter after `await` to discard stale results.
- **Editor remounting** — wrap `MarkdownEditor` in `{#key currentPath}` so each file gets a fresh CodeMirror instance with clean undo history.
- **Config identifier:** `com.writepithy.app`
- **Autosave flush-before-switch** — always `await autosave.flushAndWait()` before opening a different file or renaming. After rename, call `autosave.setOpenedFile(newPath, doc)` to reset the baseline.
