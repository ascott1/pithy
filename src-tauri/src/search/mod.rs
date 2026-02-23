pub mod query;
pub mod schema;
pub mod tags;
pub mod watcher;
pub mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use notify::RecommendedWatcher;
use tantivy::{Index, IndexReader, ReloadPolicy};
use tokio::sync::mpsc;

use schema::SchemaFields;

pub const STATUS_BUILDING: u8 = 0;
pub const STATUS_READY: u8 = 1;
pub const STATUS_ERROR: u8 = 2;

pub enum IndexOp {
    Upsert { rel_path: String },
    Delete { rel_path: String },
    Rebuild,
}

pub struct SearchState {
    pub index: Index,
    pub reader: IndexReader,
    pub fields: SchemaFields,
    pub status: Arc<AtomicU8>,
    pub op_sender: mpsc::UnboundedSender<IndexOp>,
    _watcher: Option<RecommendedWatcher>,
}

pub fn init_search(vault_dir: PathBuf) -> Result<SearchState, String> {
    let (tantivy_schema, fields) = schema::build_schema();
    let index_dir = vault_dir.join(".pithy").join("search");
    let index = schema::open_or_create_index(&index_dir, &tantivy_schema)?;

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .map_err(|e| format!("Failed to create index reader: {e}"))?;

    let (tx, rx) = mpsc::unbounded_channel();
    let status = Arc::new(AtomicU8::new(STATUS_BUILDING));

    // Spawn the index writer worker
    let worker_index = index.clone();
    let worker_fields = fields.clone();
    let worker_vault = vault_dir.clone();
    let worker_status = Arc::clone(&status);
    tauri::async_runtime::spawn(async move {
        worker::run_worker(rx, worker_index, worker_fields, worker_vault, worker_status).await;
    });

    // Start file watcher
    let watcher = match watcher::start_watcher(vault_dir, tx.clone()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("Warning: file watcher failed to start: {e}");
            None
        }
    };

    Ok(SearchState {
        index,
        reader,
        fields,
        status,
        op_sender: tx,
        _watcher: watcher,
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
    match state.status.load(Ordering::SeqCst) {
        STATUS_BUILDING => "building".into(),
        STATUS_READY => "ready".into(),
        _ => "error".into(),
    }
}

#[tauri::command]
pub fn list_tags(state: tauri::State<'_, SearchState>) -> Result<Vec<String>, String> {
    let searcher = state.reader.searcher();
    let mut tags = std::collections::BTreeSet::new();

    for segment_reader in searcher.segment_readers() {
        let inv_index = segment_reader.inverted_index(state.fields.tags)
            .map_err(|e| format!("Failed to read tags index: {e}"))?;
        let mut term_stream = inv_index.terms().stream()
            .map_err(|e| format!("Failed to stream terms: {e}"))?;
        while term_stream.advance() {
            if let Ok(s) = std::str::from_utf8(term_stream.key()) {
                tags.insert(s.to_string());
            }
        }
    }

    Ok(tags.into_iter().collect())
}

#[tauri::command]
pub fn search_rebuild(state: tauri::State<'_, SearchState>) -> Result<(), String> {
    state
        .op_sender
        .send(IndexOp::Rebuild)
        .map_err(|_| "Search worker not running".to_string())
}
