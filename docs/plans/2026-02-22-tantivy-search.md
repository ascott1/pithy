# Tantivy Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add full-text search to Pithy using Tantivy — initial index build on startup, incremental updates on file changes, and a search query command exposed to the frontend.

**Architecture:** A new `search.rs` module owns a Tantivy index stored at `vault/.pithy/search/`. On startup, a background task builds the index by walking the vault. Incremental updates are triggered by hooking `save_file` and `rename_file` in `fs.rs`, plus a `notify` file watcher for external changes. A single background worker thread serializes all index writes, coalescing rapid updates. One primary Tauri command (`search_query`) serves results to the frontend.

**Tech Stack:** Tantivy (Rust full-text search), notify (file watcher), tokio mpsc (worker channel), serde (IPC serialization)

---

## Task 1: Add Tantivy + notify dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add dependencies to Cargo.toml**

Add to `[dependencies]`:
```toml
tantivy = "0.22"
notify = "7"
notify-debouncer-mini = "0.5"
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles with no errors (new deps download)

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add tantivy and notify dependencies"
```

---

## Task 2: Tag extraction — tests first

**Files:**
- Create: `src-tauri/src/search/tags.rs`
- Create: `src-tauri/src/search/mod.rs` (initially just `pub mod tags;`)

Tag extraction parses `#tag` from markdown body, excluding tags inside fenced code blocks, inline code, and URLs. This is a pure function with no Tantivy dependency — test it in isolation.

**Step 1: Create the module structure**

Create `src-tauri/src/search/mod.rs`:
```rust
pub mod tags;
```

Add `mod search;` to `src-tauri/src/lib.rs` (after `mod fs;`). Don't wire anything into the Tauri builder yet.

**Step 2: Write failing tests for tag extraction**

Create `src-tauri/src/search/tags.rs`:
```rust
/// Extract `#tags` from markdown text, excluding tags inside:
/// - Fenced code blocks (``` or ~~~)
/// - Inline code (`...`)
/// - URLs (http:// or https://)
/// - Markdown link destinations [text](url)
/// Returns deduplicated, lowercased tag names without the `#` prefix.
pub fn extract_tags(text: &str) -> Vec<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tags() {
        let result = extract_tags("Hello #world and #rust are great");
        assert_eq!(result, vec!["rust", "world"]);
    }

    #[test]
    fn tag_with_hyphens_and_underscores() {
        let result = extract_tags("#my-tag and #my_tag");
        assert_eq!(result, vec!["my-tag", "my_tag"]);
    }

    #[test]
    fn tag_with_slashes() {
        let result = extract_tags("Nested #project/pithy tag");
        assert_eq!(result, vec!["project/pithy"]);
    }

    #[test]
    fn ignores_headings() {
        let result = extract_tags("# Heading\n## Subheading\n#tag");
        assert_eq!(result, vec!["tag"]);
    }

    #[test]
    fn ignores_fenced_code_block() {
        let text = "Before #visible\n```\n#hidden\n```\nAfter #also-visible";
        let result = extract_tags(text);
        assert_eq!(result, vec!["also-visible", "visible"]);
    }

    #[test]
    fn ignores_tilde_fenced_code_block() {
        let text = "~~~\n#hidden\n~~~\n#visible";
        let result = extract_tags(text);
        assert_eq!(result, vec!["visible"]);
    }

    #[test]
    fn ignores_inline_code() {
        let result = extract_tags("Use `#hidden` but #visible");
        assert_eq!(result, vec!["visible"]);
    }

    #[test]
    fn ignores_url_fragments() {
        let result = extract_tags("Visit https://example.com/#anchor and #real");
        assert_eq!(result, vec!["real"]);
    }

    #[test]
    fn ignores_markdown_link_url() {
        let result = extract_tags("[link](https://example.com/#frag) #real");
        assert_eq!(result, vec!["real"]);
    }

    #[test]
    fn deduplicates() {
        let result = extract_tags("#rust and #rust again");
        assert_eq!(result, vec!["rust"]);
    }

    #[test]
    fn lowercases() {
        let result = extract_tags("#Rust #PYTHON");
        assert_eq!(result, vec!["python", "rust"]);
    }

    #[test]
    fn empty_input() {
        let result = extract_tags("");
        assert!(result.is_empty());
    }

    #[test]
    fn hash_only_not_a_tag() {
        let result = extract_tags("# ");
        assert!(result.is_empty());
    }

    #[test]
    fn tag_at_start_of_line() {
        let result = extract_tags("#first\nsome text");
        assert_eq!(result, vec!["first"]);
    }

    #[test]
    fn tag_after_punctuation() {
        let result = extract_tags("(#parens) [#brackets] {#braces}");
        assert_eq!(result, vec!["braces", "brackets", "parens"]);
    }
}
```

**Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test search::tags`
Expected: FAIL — `todo!()` panics

