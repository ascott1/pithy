# Configuration

Pithy is configured via the Settings screen (**Cmd+,** or Pithy → Settings…). Changes apply live — no restart needed. Power users can also edit the TOML config file directly at the path shown below; the Settings UI reads and writes the same file.

## Location

```
~/.config/pithy/config.toml
```

Pithy creates this file with sensible defaults on first launch.

## Reference

### `version`

Config file format version. Do not change this.

```toml
version = 1
```

---

### `[vault]`

#### `dir`

Directory where your markdown notes are stored. Pithy creates the directory if it doesn't exist. Use an absolute path; `~` expands to your home directory.

```toml
[vault]
dir = "~/Documents/Pithy"
```

- **Type:** string (absolute path)
- **Default:** `~/Documents/Pithy`

---

### `[editor]`

#### `font-size`

Font size in pixels for the editor body text. Clamped to the range 8–48; out-of-range values fall back to the default.

```toml
[editor]
font-size = 15
```

- **Type:** integer
- **Default:** `15`
- **Range:** 8–48

#### `font-family`

CSS `font-family` value for the editor. Accepts a comma-separated list of font names. Quoted names with spaces are supported.

```toml
[editor]
font-family = '-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif'
```

To use a monospaced font:

```toml
[editor]
font-family = 'Iosevka, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'
```

- **Type:** string (CSS font-family)
- **Default:** `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif`

#### `line-height`

Line height multiplier for editor text. A unitless number (e.g., `1.7` means 1.7× the font size).

```toml
[editor]
line-height = 1.7
```

- **Type:** float
- **Default:** `1.7`

---

### `auto-update-links`

Controls whether Pithy automatically updates `[[wikilinks]]` in other notes when you rename a file. When `false`, a confirmation dialog is shown before updating references.

```toml
auto-update-links = true
```

- **Type:** boolean
- **Default:** `true`

---

### `[theme]`

Controls the color theme. Pithy ships with `default-light` and `default-dark` built-in themes. Custom themes are `.css` files placed in `~/.config/pithy/themes/`.

#### `mode`

Theme mode. `"auto"` follows the OS light/dark setting, `"light"` or `"dark"` forces one.

```toml
[theme]
mode = "auto"
```

- **Type:** `"auto"` | `"light"` | `"dark"`
- **Default:** `"auto"`

#### `light`

Name of the theme to use in light mode. References a `.css` file in `~/.config/pithy/themes/` or a built-in theme.

```toml
[theme]
light = "default-light"
```

- **Type:** string (theme name)
- **Default:** `"default-light"`

#### `dark`

Name of the theme to use in dark mode.

```toml
[theme]
dark = "default-dark"
```

- **Type:** string (theme name)
- **Default:** `"default-dark"`

---

### `[daily]`

Configures daily notes created with **Cmd+D**.

#### `dir`

Subdirectory for daily notes, relative to the vault root. Notes are created inside this folder.

```toml
[daily]
dir = "daily"
```

- **Type:** string (relative path)
- **Default:** `"daily"`

#### `format`

Filename format for daily notes. Supports `YYYY`, `MM`, `DD` tokens.

```toml
[daily]
format = "YYYY-MM-DD"
```

- **Type:** string (date format)
- **Default:** `"YYYY-MM-DD"`

---

### `[status-bar]`

Controls the info bar displayed at the bottom of the editor.

#### `show`

Show or hide the entire status bar.

```toml
[status-bar]
show = true
```

- **Type:** boolean
- **Default:** `true`

#### `show-backlinks`

Show the backlinks count in the status bar.

```toml
[status-bar]
show-backlinks = true
```

- **Type:** boolean
- **Default:** `true`

#### `show-word-count`

Show the word count in the status bar.

```toml
[status-bar]
show-word-count = true
```

- **Type:** boolean
- **Default:** `true`

---

## Full Example

```toml
version = 1
auto-update-links = true

[vault]
dir = "~/Notes"

[editor]
font-size = 16
font-family = 'Iosevka, ui-monospace, monospace'
line-height = 1.8

[theme]
mode = "auto"
light = "default-light"
dark = "default-dark"

[daily]
dir = "daily"
format = "YYYY-MM-DD"

[status-bar]
show = true
show-backlinks = true
show-word-count = true
```

## Troubleshooting

- **Invalid TOML:** If the config file can't be parsed, Pithy falls back to defaults and shows a warning banner.
- **Relative vault path:** The vault `dir` must be absolute (or start with `~`). Pithy will refuse to start if it resolves to a relative path.
- **Changes not applying:** All config changes require a full restart of Pithy.
