use crate::config::AppState;
use crate::search::{IndexOp, SearchState};
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

fn normalize_for_match(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_dash = false;
    for ch in lower.chars() {
        if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        } else {
            out.push(ch);
            prev_dash = false;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Returns byte offset ranges `(start, end)` of the inner text of each `[[...]]` wikilink.
fn find_wikilinks(content: &str) -> Vec<(usize, usize)> {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;
    while i + 1 < len {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let inner_start = i + 2;
            let mut j = inner_start;
            while j + 1 < len {
                if bytes[j] == b'\n' {
                    break;
                }
                if bytes[j] == b']' && bytes[j + 1] == b']' {
                    if j > inner_start {
                        results.push((inner_start, j));
                    }
                    i = j + 2;
                    break;
                }
                j += 1;
            }
            if j + 1 >= len || bytes[j] == b'\n' {
                i = j + 1;
            }
        } else {
            i += 1;
        }
    }
    results
}

/// Convert a sanitized stem back to display form (dashes → spaces).
fn stem_to_display(stem: &str) -> String {
    stem.replace('-', " ")
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
    search: tauri::State<'_, SearchState>,
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
    let _ = search.op_sender.send(IndexOp::Delete { rel_path: old_rel_path });
    let _ = search.op_sender.send(IndexOp::Upsert { rel_path: new_rel_path });
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
pub fn save_file(rel_path: String, contents: String, state: tauri::State<'_, AppState>, search: tauri::State<'_, SearchState>) -> Result<(), String> {
    let path = resolve_path(&state.config.vault_dir, &rel_path)?;
    atomic_write(&path, contents.as_bytes())?;
    let _ = search.op_sender.send(IndexOp::Upsert { rel_path });
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikilinkReference {
    pub rel_path: String,
    pub count: usize,
}

#[tauri::command]
pub fn find_wikilink_references(
    old_stem: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WikilinkReference>, String> {
    let vault = &state.config.vault_dir;
    let target_norm = normalize_for_match(&old_stem);
    let mut refs = Vec::new();

    for entry in WalkDir::new(vault)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let links = find_wikilinks(&content);
        let count = links
            .iter()
            .filter(|(start, end)| normalize_for_match(&content[*start..*end]) == target_norm)
            .count();
        if count > 0 {
            if let Ok(rel) = entry.path().strip_prefix(vault) {
                refs.push(WikilinkReference {
                    rel_path: rel.to_string_lossy().into_owned(),
                    count,
                });
            }
        }
    }

    Ok(refs)
}

#[tauri::command]
pub fn update_wikilink_references(
    old_stem: String,
    new_stem: String,
    state: tauri::State<'_, AppState>,
    search: tauri::State<'_, SearchState>,
) -> Result<Vec<String>, String> {
    let vault = &state.config.vault_dir;
    let target_norm = normalize_for_match(&old_stem);
    let replacement = stem_to_display(&new_stem);
    let mut modified = Vec::new();

    for entry in WalkDir::new(vault)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let links = find_wikilinks(&content);
        let matching: Vec<(usize, usize)> = links
            .into_iter()
            .filter(|(start, end)| normalize_for_match(&content[*start..*end]) == target_norm)
            .collect();

        if matching.is_empty() {
            continue;
        }

        // Build new content by replacing matching wikilink inner text (reverse order to preserve offsets)
        let mut new_content = content.clone();
        for (start, end) in matching.into_iter().rev() {
            new_content.replace_range(start..end, &replacement);
        }

        atomic_write(entry.path(), new_content.as_bytes())?;

        if let Ok(rel) = entry.path().strip_prefix(vault) {
            let rel_str = rel.to_string_lossy().into_owned();
            let _ = search.op_sender.send(IndexOp::Upsert { rel_path: rel_str.clone() });
            modified.push(rel_str);
        }
    }

    Ok(modified)
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
    fn find_wikilinks_and_replace() {
        let content = "See [[my note]] and [[My Note]] but not [[other]]";
        let links = find_wikilinks(content);
        let target_norm = normalize_for_match("my note");
        let matching: Vec<(usize, usize)> = links
            .into_iter()
            .filter(|(s, e)| normalize_for_match(&content[*s..*e]) == target_norm)
            .collect();
        assert_eq!(matching.len(), 2);

        let replacement = "renamed note";
        let mut new_content = content.to_string();
        for (start, end) in matching.into_iter().rev() {
            new_content.replace_range(start..end, replacement);
        }
        assert_eq!(new_content, "See [[renamed note]] and [[renamed note]] but not [[other]]");
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
    fn normalize_for_match_basic() {
        assert_eq!(normalize_for_match("Hello World"), "hello-world");
        assert_eq!(normalize_for_match("project-kickoff"), "project-kickoff");
        assert_eq!(normalize_for_match("some_note"), "some-note");
        assert_eq!(normalize_for_match("  spaced  out  "), "spaced-out");
        assert_eq!(normalize_for_match("MiXeD---CaSe"), "mixed-case");
        assert_eq!(normalize_for_match("hello_-_world"), "hello-world");
    }

    #[test]
    fn normalize_parity_with_typescript() {
        // Matches the TS normalizeForMatch: lowercase, collapse whitespace/dashes/underscores to single dash
        assert_eq!(normalize_for_match("My Note"), "my-note");
        assert_eq!(normalize_for_match("my-note"), "my-note");
        assert_eq!(normalize_for_match("my_note"), "my-note");
        assert_eq!(normalize_for_match("MY  NOTE"), "my-note");
    }

    #[test]
    fn find_wikilinks_basic() {
        let content = "Hello [[world]] and [[foo bar]]!";
        let links = find_wikilinks(content);
        assert_eq!(links.len(), 2);
        assert_eq!(&content[links[0].0..links[0].1], "world");
        assert_eq!(&content[links[1].0..links[1].1], "foo bar");
    }

    #[test]
    fn find_wikilinks_ignores_newlines() {
        let content = "[[ok]]\n[[broken\nlink]]\n[[also ok]]";
        let links = find_wikilinks(content);
        assert_eq!(links.len(), 2);
        assert_eq!(&content[links[0].0..links[0].1], "ok");
        assert_eq!(&content[links[1].0..links[1].1], "also ok");
    }

    #[test]
    fn find_wikilinks_empty_brackets() {
        let content = "[[]] not a link";
        let links = find_wikilinks(content);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn find_wikilinks_adjacent() {
        let content = "[[a]][[b]]";
        let links = find_wikilinks(content);
        assert_eq!(links.len(), 2);
        assert_eq!(&content[links[0].0..links[0].1], "a");
        assert_eq!(&content[links[1].0..links[1].1], "b");
    }

    #[test]
    fn stem_to_display_basic() {
        assert_eq!(stem_to_display("hello-world"), "hello world");
        assert_eq!(stem_to_display("simple"), "simple");
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