**Step 4: Implement `extract_tags`**

Replace `todo!()` with:
```rust
pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut in_fenced_block = false;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;

    // Process line-by-line for fenced code blocks, character-by-character within lines
    for line in text.lines() {
        let trimmed = line.trim_start();

        // Toggle fenced code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fenced_block = !in_fenced_block;
            continue;
        }

        if in_fenced_block {
            continue;
        }

        let line_chars: Vec<char> = line.chars().collect();
        let line_len = line_chars.len();
        let mut j = 0;

        while j < line_len {
            // Skip inline code
            if line_chars[j] == '`' {
                j += 1;
                while j < line_len && line_chars[j] != '`' {
                    j += 1;
                }
                if j < line_len {
                    j += 1; // skip closing `
                }
                continue;
            }

            // Skip URLs
            if j + 8 < line_len {
                let rest: String = line_chars[j..].iter().collect();
                if rest.starts_with("https://") || rest.starts_with("http://") {
                    while j < line_len && !line_chars[j].is_whitespace() && line_chars[j] != ')' {
                        j += 1;
                    }
                    continue;
                }
            }

            // Skip markdown link destinations: ](url)
            if line_chars[j] == ']' && j + 1 < line_len && line_chars[j + 1] == '(' {
                j += 2; // skip ](
                while j < line_len && line_chars[j] != ')' {
                    j += 1;
                }
                if j < line_len {
                    j += 1; // skip )
                }
                continue;
            }

            // Detect tags
            if line_chars[j] == '#' {
                // Check boundary: must be at start or preceded by whitespace/punctuation
                let is_boundary = j == 0
                    || line_chars[j - 1].is_whitespace()
                    || matches!(line_chars[j - 1], '(' | '[' | '{');

                if is_boundary {
                    // Check next char is alphanumeric or underscore (not space = heading, not # = heading)
                    if j + 1 < line_len
                        && (line_chars[j + 1].is_alphanumeric() || line_chars[j + 1] == '_')
                    {
                        let start = j + 1;
                        j = start;
                        while j < line_len
                            && (line_chars[j].is_alphanumeric()
                                || line_chars[j] == '-'
                                || line_chars[j] == '_'
                                || line_chars[j] == '/')
                        {
                            j += 1;
                        }
                        let tag: String =
                            line_chars[start..j].iter().collect::<String>().to_lowercase();
                        if !tag.is_empty() && seen.insert(tag.clone()) {
                            tags.push(tag);
                        }
                        continue;
                    }
                }
            }

            j += 1;
        }
    }

    tags.sort();
    tags
}
```

**Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test search::tags`
Expected: All 16 tests PASS

**Step 6: Commit**

```bash
git add src-tauri/src/search/
git commit -m "feat(search): add tag extraction with code/URL exclusion"
```

---

## Task 3: Tantivy schema + index lifecycle

**Files:**
- Modify: `src-tauri/src/search/mod.rs`
- Modify: `src-tauri/src/lib.rs`

This task creates the Tantivy schema, `SearchState`, and index open/create logic. No querying or writing yet — just the scaffolding.

**Step 1: Write the schema and state types**

