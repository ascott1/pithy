use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_TEMPLATE: &str = r#"# Pithy configuration
# This file is read on startup.
# After editing, restart Pithy to apply changes.
#
# Location: ~/.config/pithy/config.toml

version = 1

[vault]
# Directory where your markdown notes live.
# Use an absolute path. "~" expands to your home directory.
# Pithy will create the directory if it doesn't exist.
# Changing this requires restarting Pithy.
dir = "~/Documents/Pithy"

# Automatically update [[wikilinks]] in other notes when you rename a file.
# Set to false to be prompted before updating.
# auto-update-links = true

[editor]
# Font size in pixels for the editor body text.
# font-size = 15

# Font family (CSS font-family value).
# font-family = '-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif'

# Line height multiplier for editor text.
# line-height = 1.7

# [theme]
# Color theme mode: "auto" follows OS light/dark, "light" or "dark" forces one.
# mode = "auto"
#
# Theme names reference .css files in ~/.config/pithy/themes/
# Built-in themes: "default-light", "default-dark"
# light = "default-light"
# dark = "default-dark"

# [daily]
# Subdirectory for daily notes (relative to vault root).
# dir = "daily"
#
# Filename format for daily notes. Supports YYYY, MM, DD tokens.
# format = "YYYY-MM-DD"

# [status-bar]
# Show the info bar below the editor.
# show = true
#
# Show backlinks count in the info bar.
# show-backlinks = true
#
# Show word count in the info bar.
# show-word-count = true
"#;

const DEFAULT_LIGHT_CSS: &str = r#":root {
  --editor-bg: #ffffff;
  --editor-text: #37352f;
  --editor-cursor: #37352f;
  --editor-selection: #d3e0f0;
  --accent-color: #2383e2;
  --dirty-color: #d9730d;
  --link-color: #2383e2;
  --error-color: #c4463a;
  --code-bg: rgba(135, 131, 120, 0.1);
  --code-block-bg: rgba(135, 131, 120, 0.04);
  --border-color: rgba(55, 53, 47, 0.16);
  --backdrop-color: rgba(15, 15, 15, 0.6);
  --shadow-color: rgba(15, 15, 15, 0.1);
  --tag-color: #2383e2;
  --tag-bg: rgba(35, 131, 226, 0.08);
}
"#;

const DEFAULT_DARK_CSS: &str = r#":root {
  --editor-bg: #1c1c1e;
  --editor-text: #d1d1d6;
  --editor-cursor: #e5e5ea;
  --editor-selection: #2c3a50;
  --accent-color: #5a9cf5;
  --dirty-color: #e09430;
  --link-color: #5a9cf5;
  --error-color: #e05545;
  --code-bg: rgba(180, 180, 195, 0.08);
  --code-block-bg: rgba(180, 180, 195, 0.04);
  --border-color: rgba(180, 180, 195, 0.15);
  --backdrop-color: rgba(0, 0, 0, 0.50);
  --shadow-color: rgba(0, 0, 0, 0.40);
  --tag-color: #5a9cf5;
  --tag-bg: rgba(90, 156, 245, 0.10);
}
"#;

fn default_status_bar_bool() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusBarConfig {
    #[serde(default = "default_status_bar_bool")]
    pub show: bool,
    #[serde(default = "default_status_bar_bool")]
    pub show_backlinks: bool,
    #[serde(default = "default_status_bar_bool")]
    pub show_word_count: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            show: true,
            show_backlinks: true,
            show_word_count: true,
        }
    }
}

fn default_auto_update_links() -> bool {
    true
}

fn default_daily_dir() -> String {
    "daily".into()
}

