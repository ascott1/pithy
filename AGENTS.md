# Pithy — Agent Context

## What Is This?

Pithy is a fast, focused markdown notes app for macOS desktop. The core philosophy: **get out of the way and let users write**. Think Ghostty's ethos applied to note-taking — speed, simplicity, no plugin sprawl.

**Target users:** Developers, writers, and technical professionals who think in markdown, want plain files on disk, and are frustrated by the weight of tools like Obsidian.

## Tech Stack

| Layer | Technology | Notes |
|---|---|---|
| Desktop framework | **Tauri 2** | Rust backend, native webview. No Electron. |
| Frontend framework | **Svelte 5** | Uses runes (`$state`, `$derived`, etc.). SvelteKit with `adapter-static` for SSG. |
| Editor | **CodeMirror 6** | Not yet integrated — will be added. |
| Search | **Tantivy** | Not yet integrated — Rust full-text search library. |
| Language (UI) | **TypeScript** | Strict mode enabled. |
| Language (backend) | **Rust** | Via Tauri commands. |
| Package manager | **pnpm** | |
| Build tool | **Vite 6** | Dev server on port 1420. |

## Project Structure

```
pithy/
├── src/                    # Frontend (Svelte/TypeScript)
│   ├── routes/             # SvelteKit routes
│   │   ├── +layout.ts      # SSR disabled (ssr = false, prerender = true)
│   │   └── +page.svelte    # Main page (currently Tauri boilerplate)
│   └── app.html            # HTML shell
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point — calls pithy_lib::run()
│   │   └── lib.rs          # Tauri builder, commands defined here
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri config (window size, app ID, build commands)
├── static/                 # Static assets served at /
├── package.json            # Node dependencies & scripts
├── svelte.config.js        # SvelteKit config with adapter-static
├── vite.config.js          # Vite config (Tauri dev server settings)
└── tsconfig.json           # TypeScript config (strict, bundler resolution)
```

## Current State

The project is a fresh Tauri 2 + SvelteKit + Svelte 5 scaffold. The boilerplate "greet" command and demo UI are still in place. No application logic has been implemented yet.

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

TypeScript/Svelte handles: UI rendering, editor state, CodeMirror extensions, keybinding dispatch, theme application.

Keep the IPC surface small — well-defined Tauri commands, not a chatty bridge.

## Core Design Decisions

### Title Is the Filename
- No frontmatter, no title field in file contents. The filename stem IS the note's identity.
- The editor shows an **editable title block** above the CodeMirror buffer (separate Svelte component, not part of the CM buffer).
- Display: dashes/underscores → spaces (`project-kickoff.md` → "project kickoff"). Display-only; file on disk unchanged.
- Editing the title triggers a file rename. If wikilinks reference the old name, show a confirmation dialog for bulk rewrite.

### Filename Sanitization
A single deterministic function (defined in Rust, exposed via Tauri command) used everywhere: spaces → dashes, strip illegal characters (`/ \ : * ? " < > |`), lowercase everything.

### Inline Rendering (Not WYSIWYG)
The editor uses CodeMirror decorations to render markdown inline (bold appears bold, headers resize, links clickable) while raw markdown is accessible on cursor focus. This is the Obsidian/HyperMD model, not Typora-style.

**MVP inline rendering:** headers, bold, italic, links, inline code, code blocks, blockquotes, horizontal rules.
**Deferred:** tables, footnotes, math blocks, embedded images.

### Navigation: Cmd+K Is Everything
- Cmd+K is the **unified** quick switcher for navigation AND creation.
- No Cmd+N. If the typed name doesn't match, a "Create" option appears.
- Default state (before typing) shows recent files.
- No sidebar in MVP.

### Wikilinks
- `[[wikilinks]]` resolve against filename stems, case-insensitive.
- Autocomplete triggered on `[[` keystroke, fuzzy-matches all filename stems.
- Following a link to a nonexistent note creates it.
- Disambiguation popup when multiple files share a stem (subdirectories).

### Storage
- Plain `.md` files in a user-configured vault directory.
- **Atomic writes:** write to temp file, then rename. Never risk partial writes.
- File watcher via `notify` crate for external changes.
- Ignore dotfiles and sync artifacts (`.git/`, `.DS_Store`, `*.icloud`, `*.conflict`).
- Sync is the user's responsibility (iCloud, Dropbox, Git).

### Config Over GUI
- All settings in a TOML config file (`~/.config/pithy/config.toml`). No settings panel.
- Cmd+, opens the config file in the app itself.
- Config is self-documenting with comments.

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
| Quick capture (global) | Configurable |

## What's Out of Scope for MVP

Graph view, block references/transclusion, frontmatter parsing, PDF/media, export, plugin system, mobile, GUI settings, sidebar, folder tree, collaboration, E2E encrypted sync.

## Milestone Phases

1. **Foundation (Phase 1):** Tauri scaffold, CodeMirror integration, file open/save with atomic writes, vault config, config file loading, title block display.
2. **Core Editing (Phase 2):** Inline rendering, title editing + rename, wikilink autocomplete, Vim mode, spellcheck.
3. **Navigation & Search (Phase 3):** Tantivy index, Cmd+K switcher, full-text search UI, tag indexing, recent files.
4. **Linking & Capture (Phase 4):** Wikilink follow/create, rename with link-rewrite, backlinks panel, daily notes, global capture window.
5. **Polish & Ship (Phase 5):** Themes (4 built-in), perf benchmarks, macOS polish, docs, packaging (DMG/Homebrew).

## Conventions to Follow

- **Svelte 5 runes** — use `$state`, `$derived`, `$effect`, not legacy `let` reactivity.
- **TypeScript strict mode** — no `any` unless absolutely necessary.
- **Rust Tauri commands** — keep IPC surface minimal. Each command does one well-defined thing.
- **Atomic file writes** — always write-to-temp-then-rename for any file mutation.
- **macOS-first** — follow platform conventions (Cmd shortcuts, system fonts, accent colors). Linux/Windows deferred.
- **No unnecessary abstractions** — build what's needed now. No plugin architecture, no extension points, no premature generalization.
- **Config identifier:** `com.writepithy.app`