In `src-tauri/src/search/mod.rs`, replace contents with:
```rust
pub mod tags;

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, ReloadPolicy};
use tokio::sync::mpsc;

/// Operations sent to the index writer worker.
pub enum IndexOp {
    Upsert { rel_path: String },
    Delete { rel_path: String },
    Rebuild,
}

/// Search index status.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SearchStatus {
    Building = 0,
    Ready = 1,
    Error = 2,
}

impl From<u8> for SearchStatus {
    fn from(v: u8) -> Self {
        match v {
            0 => SearchStatus::Building,
            1 => SearchStatus::Ready,
            _ => SearchStatus::Error,
        }
    }
}

/// Holds field handles for quick access.
#[derive(Clone)]
pub struct SchemaFields {
    pub path: Field,
    pub filename_stem: Field,
    pub body: Field,
    pub tags: Field,
    pub modified_date: Field,
}

pub struct SearchState {
    pub vault_dir: PathBuf,
    pub index: Index,
    pub reader: IndexReader,
    pub fields: SchemaFields,
    pub schema: Schema,
    pub status: AtomicU8,
    pub op_sender: mpsc::UnboundedSender<IndexOp>,
}

/// Build the Tantivy schema.
/// Fields:
///   path          — STRING | STORED (unique key for upsert/delete)
///   filename_stem — TEXT | STORED (tokenized title search)
///   body          — TEXT | STORED (full-text + snippets)
///   tags          — STRING | STORED (one entry per tag, filterable)
///   modified_date — I64 | FAST | STORED (unix epoch secs, for sorting)
pub fn build_schema() -> (Schema, SchemaFields) {
    let mut builder = Schema::builder();

    let path = builder.add_text_field("path", STRING | STORED);
    let filename_stem = builder.add_text_field("filename_stem", TEXT | STORED);
    let body = builder.add_text_field("body", TEXT | STORED);
    let tags = builder.add_text_field("tags", STRING | STORED);
    let modified_date = builder.add_i64_field(
        "modified_date",
        NumericOptions::default().set_fast().set_stored().set_indexed(),
    );

    let schema = builder.build();
    let fields = SchemaFields {
        path,
        filename_stem,
        body,
        tags,
        modified_date,
    };
    (schema, fields)
}

/// Open or create the Tantivy index at `index_dir`.
pub fn open_or_create_index(index_dir: &Path, schema: &Schema) -> Result<Index, String> {
    std::fs::create_dir_all(index_dir).map_err(|e| format!("Failed to create index dir: {e}"))?;

    // Try opening existing index first
    match Index::open_in_dir(index_dir) {
        Ok(index) => Ok(index),
        Err(_) => {
            // Create new index
            Index::create_in_dir(index_dir, schema.clone())
                .map_err(|e| format!("Failed to create index: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn schema_has_expected_fields() {
        let (schema, fields) = build_schema();
        assert_eq!(schema.get_field_name(fields.path), "path");
        assert_eq!(schema.get_field_name(fields.filename_stem), "filename_stem");
        assert_eq!(schema.get_field_name(fields.body), "body");
        assert_eq!(schema.get_field_name(fields.tags), "tags");
        assert_eq!(schema.get_field_name(fields.modified_date), "modified_date");
    }

    #[test]
    fn open_or_create_creates_new_index() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path().join("search");
        let (schema, _) = build_schema();
        let result = open_or_create_index(&index_dir, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn open_or_create_reopens_existing() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path().join("search");
        let (schema, _) = build_schema();

        // Create
        open_or_create_index(&index_dir, &schema).unwrap();
        // Reopen
        let result = open_or_create_index(&index_dir, &schema);
        assert!(result.is_ok());
    }
}
```

**Step 2: Run tests to verify**

Run: `cd src-tauri && cargo test search`
Expected: Schema tests + tag tests all PASS

**Step 3: Commit**

```bash
git add src-tauri/src/search/mod.rs
git commit -m "feat(search): add Tantivy schema and index lifecycle"
```

---

## Task 4: Index writer worker

**Files:**
- Create: `src-tauri/src/search/worker.rs`
- Modify: `src-tauri/src/search/mod.rs`

The worker runs on a dedicated thread, receives `IndexOp` messages via an mpsc channel, and serializes all Tantivy writes. It coalesces rapid updates by collecting ops for a short window before committing.

**Step 1: Create the worker module**

Add `pub mod worker;` to `src-tauri/src/search/mod.rs`.

