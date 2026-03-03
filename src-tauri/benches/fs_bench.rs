use criterion::{criterion_group, criterion_main, Criterion};
use pithy_lib::fs::{atomic_write, find_wikilinks, normalize_for_match, stem_to_display, wikilink_stem};
use std::fs;
use tempfile::tempdir;

/// Benchmark: atomic_write with a typical ~2KB markdown note.
/// Target: < 10ms
fn bench_atomic_write_2kb(c: &mut Criterion) {
    let content = "# My Note\n\n".to_string()
        + &"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(30)
        + "\n\n## Section\n\n"
        + &"Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(20);

    let dir = tempdir().unwrap();
    let path = dir.path().join("note.md");

    c.bench_function("atomic_write_2kb", |b| {
        b.iter(|| {
            atomic_write(&path, content.as_bytes()).unwrap();
        });
    });
}

/// Benchmark: atomic_write with a larger ~50KB note.
fn bench_atomic_write_50kb(c: &mut Criterion) {
    let content = "# Large Note\n\n".to_string()
        + &"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.\n".repeat(300);

    let dir = tempdir().unwrap();
    let path = dir.path().join("large-note.md");

    c.bench_function("atomic_write_50kb", |b| {
        b.iter(|| {
            atomic_write(&path, content.as_bytes()).unwrap();
        });
    });
}

/// Benchmark: wikilink rewrite across 100 files.
/// Simulates the full compute + write cycle of update_wikilink_references.
/// Target: < 500ms for 100 affected files
fn bench_wikilink_rewrite_100_files(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("vault");
    fs::create_dir_all(&vault).unwrap();

    // Create 100 .md files, each containing 3 wikilinks to "old target"
    for i in 0..100 {
        let content = format!(
            "# Note {i}\n\nSee [[old target]] for details.\n\nAlso [[old target|alias]] and [[old target]].\n\nSome other [[unrelated]] link.\n"
        );
        let path = vault.join(format!("note-{i:03}.md"));
        fs::write(&path, &content).unwrap();
    }

    let target_norm = normalize_for_match("old target");
    let replacement = stem_to_display("new target");

    c.bench_function("wikilink_rewrite_100_files", |b| {
        b.iter(|| {
            // Phase 1: compute all replacements
            let mut writes: Vec<(std::path::PathBuf, String)> = Vec::new();

            for entry in walkdir::WalkDir::new(&vault)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file()
                        && e.path().extension().map_or(false, |ext| ext == "md")
                })
            {
                let content = fs::read_to_string(entry.path()).unwrap();
                let links = find_wikilinks(&content);
                let matching: Vec<(usize, usize)> = links
                    .into_iter()
                    .filter(|(start, end)| {
                        normalize_for_match(wikilink_stem(&content[*start..*end]))
                            == target_norm
                    })
                    .collect();

                if matching.is_empty() {
                    continue;
                }

                let mut new_content = content.clone();
                for (start, end) in matching.into_iter().rev() {
                    let inner = &content[start..end];
                    let stem_end = match inner.find('|') {
                        Some(pos) => start + pos,
                        None => end,
                    };
                    new_content.replace_range(start..stem_end, &replacement);
                }
                writes.push((entry.path().to_path_buf(), new_content));
            }

            // Phase 2: write all files
            for (path, content) in &writes {
                atomic_write(path, content.as_bytes()).unwrap();
            }
        });
    });
}

/// Benchmark: find_wikilinks parsing on a large document.
fn bench_find_wikilinks_large_doc(c: &mut Criterion) {
    let mut content = String::with_capacity(50_000);
    for i in 0..500 {
        content.push_str(&format!(
            "Line {i}: Some text with [[link-{i}]] and [[another link|alias {i}]] in it.\n"
        ));
    }

    c.bench_function("find_wikilinks_500_links", |b| {
        b.iter(|| {
            let links = find_wikilinks(&content);
            assert!(links.len() >= 500);
        });
    });
}

criterion_group!(
    benches,
    bench_atomic_write_2kb,
    bench_atomic_write_50kb,
    bench_wikilink_rewrite_100_files,
    bench_find_wikilinks_large_doc,
);
criterion_main!(benches);
