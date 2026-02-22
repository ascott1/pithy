# Adding a New Config Setting

This guide walks through adding a new editor setting end-to-end. The pipeline has **3 touch-points**.

## Example: Adding `tab-size`

### 1. Rust — `src-tauri/src/config.rs`

**a) Add a default function and the field to `EditorConfig`:**

```rust
fn default_editor_tab_size() -> u32 {
    4
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EditorConfig {
    // ... existing fields ...
    #[serde(default = "default_editor_tab_size")]
    pub tab_size: u32,
}
```

Update the `Default` impl to include `tab_size: default_editor_tab_size()`.

**b) Add the field to `EditorConfigInfo`** (the camelCase struct serialized to the frontend):

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigInfo {
    // ... existing fields ...
    pub tab_size: u32,
}
```

**c) Map the field in `get_config_info`:**

```rust
editor: EditorConfigInfo {
    // ... existing fields ...
    tab_size: state.config.editor.tab_size,
},
```

**d) (Optional) Add validation in `load_or_create_at`:**

```rust
let mut editor = config.editor;
if !(2..=8).contains(&editor.tab_size) {
    editor.tab_size = default_editor_tab_size();
}
```

**e) Add the setting (commented out) to `DEFAULT_TEMPLATE`:**

```toml
[editor]
# ...existing settings...

# Tab size (number of spaces).
# tab-size = 4
```

Note: TOML keys use **kebab-case** (e.g., `tab-size`, not `tab_size`).

### 2. TypeScript — `src/lib/tauri/config.ts`

Add the field to `EditorConfigInfo`:

```ts
export interface EditorConfigInfo {
    // ... existing fields ...
    tabSize: number;
}
```

### 3. Frontend — `src/routes/+page.svelte`

Apply the value as a CSS custom property in `onMount`:

```ts
document.documentElement.style.setProperty("--editor-tab-size", `${info.editor.tabSize}`);
```

Add a default in the `:global(:root)` CSS block:

```css
--editor-tab-size: 4;
```

Then use `var(--editor-tab-size)` in `MarkdownEditor.svelte`'s CodeMirror theme or wherever the setting applies.

### 4. Docs — `docs/configuration.md`

Add the new setting to the `[editor]` section and the full example.

## Checklist

- [ ] Default function + field on `EditorConfig` (with `#[serde(default)]`)
- [ ] `Default` impl updated
- [ ] Field on `EditorConfigInfo`
- [ ] Mapped in `get_config_info`
- [ ] Validation/clamping in `load_or_create_at` (if applicable)
- [ ] Added (commented out) to `DEFAULT_TEMPLATE`
- [ ] TS `EditorConfigInfo` interface updated
- [ ] CSS custom property set in `onMount` + default in `:global(:root)`
- [ ] Used in component (e.g., CM theme)
- [ ] `docs/configuration.md` updated
- [ ] `cargo test` passes
- [ ] `pnpm check` passes

## Architecture Notes

- **`EditorConfig`** uses `#[serde(rename_all = "kebab-case")]` — Rust field names are `snake_case`, TOML keys are `kebab-case`. This is automatic.
- **`EditorConfigInfo`** uses `#[serde(rename_all = "camelCase")]` — Rust field names are `snake_case`, JSON keys are `camelCase`. This is automatic.
- **`EditorConfig` is embedded directly in `ResolvedConfig`** — no need to flatten individual fields. New fields flow through automatically.
- **CSS custom properties** are the bridge between config values and CodeMirror. Define on `:global(:root)` so they penetrate CM's scoped styles.
- **All settings require restart** (MVP). No live-reload mechanism yet.