Create `src-tauri/src/search/worker.rs`:
```rust
use crate::search::tags::extract_tags;
use crate::search::{IndexOp, SchemaFields, SearchStatus};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tantivy::doc;
use tantivy::schema::Schema;
use tantivy::{Index, IndexWriter, Term};
use tokio::sync::mpsc;

/// Index a single file. Deletes any existing doc with the same path, then adds.
fn index_file(
    writer: &IndexWriter,
    fields: &SchemaFields,
    vault_dir: &Path,
    rel_path: &str,
) -> Result<(), String> {
    let full_path = vault_dir.join(rel_path);

    // Delete existing entry
    let path_term = Term::from_field_text(fields.path, rel_path);
    writer.delete_term(path_term);

    // Read file — if missing, just delete (file was removed between events)
    let contents = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    // Extract metadata
    let stem = Path::new(rel_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let tags = extract_tags(&contents);

    let modified = fs::metadata(&full_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Build document
    let mut doc = tantivy::TantivyDocument::default();
    doc.add_text(fields.path, rel_path);
    doc.add_text(fields.filename_stem, &stem);
    doc.add_text(fields.body, &contents);
    for tag in &tags {
        doc.add_text(fields.tags, tag);
    }
    doc.add_i64(fields.modified_date, modified);

    writer.add_document(doc).map_err(|e| e.to_string())?;
    Ok(())
}

/// Walk the vault and index all .md files. Used for initial build and rebuild.
pub fn build_full_index(
    writer: &IndexWriter,
    fields: &SchemaFields,
    vault_dir: &Path,
) -> Result<usize, String> {
    // Clear existing index
    writer.delete_all_documents().map_err(|e| e.to_string())?;

    let mut count = 0;

    for entry in walkdir::WalkDir::new(vault_dir)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().map_or(true, |ext| ext != "md") {
            continue;
        }

        if let Ok(rel) = path.strip_prefix(vault_dir) {
            let rel_str = rel.to_string_lossy().into_owned();
            if let Err(e) = index_file(writer, fields, vault_dir, &rel_str) {
                eprintln!("search: failed to index {}: {}", rel_str, e);
            } else {
                count += 1;
            }
        }
    }

    writer.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Spawn the index writer worker. Returns when the channel closes.
pub async fn run_worker(
    mut receiver: mpsc::UnboundedReceiver<IndexOp>,
    index: Index,
    fields: SchemaFields,
    vault_dir: PathBuf,
    status: Arc<AtomicU8>,
) {
    // 50MB heap for writer — reasonable for a notes app
    let mut writer: IndexWriter = match index.writer(50_000_000) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("search: failed to create writer: {}", e);
            status.store(SearchStatus::Error as u8, Ordering::SeqCst);
            return;
        }
    };

    // Initial full build
    status.store(SearchStatus::Building as u8, Ordering::SeqCst);
    match build_full_index(&writer, &fields, &vault_dir) {
        Ok(count) => {
            println!("search: indexed {} files", count);
            status.store(SearchStatus::Ready as u8, Ordering::SeqCst);
        }
        Err(e) => {
            eprintln!("search: initial build failed: {}", e);
            status.store(SearchStatus::Error as u8, Ordering::SeqCst);
        }
    }

    // Process incremental ops
    let mut pending_upserts: HashSet<String> = HashSet::new();
    let mut pending_deletes: HashSet<String> = HashSet::new();
    let mut dirty = false;

    loop {
        // Try to receive, with a timeout to batch/coalesce
        let op = if dirty {
            // If we have pending changes, use a short timeout to coalesce
            match tokio::time::timeout(
                std::time::Duration::from_millis(250),
                receiver.recv(),
            )
            .await
            {
                Ok(Some(op)) => Some(op),
                Ok(None) => break, // channel closed
                Err(_) => None,    // timeout — flush pending
            }
        } else {
            // No pending changes — block until next op
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
                status.store(SearchStatus::Building as u8, Ordering::SeqCst);
                match build_full_index(&writer, &fields, &vault_dir) {
                    Ok(count) => {
                        println!("search: rebuilt index with {} files", count);
                        status.store(SearchStatus::Ready as u8, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!("search: rebuild failed: {}", e);
                        status.store(SearchStatus::Error as u8, Ordering::SeqCst);
                    }
                }
                dirty = false;
            }
            None => {
                // Timeout — flush pending changes
                if dirty {
                    for rel_path in pending_deletes.drain() {
                        let term = Term::from_field_text(fields.path, &rel_path);
                        writer.delete_term(term);
                    }
                    for rel_path in pending_upserts.drain() {
                        if let Err(e) = index_file(&writer, &fields, &vault_dir, &rel_path) {
                            eprintln!("search: incremental index failed for {}: {}", rel_path, e);
                        }
                    }
                    if let Err(e) = writer.commit() {
                        eprintln!("search: commit failed: {}", e);
                    }
                    dirty = false;
                }
            }
        }
    }

    // Drain remaining
    if dirty {
        for rel_path in pending_deletes {
            let term = Term::from_field_text(fields.path, &rel_path);
            writer.delete_term(term);
        }
        for rel_path in pending_upserts {
            let _ = index_file(&writer, &fields, &vault_dir, &rel_path);
        }
        let _ = writer.commit();
    }
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles (note: `tokio` may need adding — see step 2a)

**Step 2a: Add tokio if needed**

Tauri 2 uses tokio internally. Check if `tokio` is already available. If `cargo check` fails on `tokio::sync::mpsc` or `tokio::time`, add to `Cargo.toml`:
```toml
tokio = { version = "1", features = ["sync", "time", "rt"] }
```

**Step 3: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All existing + search tests PASS

**Step 4: Commit**

```bash
git add src-tauri/src/search/worker.rs src-tauri/src/search/mod.rs
git commit -m "feat(search): add index writer worker with coalesced commits"
```

---

## Task 5: File watcher for external changes

**Files:**
- Create: `src-tauri/src/search/watcher.rs`
- Modify: `src-tauri/src/search/mod.rs`

The file watcher uses `notify` to detect external file changes (iCloud sync, git pull, external editors) and feeds `IndexOp`s to the worker.

**Step 1: Create the watcher module**

Add `pub mod watcher;` to `src-tauri/src/search/mod.rs`.

Create `src-tauri/src/search/watcher.rs`:
```rust
use crate::search::IndexOp;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Returns true if this path should be ignored by the watcher.
fn should_ignore(path: &Path, vault_dir: &Path) -> bool {
    // Must be inside vault
    let rel = match path.strip_prefix(vault_dir) {
        Ok(r) => r,
        Err(_) => return true,
    };

    // Ignore dotfiles/dotfolders
    for component in rel.components() {
        let s = component.as_os_str().to_string_lossy();
        if s.starts_with('.') {
            return true;
        }
    }

    // Ignore non-.md files
    if path.extension().map_or(true, |ext| ext != "md") {
        return true;
    }

    // Ignore known sync artifacts
    let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    if name.ends_with(".icloud")
        || name.ends_with(".conflict")
        || name == ".DS_Store"
    {
        return true;
    }

    // Ignore temp files from atomic writes
    if name.ends_with(".pithy-tmp") {
        return true;
    }

    false
}

