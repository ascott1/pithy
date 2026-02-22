use crate::config::AppState;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

fn resolve_path(vault_dir: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(rel_path);

    if rel.is_absolute() {
        return Err("Invalid path".into());
    }

    for c in rel.components() {
        match c {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("Invalid path".into());
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(vault_dir.join(rel))
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

fn sanitize_filename_impl(name: &str) -> String {
    let mut s = name.trim().to_lowercase();

    if let Some(stripped) = s.strip_suffix(".md") {
        s = stripped.to_string();
    }

    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;

    for ch in s.chars() {
        if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            continue;
        }

        if ch.is_whitespace() || ch == '_' || ch == '-' {
            if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }

        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        }
    }

    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "untitled".into()
    } else {
        out
    }
}

#[tauri::command]
pub fn sanitize_filename(name: String) -> Result<String, String> {
    Ok(sanitize_filename_impl(&name))
}

#[tauri::command]
pub fn rename_file(
    old_rel_path: String,
    new_rel_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let vault = &state.config.vault_dir;
    let old_path = resolve_path(vault, &old_rel_path)?;
    let new_path = resolve_path(vault, &new_rel_path)?;

    if !old_path.exists() {
        return Err("Source file does not exist".into());
    }
    if new_path.exists() {
        return Err("A note with that name already exists".into());
    }

    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_files(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let vault = &state.config.vault_dir;
    fs::create_dir_all(&vault).map_err(|e| e.to_string())?;

    let mut files: Vec<String> = WalkDir::new(&vault)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext == "md")
        })
        .filter_map(|e| {
            e.path()
                .strip_prefix(&vault)
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .collect();

    if files.is_empty() {
        let welcome = vault.join("welcome.md");
        atomic_write(&welcome, b"# Welcome to Pithy\n\nStart writing.\n")?;
        files.push("welcome.md".into());
    }

    files.sort();
    Ok(files)
}

#[tauri::command]
pub fn read_file(rel_path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let path = resolve_path(&state.config.vault_dir, &rel_path)?;
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_file(rel_path: String, contents: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let path = resolve_path(&state.config.vault_dir, &rel_path)?;
    atomic_write(&path, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sanitize_basic_cases() {
        assert_eq!(sanitize_filename_impl(" Hello World.md "), "hello-world");
        assert_eq!(sanitize_filename_impl("____"), "untitled");
        assert_eq!(sanitize_filename_impl("My/Bad:Name?.md"), "mybadname");
        assert_eq!(sanitize_filename_impl("  already-clean  "), "already-clean");
        assert_eq!(sanitize_filename_impl("UPPER CASE"), "upper-case");
    }

    #[test]
    fn sanitize_strips_trailing_dashes() {
        assert_eq!(sanitize_filename_impl("hello---"), "hello");
        assert_eq!(sanitize_filename_impl("---hello---"), "hello");
    }

    #[test]
    fn sanitize_empty_becomes_untitled() {
        assert_eq!(sanitize_filename_impl(""), "untitled");
        assert_eq!(sanitize_filename_impl("   "), "untitled");
        assert_eq!(sanitize_filename_impl("///"), "untitled");
    }

    #[test]
    fn resolve_path_rejects_traversal() {
        let vault = PathBuf::from("/tmp/vault");
        assert!(resolve_path(&vault, "../evil.md").is_err());
        assert!(resolve_path(&vault, "foo/../../etc/passwd").is_err());
    }

    #[test]
    fn resolve_path_rejects_absolute() {
        let vault = PathBuf::from("/tmp/vault");
        assert!(resolve_path(&vault, "/etc/passwd").is_err());
    }

    #[test]
    fn resolve_path_accepts_valid_relative() {
        let vault = PathBuf::from("/tmp/vault");
        let result = resolve_path(&vault, "notes/hello.md");
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("notes/hello.md"));
    }

    #[test]
    fn atomic_write_creates_dirs_and_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("note.md");

        atomic_write(&path, b"hello").unwrap();

        let got = std::fs::read_to_string(&path).unwrap();
        assert_eq!(got, "hello");
    }

    #[test]
    fn atomic_write_no_leftover_temp_on_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("note.md");

        atomic_write(&path, b"content").unwrap();

        let temp = dir.path().join(".note.md.pithy-tmp");
        assert!(!temp.exists());
    }
}
