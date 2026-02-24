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
"#;

fn default_auto_update_links() -> bool {
    true
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
    #[serde(default = "default_auto_update_links")]
    pub auto_update_links: bool,
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
            auto_update_links: default_auto_update_links(),
        }
    }
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub config_path: PathBuf,
    pub vault_dir_raw: String,
    pub vault_dir: PathBuf,
    pub editor: EditorConfig,
    pub auto_update_links: bool,
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
pub struct ConfigInfo {
    pub config_path: String,
    pub vault_dir: String,
    pub vault_dir_display: String,
    pub warning: Option<String>,
    pub editor: EditorConfigInfo,
    pub auto_update_links: bool,
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
    let mut warning: Option<String> = None;

    let config = if config_path.exists() {
        let raw = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
        match toml::from_str::<Config>(&raw) {
            Ok(c) => c,
            Err(e) => {
                warning = Some(format!(
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

    Ok((
        ResolvedConfig {
            config_path: config_path.to_path_buf(),
            vault_dir_raw: config.vault.dir.clone(),
            vault_dir,
            editor,
            auto_update_links: config.auto_update_links,
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
        auto_update_links: state.config.auto_update_links,
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
    fn load_or_create_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("nested").join("dir").join("config.toml");

        let (_, warning) =
            load_or_create_at(&cfg_path, "/Users/test").unwrap();

        assert!(warning.is_none());
        assert!(cfg_path.exists());
    }
}