/// Start watching the vault directory for file changes.
/// Sends IndexOps to the provided sender.
/// Returns the watcher handle (must be kept alive).
pub fn start_watcher(
    vault_dir: PathBuf,
    op_sender: mpsc::UnboundedSender<IndexOp>,
) -> Result<RecommendedWatcher, String> {
    let vault_dir_clone = vault_dir.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            let event = match result {
                Ok(e) => e,
                Err(_) => return,
            };

            for path in &event.paths {
                if should_ignore(path, &vault_dir_clone) {
                    continue;
                }

                let rel = match path.strip_prefix(&vault_dir_clone) {
                    Ok(r) => r.to_string_lossy().into_owned(),
                    Err(_) => continue,
                };

                let op = match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        IndexOp::Upsert { rel_path: rel }
                    }
                    EventKind::Remove(_) => IndexOp::Delete { rel_path: rel },
                    _ => continue,
                };

                // Ignore send errors — worker may have shut down
                let _ = op_sender.send(op);
            }
        },
        Config::default(),
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
        PathBuf::from("/tmp/vault")
    }

    #[test]
    fn ignores_dotfiles() {
        assert!(should_ignore(Path::new("/tmp/vault/.git/HEAD"), &vault()));
        assert!(should_ignore(
            Path::new("/tmp/vault/.DS_Store"),
            &vault()
        ));
    }

    #[test]
    fn ignores_non_md() {
        assert!(should_ignore(Path::new("/tmp/vault/file.txt"), &vault()));
        assert!(should_ignore(Path::new("/tmp/vault/image.png"), &vault()));
    }

    #[test]
    fn ignores_pithy_tmp() {
        assert!(should_ignore(
            Path::new("/tmp/vault/.note.md.pithy-tmp"),
            &vault()
        ));
    }

    #[test]
    fn ignores_sync_artifacts() {
        assert!(should_ignore(
            Path::new("/tmp/vault/note.md.icloud"),
            &vault()
        ));
        assert!(should_ignore(
            Path::new("/tmp/vault/note.md.conflict"),
            &vault()
        ));
    }

    #[test]
    fn accepts_valid_md() {
        assert!(!should_ignore(Path::new("/tmp/vault/note.md"), &vault()));
        assert!(!should_ignore(
            Path::new("/tmp/vault/sub/dir/note.md"),
            &vault()
        ));
    }

    #[test]
    fn ignores_pithy_index_dir() {
        assert!(should_ignore(
            Path::new("/tmp/vault/.pithy/search/meta.json"),
            &vault()
        ));
    }
}
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test search::watcher`
Expected: All watcher filter tests PASS

**Step 3: Commit**

```bash
git add src-tauri/src/search/watcher.rs src-tauri/src/search/mod.rs
git commit -m "feat(search): add notify file watcher with ignore rules"
```

---

## Task 6: Search query execution

**Files:**
- Create: `src-tauri/src/search/query.rs`
- Modify: `src-tauri/src/search/mod.rs`

This task implements the Tantivy query logic — parsing user input, building queries with field boosts, and returning results with snippets.

**Step 1: Create the query module**

Add `pub mod query;` to `src-tauri/src/search/mod.rs`.

Create `src-tauri/src/search/query.rs`:
```rust
use crate::search::{SchemaFields, SearchStatus};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::snippet::SnippetGenerator;
use tantivy::{Index, IndexReader, Term};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub filename_stem: String,
    pub snippet: Option<String>,
    pub tags: Vec<String>,
    pub modified_date: i64,
    pub score: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub query: String,
}

