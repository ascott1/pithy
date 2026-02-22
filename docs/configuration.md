# Configuration

Pithy is configured entirely through a single TOML file. There is no settings UI — open the config file with **Cmd+,** (or Pithy → Settings…) to edit it directly in the app.

**All changes require restarting Pithy to take effect.**

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

## Full Example

```toml
version = 1

[vault]
dir = "~/Notes"

[editor]
font-size = 16
font-family = 'Iosevka, ui-monospace, monospace'
line-height = 1.8
```

## Troubleshooting

- **Invalid TOML:** If the config file can't be parsed, Pithy falls back to defaults and shows a warning banner.
- **Relative vault path:** The vault `dir` must be absolute (or start with `~`). Pithy will refuse to start if it resolves to a relative path.
- **Changes not applying:** All config changes require a full restart of Pithy.
