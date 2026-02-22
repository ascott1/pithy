use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tantivy::doc;
use tantivy::{Index, IndexWriter, Term};
use tokio::sync::mpsc;
use tokio::time::timeout;
use walkdir::WalkDir;

use crate::search::schema::SchemaFields;
use crate::search::tags::extract_tags;
use crate::search::{IndexOp, STATUS_BUILDING, STATUS_ERROR, STATUS_READY};

const COALESCE_TIMEOUT: Duration = Duration::from_millis(250);
const WRITER_HEAP_SIZE: usize = 50_000_000;

fn index_file(
    writer: &mut IndexWriter,
    fields: &SchemaFields,
    vault_dir: &Path,
    rel_path: &str,
) {
    let delete_term = Term::from_field_text(fields.path, rel_path);
    writer.delete_term(delete_term);

    let full_path = vault_dir.join(rel_path);
    let body = match std::fs::read_to_string(&full_path) {
        Ok(contents) => contents,
        Err(_) => return, // file gone — delete-only is fine
    };

    let stem = Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let tags = extract_tags(&body);

    let mtime = std::fs::metadata(&full_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut doc = doc!(
        fields.path => rel_path,
        fields.filename_stem => stem,
        fields.body => body,
        fields.modified_date => mtime,
    );

    for tag in &tags {
        doc.add_text(fields.tags, tag);
    }

    writer.add_document(doc).ok();
}

pub fn build_full_index(
    writer: &mut IndexWriter,
    fields: &SchemaFields,
    vault_dir: &Path,
) -> usize {
    writer.delete_all_documents().ok();

    let mut count = 0;
    for entry in WalkDir::new(vault_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(vault_dir) {
            if let Some(rel_str) = rel.to_str() {
                index_file(writer, fields, vault_dir, rel_str);
                count += 1;
            }
        }
    }

    writer.commit().ok();
    count
}

pub async fn run_worker(
    mut receiver: mpsc::UnboundedReceiver<IndexOp>,
    index: Index,
    fields: SchemaFields,
    vault_dir: std::path::PathBuf,
    status: Arc<AtomicU8>,
) {
    let mut writer = match index.writer(WRITER_HEAP_SIZE) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create index writer: {e}");
            status.store(STATUS_ERROR, Ordering::SeqCst);
            return;
        }
    };

    // Initial full build
    status.store(STATUS_BUILDING, Ordering::SeqCst);
    let count = build_full_index(&mut writer, &fields, &vault_dir);
    println!("Search index built: {count} files indexed");
    status.store(STATUS_READY, Ordering::SeqCst);

    // Incremental processing loop
    let mut pending_upserts: HashSet<String> = HashSet::new();
    let mut pending_deletes: HashSet<String> = HashSet::new();
    let mut dirty = false;

    loop {
        let op = if dirty {
            match timeout(COALESCE_TIMEOUT, receiver.recv()).await {
                Ok(Some(op)) => Some(op),
                Ok(None) => break, // channel closed
                Err(_) => None,    // timeout — flush
            }
        } else {
            match receiver.recv().await {
                Some(op) => Some(op),
                None => break, // channel closed
            }
        };

        match op {
            Some(IndexOp::Upsert { rel_path }) => {
                pending_deletes.remove(&rel_path);
                pending_upserts.insert(rel_path);
                dirty = true;
            }
            Some(IndexOp::Delete { rel_path }) => {
                pending_upserts.remove(&rel_path);
                pending_deletes.insert(rel_path);
                dirty = true;
            }
            Some(IndexOp::Rebuild) => {
                pending_upserts.clear();
                pending_deletes.clear();
                dirty = false;
                status.store(STATUS_BUILDING, Ordering::SeqCst);
                let count = build_full_index(&mut writer, &fields, &vault_dir);
                println!("Search index rebuilt: {count} files indexed");
                status.store(STATUS_READY, Ordering::SeqCst);
            }
            None => {
                // Timeout — flush pending changes
                flush_pending(
                    &mut writer,
                    &fields,
                    &vault_dir,
                    &mut pending_upserts,
                    &mut pending_deletes,
                );
                dirty = false;
            }
        }
    }

    // Channel closed — flush remaining
    if dirty {
        flush_pending(
            &mut writer,
            &fields,
            &vault_dir,
            &mut pending_upserts,
            &mut pending_deletes,
        );
    }
}

fn flush_pending(
    writer: &mut IndexWriter,
    fields: &SchemaFields,
    vault_dir: &Path,
    pending_upserts: &mut HashSet<String>,
    pending_deletes: &mut HashSet<String>,
) {
    for rel_path in pending_deletes.drain() {
        let term = Term::from_field_text(fields.path, &rel_path);
        writer.delete_term(term);
    }
    for rel_path in pending_upserts.drain() {
        index_file(writer, fields, vault_dir, &rel_path);
    }
    writer.commit().ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::schema::build_schema;
    use tempfile::tempdir;

    #[test]
    fn build_full_index_indexes_md_files() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(vault.join("note-one.md"), "Hello #world").unwrap();
        std::fs::write(vault.join("note-two.md"), "Second note #rust").unwrap();
        std::fs::write(vault.join("ignored.txt"), "Not markdown").unwrap();

        let index_dir = dir.path().join("index");
        let (schema, fields) = build_schema();
        let index = crate::search::schema::open_or_create_index(&index_dir, &schema).unwrap();
        let mut writer = index.writer(WRITER_HEAP_SIZE).unwrap();

        let count = build_full_index(&mut writer, &fields, &vault);
        assert_eq!(count, 2);

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        assert_eq!(searcher.num_docs(), 2);
    }

    #[test]
    fn build_full_index_ignores_dotfiles() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault");
        let hidden = vault.join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(vault.join("visible.md"), "hi").unwrap();
        std::fs::write(hidden.join("secret.md"), "hidden").unwrap();

        let index_dir = dir.path().join("index");
        let (schema, fields) = build_schema();
        let index = crate::search::schema::open_or_create_index(&index_dir, &schema).unwrap();
        let mut writer = index.writer(WRITER_HEAP_SIZE).unwrap();

        let count = build_full_index(&mut writer, &fields, &vault);
        assert_eq!(count, 1);
    }

    #[test]
    fn index_file_handles_missing_file() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();

        let index_dir = dir.path().join("index");
        let (schema, fields) = build_schema();
        let index = crate::search::schema::open_or_create_index(&index_dir, &schema).unwrap();
        let mut writer = index.writer(WRITER_HEAP_SIZE).unwrap();

        // Should not panic when file doesn't exist
        index_file(&mut writer, &fields, &vault, "nonexistent.md");
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        assert_eq!(searcher.num_docs(), 0);
    }
}