/// Execute a search query against the index.
pub fn execute_search(
    reader: &IndexReader,
    fields: &SchemaFields,
    index: &Index,
    query_str: &str,
    limit: usize,
    offset: usize,
) -> Result<SearchResponse, String> {
    let searcher = reader.searcher();

    // Separate tag filters from free text
    let mut free_terms = Vec::new();
    let mut tag_filters = Vec::new();

    for token in query_str.split_whitespace() {
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.is_empty() {
                tag_filters.push(tag.to_lowercase());
            }
        } else {
            free_terms.push(token.to_string());
        }
    }

    let free_text = free_terms.join(" ");

    // Build the query
    let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

    if !free_text.is_empty() {
        // Full-text query with boosts: filename_stem^3, body^1
        let query_parser =
            QueryParser::for_index(index, vec![fields.filename_stem, fields.body]);
        let text_query = query_parser
            .parse_query(&free_text)
            .map_err(|e| format!("Invalid query: {e}"))?;

        subqueries.push((Occur::Must, text_query));
    }

    // Add tag filters
    for tag in &tag_filters {
        let term_query = TermQuery::new(
            Term::from_field_text(fields.tags, tag),
            IndexRecordOption::Basic,
        );
        subqueries.push((Occur::Must, Box::new(term_query)));
    }

    if subqueries.is_empty() {
        return Ok(SearchResponse {
            hits: vec![],
            query: query_str.to_string(),
        });
    }

    let query = BooleanQuery::new(subqueries);

    // Collect results
    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(offset + limit))
        .map_err(|e| format!("Search failed: {e}"))?;

    // Build snippet generator for body field
    let snippet_generator = SnippetGenerator::create(&searcher, &query, fields.body)
        .map_err(|e| format!("Snippet generation failed: {e}"))?;

    let mut hits = Vec::new();
    for (score, doc_address) in top_docs.into_iter().skip(offset) {
        let doc: tantivy::TantivyDocument = searcher
            .doc(doc_address)
            .map_err(|e| format!("Failed to retrieve doc: {e}"))?;

        let path = doc
            .get_first(fields.path)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let filename_stem = doc
            .get_first(fields.filename_stem)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tags: Vec<String> = doc
            .get_all(fields.tags)
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let modified_date = doc
            .get_first(fields.modified_date)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let snippet = snippet_generator.snippet_from_doc(&doc);
        let snippet_html = snippet.to_html();
        let snippet_opt = if snippet_html.is_empty() {
            None
        } else {
            Some(snippet_html)
        };

        hits.push(SearchHit {
            path,
            filename_stem,
            snippet: snippet_opt,
            tags,
            modified_date,
            score,
        });
    }

    Ok(SearchResponse {
        hits,
        query: query_str.to_string(),
    })
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src-tauri/src/search/query.rs src-tauri/src/search/mod.rs
git commit -m "feat(search): add query execution with snippets and tag filtering"
```

---

## Task 7: Wire search into Tauri — commands + setup

**Files:**
- Modify: `src-tauri/src/search/mod.rs`
- Modify: `src-tauri/src/lib.rs`

This task wires everything together: initializes `SearchState` in `setup`, starts the worker and watcher, and exposes Tauri commands.

**Step 1: Add Tauri commands to search/mod.rs**

Add these Tauri commands to `src-tauri/src/search/mod.rs`:

```rust
use crate::config::AppState;

