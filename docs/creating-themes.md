# Creating Themes

Pithy themes are CSS files that define color variables. Place them in `~/.config/pithy/themes/` and reference them in your config.

## Quick Start

1. Create a file at `~/.config/pithy/themes/my-theme.css`
2. Define CSS custom properties inside a `:root {}` block
3. Set it in `~/.config/pithy/config.toml`:

```toml
[theme]
mode = "light"
light = "my-theme"
```

4. Restart Pithy

## Config Options

```toml
[theme]
# "auto" (default): follows OS light/dark preference
# "light": always use the light theme
# "dark": always use the dark theme
mode = "auto"

# Theme names reference .css files in ~/.config/pithy/themes/
# Built-in: "default-light", "default-dark"
light = "default-light"
dark = "default-dark"
```

- `mode = "auto"` uses `light` theme when OS is light, `dark` theme when OS is dark
- `mode = "light"` always uses `light` theme regardless of OS
- `mode = "dark"` always uses `dark` theme regardless of OS
- Theme names can be specified with or without `.css` extension (`"github"` and `"github.css"` both work)
- Missing theme files fall back to the built-in default with a warning

## Theme File Format

A theme is a `.css` file containing a `:root {}` block with CSS custom properties:

```css
:root {
  --editor-bg: #ffffff;
  --editor-text: #24292e;
  --editor-cursor: #24292e;
  --editor-selection: #c8e1ff;
  --accent-color: #0366d6;
  --link-color: #0366d6;
  --dirty-color: #e36209;
  --error-color: #cb2431;
  --code-bg: rgba(27, 31, 35, 0.05);
  --code-block-bg: rgba(27, 31, 35, 0.03);
  --border-color: rgba(27, 31, 35, 0.15);
  --backdrop-color: rgba(0, 0, 0, 0.25);
  --shadow-color: rgba(0, 0, 0, 0.15);
}
```

No `@media` queries needed — the app handles light/dark selection.

## Variable Reference

| Variable | Description | Example (light) | Example (dark) |
|---|---|---|---|
| `--editor-bg` | Editor and app background | `#ffffff` | `#1e1c1a` |
| `--editor-text` | Main text color | `#37352f` | `#d1ccc5` |
| `--editor-cursor` | Text cursor color | `#37352f` | `#c8c2ba` |
| `--editor-selection` | Text selection highlight | `#d3e0f0` | `#2e3d55` |
| `--accent-color` | UI accents (active toggles, focus rings) | `#2383e2` | `#7b8fd4` |
| `--link-color` | Links and wikilinks | `#2383e2` | `#7b8fd4` |
| `--dirty-color` | Unsaved changes indicator | `#d9730d` | `#d4943a` |
| `--error-color` | Error messages and states | `#c4463a` | `#d4574b` |
| `--code-bg` | Inline code background | `rgba(135,131,120,0.1)` | `rgba(200,195,185,0.08)` |
| `--code-block-bg` | Fenced code block background | `rgba(135,131,120,0.04)` | `rgba(200,195,185,0.04)` |
| `--border-color` | Blockquote borders, horizontal rules | `rgba(55,53,47,0.16)` | `rgba(200,195,185,0.18)` |
| `--backdrop-color` | Modal overlay background | `rgba(15,15,15,0.6)` | `rgba(0,0,0,0.45)` |
| `--shadow-color` | Box shadow color | `rgba(15,15,15,0.1)` | `rgba(0,0,0,0.35)` |
| `--tag-color` | Hashtag text color | `#2383e2` | `#5a9cf5` |
| `--tag-bg` | Hashtag background | `rgba(35,131,226,0.08)` | `rgba(90,156,245,0.10)` |

Font settings (`--editor-font-size`, `--editor-font-family`, `--editor-line-height`) are controlled by the `[editor]` config section, not themes.

## Tips

- You only need to set the variables you want to change — unset variables use the built-in defaults
- Use `rgba()` for translucent values (code backgrounds, borders, backdrops)
- Test both the editor and modals (Cmd+K, Cmd+Shift+F) when designing a theme
- The `color-mix()` function is used throughout the UI relative to `--editor-text` and `--editor-bg`, so those two values have the most impact
