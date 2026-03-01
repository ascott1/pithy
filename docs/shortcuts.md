# Keyboard Shortcuts

All shortcuts listed here are for macOS. Pithy is macOS-first; Linux/Windows shortcuts are deferred.

## Global Shortcuts

These work anywhere in the app regardless of focus.

| Shortcut | Action | Notes |
|---|---|---|
| Cmd+K | Quick switcher | Toggle open/close. Navigate files, create new notes, delete current note. |
| Cmd+Shift+F | Full-text search | Toggle Tantivy-powered search panel. |
| Cmd+D | Daily note | Open or create today's daily note. Format configured via `[daily]` config. |
| Cmd+, | Open config | Opens `config.toml` in the editor (via native menu). |
| Cmd+Backspace | Delete current note | Moves current note to Trash. Shows confirmation dialog with backlink warnings. Only active in vault mode. |

## Markdown Formatting

These work in the editor in vault/markdown mode (not in config editing mode).

| Shortcut | Action | Notes |
|---|---|---|
| Cmd+B | Toggle bold | Wraps/unwraps selection with `**`. |
| Cmd+I | Toggle italic | Wraps/unwraps selection with `*`. |
| Cmd+E | Toggle inline code | Wraps/unwraps selection with `` ` ``. |
| Cmd+Shift+X | Toggle strikethrough | Wraps/unwraps selection with `~~`. |
| Cmd+Shift+C | Toggle code block | Wraps/unwraps selection with ``` fences. |
| Tab | Indent | Indents the current line or list item. |
| Shift+Tab | Outdent | Outdents the current line or list item. |

## Editor Shortcuts

These work when the CodeMirror editor has focus.

| Shortcut | Action | Notes |
|---|---|---|
| Cmd+S | Immediate save | Flushes autosave immediately. Autosave already runs ~350ms after typing stops. |
| Cmd+F | Find in document | Opens CodeMirror's built-in search panel. |
| Cmd+G | Find next | Standard CodeMirror search navigation. |
| Cmd+Shift+G | Find previous | Standard CodeMirror search navigation. |
| Cmd+H | Find and replace | Opens CodeMirror's search & replace panel. |
| Cmd+Z | Undo | Standard CodeMirror undo. |
| Cmd+Shift+Z | Redo | Standard CodeMirror redo. |

## Title Input Shortcuts

These work when the inline title input has focus.

| Shortcut | Action | Notes |
|---|---|---|
| Enter | Move focus to editor | Commits any title rename and focuses the editor body. |
| ArrowDown | Move focus to editor | Navigates from title into the editor body. |
| Escape | Revert title edit | Restores original title and blurs. |

## Quick Switcher (Cmd+K) Shortcuts

These work inside the quick switcher modal.

| Shortcut | Action | Notes |
|---|---|---|
| ArrowDown / ArrowUp | Navigate results | Cycles through file list with wrapping. |
| Enter | Select / Create / Delete | Selects highlighted file, creates new note (if "Create" highlighted), or deletes (if "Delete" highlighted). |
| Escape | Close switcher | Returns focus to editor. |

## Search Panel (Cmd+Shift+F) Shortcuts

These work inside the full-text search panel.

| Shortcut | Action | Notes |
|---|---|---|
| ArrowDown / ArrowUp | Navigate results | Cycles through search results with wrapping. |
| Enter | Open result | Opens the selected search result. |
| Escape | Close search | Returns focus to editor. |

## Backlinks Popover Shortcuts

These work when the backlinks popover is open.

| Shortcut | Action | Notes |
|---|---|---|
| ArrowDown / ArrowUp | Navigate backlinks | Cycles through backlink list with wrapping. |
| Enter | Open backlink | Opens the selected linking note. |
| Escape | Close popover | Dismisses the popover. |

## Dialog Shortcuts

These work in confirmation dialogs (delete confirmation, wikilink update).

| Shortcut | Action | Notes |
|---|---|---|
| Enter | Confirm action | Confirms the dialog action. |
| Escape | Cancel | Dismisses the dialog without action. |