/// Initialize the search system. Called from lib.rs setup.
pub fn init_search(vault_dir: PathBuf) -> Result<SearchState, String> {
    let (schema, fields) = build_schema();
    let index_dir = vault_dir.join(".pithy").join("search");
    let index = open_or_create_index(&index_dir, &schema)?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .map_err(|e| format!("Failed to create reader: {e}"))?;

    let (tx, rx) = mpsc::unbounded_channel();
    let status = Arc::new(AtomicU8::new(SearchStatus::Building as u8));

    // Spawn the worker
    let worker_index = index.clone();
    let worker_fields = fields.clone();
    let worker_vault = vault_dir.clone();
    let worker_status = Arc::clone(&status);
    tauri::async_runtime::spawn(async move {
        worker::run_worker(rx, worker_index, worker_fields, worker_vault, worker_status).await;
    });

    Ok(SearchState {
        vault_dir,
        index,
        reader,
        fields,
        schema,
        status: AtomicU8::new(SearchStatus::Building as u8),
        op_sender: tx,
    })
}

#[tauri::command]
pub fn search_query(
    query_text: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: tauri::State<'_, SearchState>,
) -> Result<query::SearchResponse, String> {
    let limit = limit.unwrap_or(20) as usize;
    let offset = offset.unwrap_or(0) as usize;
    query::execute_search(&state.reader, &state.fields, &state.index, &query_text, limit, offset)
}

#[tauri::command]
pub fn search_status(state: tauri::State<'_, SearchState>) -> String {
    let s: SearchStatus = state.status.load(Ordering::SeqCst).into();
    match s {
        SearchStatus::Building => "building".into(),
        SearchStatus::Ready => "ready".into(),
        SearchStatus::Error => "error".into(),
    }
}

#[tauri::command]
pub fn search_rebuild(state: tauri::State<'_, SearchState>) -> Result<(), String> {
    state
        .op_sender
        .send(IndexOp::Rebuild)
        .map_err(|_| "Search worker not running".to_string())
}
```

**Step 2: Wire into lib.rs setup**

Modify `src-tauri/src/lib.rs` to initialize search in `setup`:

After `app.manage(AppState { ... });`, add:
```rust
// Initialize search
let vault_dir = cfg.vault_dir.clone();
let search_state = search::init_search(vault_dir.clone())
    .map_err(|e| format!("Search init failed: {e}"))?;

// Start file watcher
let _watcher = search::watcher::start_watcher(
    vault_dir,
    search_state.op_sender.clone(),
)
.map_err(|e| format!("Watcher init failed: {e}"))?;

// Store watcher handle in search state (or manage separately) to keep it alive
app.manage(search_state);
```

Note: The watcher handle must be kept alive. Either store it in `SearchState` (add a field for it) or store it in a separate managed state. If watcher creation fails, log a warning but don't fail startup.

Add search commands to the `invoke_handler`:
```rust
search::search_query,
search::search_status,
search::search_rebuild,
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