fn default_daily_format() -> String {
    "YYYY-MM-DD".into()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DailyConfig {
    #[serde(default = "default_daily_dir")]
    pub dir: String,
    #[serde(default = "default_daily_format")]
    pub format: String,
}

impl Default for DailyConfig {
    fn default() -> Self {
        Self {
            dir: default_daily_dir(),
            format: default_daily_format(),
        }
    }
}

fn default_theme_mode() -> String {
    "auto".into()
}

fn default_theme_light() -> String {
    "default-light".into()
}

fn default_theme_dark() -> String {
    "default-dark".into()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ThemeConfig {
    #[serde(default = "default_theme_mode")]
    pub mode: String,
    #[serde(default = "default_theme_light")]
    pub light: String,
    #[serde(default = "default_theme_dark")]
    pub dark: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: default_theme_mode(),
            light: default_theme_light(),
            dark: default_theme_dark(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub vault: VaultConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub daily: DailyConfig,
    #[serde(default = "default_auto_update_links")]
    pub auto_update_links: bool,
    #[serde(default)]
    pub status_bar: StatusBarConfig,
}

fn default_version() -> u32 {
    1
}

fn default_vault_dir() -> String {
    "~/Documents/Pithy".into()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VaultConfig {
    #[serde(default = "default_vault_dir")]
    pub dir: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            dir: default_vault_dir(),
        }
    }
}

fn default_editor_font_size() -> u32 {
    15
}

fn default_editor_font_family() -> String {
    r#"-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif"#.to_string()
}

fn default_editor_line_height() -> f64 {
    1.7
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EditorConfig {
    #[serde(default = "default_editor_font_size")]
    pub font_size: u32,
    #[serde(default = "default_editor_font_family")]
    pub font_family: String,
    #[serde(default = "default_editor_line_height")]
    pub line_height: f64,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_size: default_editor_font_size(),
            font_family: default_editor_font_family(),
            line_height: default_editor_line_height(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            vault: VaultConfig::default(),
            editor: EditorConfig::default(),
            theme: ThemeConfig::default(),
            daily: DailyConfig::default(),
            auto_update_links: default_auto_update_links(),
            status_bar: StatusBarConfig::default(),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub config_path: PathBuf,
    pub vault_dir_raw: String,
    pub vault_dir: PathBuf,
    pub editor: EditorConfig,
    pub theme_mode: String,
    pub theme_light_css: String,
    pub theme_dark_css: String,
    pub daily: DailyConfig,
    pub auto_update_links: bool,
    pub status_bar: StatusBarConfig,
}

pub struct AppState {
    pub config: Arc<ResolvedConfig>,
    pub config_warning: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigInfo {
    pub font_size: u32,
    pub font_family: String,
    pub line_height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfigInfo {
    pub mode: String,
    pub light_css: String,
    pub dark_css: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyConfigInfo {
    pub dir: String,
    pub format: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarConfigInfo {
    pub show: bool,
    pub show_backlinks: bool,
    pub show_word_count: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInfo {
    pub config_path: String,
    pub vault_dir: String,
    pub vault_dir_display: String,
    pub warning: Option<String>,
    pub editor: EditorConfigInfo,
    pub theme: ThemeConfigInfo,
    pub daily: DailyConfigInfo,
    pub auto_update_links: bool,
    pub status_bar: StatusBarConfigInfo,
}

fn expand_tilde(path: &str, home: &str) -> String {
    if path == "~" {
        home.to_string()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{}", home, rest)
    } else {
        path.to_string()
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let temp_name = format!(
        ".{}.pithy-tmp",
        path.file_name().unwrap().to_string_lossy()
    );
    let temp_path = path.with_file_name(temp_name);

    let mut file = File::create(&temp_path).map_err(|e| e.to_string())?;

    if let Err(e) = file.write_all(contents) {
        let _ = fs::remove_file(&temp_path);
        return Err(e.to_string());
    }

    if let Err(e) = file.sync_all() {
        let _ = fs::remove_file(&temp_path);
        return Err(e.to_string());
    }

    if let Err(e) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e.to_string());
    }

    // fsync parent directory for durability
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}

/// Resolves a theme name to its CSS content.
/// Built-in names ("default-light", "default-dark") return embedded constants.
/// Other names are loaded from `~/.config/pithy/themes/{name}.css`.
fn resolve_theme_css(
    name: &str,
    themes_dir: &Path,
    warnings: &mut Vec<String>,
    fallback_builtin: &str,
) -> String {
    match name {
        "default-light" => return DEFAULT_LIGHT_CSS.to_string(),
        "default-dark" => return DEFAULT_DARK_CSS.to_string(),
        _ => {}
    }

    let stem = name.strip_suffix(".css").unwrap_or(name);
    let file_path = themes_dir.join(format!("{}.css", stem));

    match fs::read_to_string(&file_path) {
        Ok(css) => css,
        Err(_) => {
            warnings.push(format!(
                "Theme \"{}\" not found (looked for {}), using built-in",
                name,
                file_path.display()
            ));
            match fallback_builtin {
                "default-dark" => DEFAULT_DARK_CSS.to_string(),
                _ => DEFAULT_LIGHT_CSS.to_string(),
            }
        }
    }
}

pub fn config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("pithy")
        .join("config.toml"))
}

fn load_or_create_at(
    config_path: &Path,
    home: &str,
) -> Result<(ResolvedConfig, Option<String>), String> {
    let mut warnings: Vec<String> = Vec::new();

    let config = if config_path.exists() {
        let raw = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
        match toml::from_str::<Config>(&raw) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!(
                    "Failed to parse config (using defaults): {}",
                    e
                ));
                Config::default()
            }
        }
    } else {
        atomic_write(config_path, DEFAULT_TEMPLATE.as_bytes())?;
        Config::default()
    };

    let expanded = expand_tilde(&config.vault.dir, home);
    let vault_dir = PathBuf::from(&expanded);

    if !vault_dir.is_absolute() {
        return Err(format!(
            "Vault directory must be an absolute path (got: {})",
            config.vault.dir
        ));
    }

    let mut editor = config.editor;
    if !(8..=48).contains(&editor.font_size) {
        editor.font_size = default_editor_font_size();
    }

    // Resolve theme
    let theme_mode = match config.theme.mode.as_str() {
        "auto" | "light" | "dark" => config.theme.mode.clone(),
        other => {
            warnings.push(format!(
                "Invalid theme mode \"{}\" (expected auto, light, or dark), using auto",
                other
            ));
            "auto".to_string()
        }
    };

    let themes_dir = config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("themes");
    let _ = fs::create_dir_all(&themes_dir);

    let theme_light_css =
        resolve_theme_css(&config.theme.light, &themes_dir, &mut warnings, "default-light");
    let theme_dark_css =
        resolve_theme_css(&config.theme.dark, &themes_dir, &mut warnings, "default-dark");

    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    Ok((
        ResolvedConfig {
            config_path: config_path.to_path_buf(),
            vault_dir_raw: config.vault.dir.clone(),
            vault_dir,
            editor,
            theme_mode,
            theme_light_css,
            theme_dark_css,
            daily: config.daily,
            auto_update_links: config.auto_update_links,
            status_bar: config.status_bar,
        },
        warning,
    ))
}

pub fn load_or_create() -> Result<(ResolvedConfig, Option<String>), String> {
    let path = config_path()?;
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
    load_or_create_at(&path, &home)
}

#[tauri::command]
pub fn get_config_info(
    state: tauri::State<'_, AppState>,
) -> Result<ConfigInfo, String> {
    Ok(ConfigInfo {
        config_path: state.config.config_path.to_string_lossy().into_owned(),
        vault_dir: state.config.vault_dir.to_string_lossy().into_owned(),
        vault_dir_display: state.config.vault_dir_raw.clone(),
        warning: state.config_warning.clone(),
        editor: EditorConfigInfo {
            font_size: state.config.editor.font_size,
            font_family: state.config.editor.font_family.clone(),
            line_height: state.config.editor.line_height,
        },
        theme: ThemeConfigInfo {
            mode: state.config.theme_mode.clone(),
            light_css: state.config.theme_light_css.clone(),
            dark_css: state.config.theme_dark_css.clone(),
        },
        daily: DailyConfigInfo {
            dir: state.config.daily.dir.clone(),
            format: state.config.daily.format.clone(),
        },
        auto_update_links: state.config.auto_update_links,
        status_bar: StatusBarConfigInfo {
            show: state.config.status_bar.show,
            show_backlinks: state.config.status_bar.show_backlinks,
            show_word_count: state.config.status_bar.show_word_count,
        },
    })
}

#[tauri::command]
pub fn read_config_file(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    fs::read_to_string(&state.config.config_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_config_file(
    contents: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    atomic_write(&state.config.config_path, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn tilde_expansion_home_only() {
        assert_eq!(expand_tilde("~", "/Users/test"), "/Users/test");
    }

    #[test]
    fn tilde_expansion_with_subpath() {
        assert_eq!(
            expand_tilde("~/Documents/Pithy", "/Users/test"),
            "/Users/test/Documents/Pithy"
        );
    }

    #[test]
    fn tilde_expansion_no_tilde() {
        assert_eq!(
            expand_tilde("/absolute/path", "/Users/test"),
            "/absolute/path"
        );
    }

    #[test]
    fn tilde_expansion_tilde_not_at_start() {
        assert_eq!(
            expand_tilde("foo/~/bar", "/Users/test"),
            "foo/~/bar"
        );
    }

    #[test]
    fn config_path_returns_correct_path() {
        let path = config_path().unwrap();
        assert!(path.ends_with(".config/pithy/config.toml"));
    }

    #[test]
    fn load_or_create_creates_default_when_missing() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/Users/test").unwrap();

        assert!(warning.is_none());
        assert!(cfg_path.exists());
        assert_eq!(
            resolved.vault_dir,
            PathBuf::from("/Users/test/Documents/Pithy")
        );
        assert_eq!(resolved.config_path, cfg_path);

        // Verify the template was written
        let contents = fs::read_to_string(&cfg_path).unwrap();
        assert!(contents.contains("version = 1"));
        assert!(contents.contains("[vault]"));
    }

    #[test]
    fn load_or_create_with_valid_config() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert!(warning.is_none());
        assert_eq!(resolved.vault_dir, PathBuf::from("/home/user/Notes"));
    }

    #[test]
    fn load_or_create_invalid_toml_falls_back_with_warning() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        fs::write(&cfg_path, "this is [not valid {toml").unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/Users/test").unwrap();

        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Failed to parse config"));
        assert_eq!(
            resolved.vault_dir,
            PathBuf::from("/Users/test/Documents/Pithy")
        );
    }

    #[test]
    fn load_or_create_rejects_relative_vault_dir() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let bad = r#"
version = 1

[vault]
dir = "relative/path"
"#;
        fs::write(&cfg_path, bad).unwrap();

        let result = load_or_create_at(&cfg_path, "/Users/test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute path"));
    }

    #[test]
    fn daily_config_defaults() {
        let daily = DailyConfig::default();
        assert_eq!(daily.dir, "daily");
        assert_eq!(daily.format, "YYYY-MM-DD");
    }

    #[test]
    fn daily_config_custom_values() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[daily]
dir = "journal"
format = "DD-MM-YYYY"
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert!(warning.is_none());
        assert_eq!(resolved.daily.dir, "journal");
        assert_eq!(resolved.daily.format, "DD-MM-YYYY");
    }

    #[test]
    fn daily_config_partial_uses_defaults() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[daily]
dir = "notes"
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, _) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert_eq!(resolved.daily.dir, "notes");
        assert_eq!(resolved.daily.format, "YYYY-MM-DD");
    }

    #[test]
    fn load_or_create_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("nested").join("dir").join("config.toml");

        let (_, warning) =
            load_or_create_at(&cfg_path, "/Users/test").unwrap();

        assert!(warning.is_none());
        assert!(cfg_path.exists());
    }

    // --- Theme tests ---

    #[test]
    fn resolve_builtin_light_theme() {
        let dir = tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        let mut warnings = Vec::new();
        let css = resolve_theme_css("default-light", &themes_dir, &mut warnings, "default-light");
        assert!(css.contains("--editor-bg: #ffffff"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_builtin_dark_theme() {
        let dir = tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        let mut warnings = Vec::new();
        let css = resolve_theme_css("default-dark", &themes_dir, &mut warnings, "default-dark");
        assert!(css.contains("--editor-bg: #1c1c1e"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_custom_theme_file() {
        let dir = tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(themes_dir.join("github.css"), ":root { --editor-bg: #fff; }").unwrap();

        let mut warnings = Vec::new();
        let css = resolve_theme_css("github", &themes_dir, &mut warnings, "default-light");
        assert!(css.contains("--editor-bg: #fff"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_custom_theme_with_css_extension() {
        let dir = tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(themes_dir.join("github.css"), ":root { --editor-bg: #fff; }").unwrap();

        let mut warnings = Vec::new();
        let css = resolve_theme_css("github.css", &themes_dir, &mut warnings, "default-light");
        assert!(css.contains("--editor-bg: #fff"));
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_missing_theme_falls_back_with_warning() {
        let dir = tempdir().unwrap();
        let themes_dir = dir.path().join("themes");
        fs::create_dir_all(&themes_dir).unwrap();

        let mut warnings = Vec::new();
        let css = resolve_theme_css("nonexistent", &themes_dir, &mut warnings, "default-dark");
        assert!(css.contains("--editor-bg: #1c1c1e")); // fell back to dark
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("not found"));
    }

    #[test]
    fn invalid_theme_mode_warns_and_defaults_to_auto() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[theme]
mode = "invalid"
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert_eq!(resolved.theme_mode, "auto");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Invalid theme mode"));
    }

    #[test]
    fn default_config_has_default_themes() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/Users/test").unwrap();

        assert!(warning.is_none());
        assert_eq!(resolved.theme_mode, "auto");
        assert!(resolved.theme_light_css.contains("--editor-bg: #ffffff"));
        assert!(resolved.theme_dark_css.contains("--editor-bg: #1c1c1e"));
    }

    #[test]
    fn themes_directory_created_on_load() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        load_or_create_at(&cfg_path, "/Users/test").unwrap();

        let themes_dir = dir.path().join("themes");
        assert!(themes_dir.exists());
        assert!(themes_dir.is_dir());
    }

    #[test]
    fn custom_theme_in_config() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");
        let themes_dir = dir.path().join("themes");
        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(themes_dir.join("solarized.css"), ":root { --editor-bg: #fdf6e3; }").unwrap();

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[theme]
mode = "light"
light = "solarized"
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert!(warning.is_none());
        assert_eq!(resolved.theme_mode, "light");
        assert!(resolved.theme_light_css.contains("--editor-bg: #fdf6e3"));
    }

    #[test]
    fn default_template_contains_theme_section() {
        assert!(DEFAULT_TEMPLATE.contains("[theme]"));
        assert!(DEFAULT_TEMPLATE.contains("mode = \"auto\""));
    }

    // --- Status bar tests ---

    #[test]
    fn status_bar_defaults_all_true() {
        let sb = StatusBarConfig::default();
        assert!(sb.show);
        assert!(sb.show_backlinks);
        assert!(sb.show_word_count);
    }

    #[test]
    fn status_bar_custom_values() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[status-bar]
show = true
show-backlinks = false
show-word-count = false
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, warning) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert!(warning.is_none());
        assert!(resolved.status_bar.show);
        assert!(!resolved.status_bar.show_backlinks);
        assert!(!resolved.status_bar.show_word_count);
    }

    #[test]
    fn status_bar_partial_uses_defaults() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        let custom = r#"
version = 1

[vault]
dir = "~/Notes"

[status-bar]
show-backlinks = false
"#;
        fs::write(&cfg_path, custom).unwrap();

        let (resolved, _) =
            load_or_create_at(&cfg_path, "/home/user").unwrap();

        assert!(resolved.status_bar.show);
        assert!(!resolved.status_bar.show_backlinks);
        assert!(resolved.status_bar.show_word_count);
    }

    #[test]
    fn default_template_contains_status_bar_section() {
        assert!(DEFAULT_TEMPLATE.contains("[status-bar]"));
        assert!(DEFAULT_TEMPLATE.contains("show = true"));
    }
}
