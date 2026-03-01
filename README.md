# Pithy

Pithy is a fast, focused markdown notes app for macOS that stays out of your way. It's built to be a [home cooked app](https://www.robinsloan.com/lab/five-years-of-home-cooked-apps/), that meets my needs and preferences.

## Features

- Notes are plain markdown files
- Inline markdown rendering
- Cmd+K quick switcher
- Full-text search
- Wikilinks
- Backlinks
- Daily notes
- Autosave
- Themes
- Config in a single TOML file (`~/.config/pithy/config.toml`)
- Keyboard-driven

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Cmd+K | Quick switcher (navigate, create, delete) |
| Cmd+Shift+F | Full-text search |
| Cmd+D | Daily note |
| Cmd+, | Open config |
| Cmd+Backspace | Delete current note |
| Cmd+S | Immediate save |
| Cmd+B | Bold |
| Cmd+I | Italic |
| Cmd+Shift+K | Inline code |
| Cmd+Shift+X | Strikethrough |

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop framework | [Tauri 2](https://tauri.app/) (Rust backend, native webview) |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript |
| Editor | [CodeMirror 6](https://codemirror.net/) |
| Search engine | [Tantivy](https://github.com/quickwit-oss/tantivy) |
| Build tool | [Vite](https://vite.dev/) |
| Package manager | [pnpm](https://pnpm.io/) |

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/)
- [pnpm](https://pnpm.io/installation)
- macOS

### Development

```bash
# Install dependencies
pnpm install

# Start the dev server with Tauri window
pnpm tauri dev

# Run tests
pnpm test

# Type check
pnpm check
```

### Building

```bash
pnpm tauri build
```

## Project Structure

```
pithy/
├── src/                    # Frontend (Svelte/TypeScript)
│   ├── lib/                # Components, editor, Tauri wrappers
│   └── routes/             # SvelteKit pages
├── src-tauri/              # Rust backend
│   └── src/                # Tauri commands (filesystem, config, search)
├── docs/                   # Developer documentation
└── static/                 # Static assets
```

## License

[MIT](LICENSE)