**Step 4: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src-tauri/src/search/mod.rs src-tauri/src/lib.rs
git commit -m "feat(search): wire search state, commands, and watcher into Tauri setup"
```

---

## Task 8: Hook `save_file` and `rename_file` for incremental indexing

**Files:**
- Modify: `src-tauri/src/fs.rs`

This ensures in-app edits immediately trigger index updates — the most important path for keeping search current without relying solely on the file watcher.

**Step 1: Modify `save_file` to send an IndexOp**

Update `save_file` to accept `SearchState` and send an upsert after a successful write:

```rust
#[tauri::command]
pub fn save_file(
    rel_path: String,
    contents: String,
    state: tauri::State<'_, AppState>,
    search: tauri::State<'_, crate::search::SearchState>,
) -> Result<(), String> {
    let path = resolve_path(&state.config.vault_dir, &rel_path)?;
    atomic_write(&path, contents.as_bytes())?;
    let _ = search.op_sender.send(crate::search::IndexOp::Upsert {
        rel_path: rel_path.clone(),
    });
    Ok(())
}
```

**Step 2: Modify `rename_file` to send IndexOps**

```rust
#[tauri::command]
pub fn rename_file(
    old_rel_path: String,
    new_rel_path: String,
    state: tauri::State<'_, AppState>,
    search: tauri::State<'_, crate::search::SearchState>,
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

    let _ = search.op_sender.send(crate::search::IndexOp::Delete {
        rel_path: old_rel_path,
    });
    let _ = search.op_sender.send(crate::search::IndexOp::Upsert {
        rel_path: new_rel_path,
    });

    Ok(())
}
```

**Step 3: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS (existing fs tests don't inject SearchState so they'll need adjusting — see note below)

> **Note:** The existing Rust unit tests for `save_file` and `rename_file` don't use the Tauri command signatures directly — they test `atomic_write` and `resolve_path` as private functions. No test changes needed unless there are integration tests calling the commands.

**Step 4: Commit**

```bash
git add src-tauri/src/fs.rs
git commit -m "feat(search): hook save_file and rename_file for incremental indexing"
```

---

## Task 9: Frontend search IPC wrapper

**Files:**
- Create: `src/lib/tauri/search.ts`

**Step 1: Create the typed search wrapper**

```typescript
import { invoke } from "@tauri-apps/api/core";

export interface SearchHit {
	path: string;
	filenameStem: string;
	snippet: string | null;
	tags: string[];
	modifiedDate: number;
	score: number;
}

export interface SearchResponse {
	hits: SearchHit[];
	query: string;
}

export function searchQuery(
	queryText: string,
	limit?: number,
	offset?: number,
): Promise<SearchResponse> {
	return invoke<SearchResponse>("search_query", { queryText, limit, offset });
}

export function searchStatus(): Promise<string> {
	return invoke<string>("search_status");
}

export function searchRebuild(): Promise<void> {
	return invoke<void>("search_rebuild");
}
```

**Step 2: Commit**

```bash
git add src/lib/tauri/search.ts
git commit -m "feat(search): add frontend IPC wrappers for search commands"
```

---

## Task 10: Integration smoke test

**Files:**
- No new files — manual verification

**Step 1: Build and run the app**

Run: `pnpm tauri dev`
Expected: App launches without errors. Check terminal for:
- `search: indexed N files` log line (N = number of .md files in vault)

**Step 2: Test search command via dev tools**

Open the browser dev console in the Tauri window and run:
```javascript
await window.__TAURI__.core.invoke("search_status")
// Expected: "ready"

await window.__TAURI__.core.invoke("search_query", { queryText: "welcome" })
// Expected: SearchResponse with hits containing welcome.md
```

**Step 3: Test incremental indexing**

1. Edit a note in the app, wait for autosave
2. Search for the new content — should appear in results
3. Create a file externally in the vault directory
4. Search for its content — should appear after a brief delay

**Step 4: Run all automated tests**

Run: `pnpm test`
Expected: All TS + Rust tests PASS

**Step 5: Commit any fixes**

```bash
git add -p  # only stage related changes
git commit -m "fix(search): address integration issues from smoke testing"
```

---

## Dependency Graph

```
Task 1 (deps)
  └─> Task 2 (tags) ─────────────┐
  └─> Task 3 (schema) ───────────┤
                                  ├─> Task 4 (worker) ──┐
                                  │                      ├─> Task 7 (wiring)
  └─> Task 5 (watcher) ──────────┘                      │      │
                                                         │      ├─> Task 8 (fs hooks)
  Task 6 (query) ────────────────────────────────────────┘      │
                                                                ├─> Task 9 (frontend)
                                                                └─> Task 10 (smoke test)
```

Tasks 2, 3, 5 can be done in parallel after Task 1.
Task 6 can be done in parallel with Tasks 4 and 5.
Task 7 depends on Tasks 3, 4, 5, 6.
Tasks 8, 9 depend on Task 7.
Task 10 is last.
