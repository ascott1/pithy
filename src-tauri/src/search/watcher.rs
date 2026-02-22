use std::path::{Path, PathBuf};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::IndexOp;

/// Returns true if the given path should be ignored by the file watcher.
fn should_ignore(path: &Path, vault_dir: &Path) -> bool {
    let rel = match path.strip_prefix(vault_dir) {
        Ok(r) => r,
        Err(_) => return true,
    };

    // Any path component starting with '.' (dotfiles/dotfolders)
    for component in rel.components() {
        if let std::path::Component::Normal(s) = component {
            if let Some(s) = s.to_str() {
                if s.starts_with('.') {
                    return true;
                }
            }
        }
    }

    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => return true,
    };

    // Sync artifacts
    if file_name.ends_with(".icloud") || file_name.ends_with(".conflict") {
        return true;
    }

    // Atomic write temp files
    if file_name.ends_with(".pithy-tmp") {
        return true;
    }

    // Non-.md extension
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") => false,
        _ => true,
    }
}

/// Starts a file watcher on the vault directory. File system events are
/// translated into `IndexOp`s and sent to `op_sender`. The returned
/// `RecommendedWatcher` must be kept alive by the caller — dropping it stops
/// watching.
pub fn start_watcher(
    vault_dir: PathBuf,
    op_sender: mpsc::UnboundedSender<IndexOp>,
) -> Result<RecommendedWatcher, String> {
    let vault = vault_dir.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            let event = match result {
                Ok(e) => e,
                Err(_) => return,
            };

            for path in &event.paths {
                if should_ignore(path, &vault) {
                    continue;
                }

                let rel_path = match path.strip_prefix(&vault) {
                    Ok(r) => r.to_string_lossy().to_string(),
                    Err(_) => continue,
                };

                let op = match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        IndexOp::Upsert { rel_path }
                    }
                    EventKind::Remove(_) => IndexOp::Delete { rel_path },
                    _ => continue,
                };

                let _ = op_sender.send(op);
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| format!("Failed to create file watcher: {e}"))?;

    watcher
        .watch(&vault_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch vault directory: {e}"))?;

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vault() -> PathBuf {
        PathBuf::from("/vault")
    }

    #[test]
    fn ignores_dotfiles() {
        let v = vault();
        assert!(should_ignore(&v.join(".git/HEAD"), &v));
        assert!(should_ignore(&v.join(".DS_Store"), &v));
    }

    #[test]
    fn ignores_non_md() {
        let v = vault();
        assert!(should_ignore(&v.join("image.png"), &v));
        assert!(should_ignore(&v.join("notes.txt"), &v));
    }

    #[test]
    fn ignores_pithy_tmp() {
        let v = vault();
        assert!(should_ignore(&v.join("note.md.pithy-tmp"), &v));
    }

    #[test]
    fn ignores_sync_artifacts() {
        let v = vault();
        assert!(should_ignore(&v.join("file.icloud"), &v));
        assert!(should_ignore(&v.join("file.conflict"), &v));
    }

    #[test]
    fn accepts_valid_md() {
        let v = vault();
        assert!(!should_ignore(&v.join("note.md"), &v));
        assert!(!should_ignore(&v.join("sub/dir/note.md"), &v));
    }

    #[test]
    fn ignores_pithy_index_dir() {
        let v = vault();
        assert!(should_ignore(&v.join(".pithy/search/meta.json"), &v));
    }
}
