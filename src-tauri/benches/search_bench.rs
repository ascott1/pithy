use criterion::{criterion_group, criterion_main, Criterion};
use pithy_lib::search::query::execute_search;
use pithy_lib::search::schema::{build_schema, open_or_create_index};
use tantivy::doc;
use tempfile::tempdir;

const LOREM_WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing",
    "elit", "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore",
    "et", "dolore", "magna", "aliqua", "enim", "ad", "minim", "veniam",
    "quis", "nostrud", "exercitation", "ullamco", "laboris", "nisi",
    "aliquip", "ex", "ea", "commodo", "consequat", "duis", "aute", "irure",
    "in", "reprehenderit", "voluptate", "velit", "esse", "cillum", "fugiat",
    "nulla", "pariatur", "excepteur", "sint", "occaecat", "cupidatat",
];

const TAGS: &[&str] = &[
    "rust", "programming", "notes", "daily", "project", "idea",
    "todo", "meeting", "research", "design", "review", "planning",
];

fn generate_body(i: usize) -> String {
    let mut body = String::with_capacity(500);
    for j in 0..8 {
        let idx = (i * 7 + j * 13) % LOREM_WORDS.len();
        let end = ((i * 3 + j * 11) % LOREM_WORDS.len()).max(idx + 1).min(LOREM_WORDS.len());
        for w in &LOREM_WORDS[idx..end] {
            body.push_str(w);
            body.push(' ');
        }
        body.push_str(". ");
    }
    body
}

/// Benchmark: full-text search across 10k indexed documents.
/// Target: < 200ms for first page of results
fn bench_search_10k_docs(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let index_dir = dir.path().join("search");
    let (schema, fields) = build_schema();
    let index = open_or_create_index(&index_dir, &schema).unwrap();

    // Index 10k documents
    {
        let mut writer = index.writer(50_000_000).unwrap();
        for i in 0..10_000 {
            let stem = format!("note-{i:05}");
            let body = generate_body(i);
            let tag1 = TAGS[i % TAGS.len()];
            let tag2 = TAGS[(i * 3 + 1) % TAGS.len()];
            let path = format!("notes/{stem}.md");

            writer
                .add_document(doc!(
                    fields.path => path,
                    fields.filename_stem => stem,
                    fields.body => body,
                    fields.tags => tag1,
                    fields.tags => tag2,
                    fields.modified_date => 1700000000i64 + i as i64
                ))
                .unwrap();
        }
        writer.commit().unwrap();
    }

    let reader = index.reader().unwrap();

    c.bench_function("search_10k_free_text", |b| {
        b.iter(|| {
            let result =
                execute_search(&reader, &fields, &index, "lorem ipsum dolor", 20, 0).unwrap();
            assert!(!result.hits.is_empty());
        });
    });

    c.bench_function("search_10k_tag_filter", |b| {
        b.iter(|| {
            let result =
                execute_search(&reader, &fields, &index, "#rust", 20, 0).unwrap();
            assert!(!result.hits.is_empty());
        });
    });

    c.bench_function("search_10k_combined", |b| {
        b.iter(|| {
            let result = execute_search(
                &reader,
                &fields,
                &index,
                "tempor incididunt #programming",
                20,
                0,
            )
            .unwrap();
            // May or may not have hits depending on distribution
            let _ = result;
        });
    });

    c.bench_function("search_10k_single_term", |b| {
        b.iter(|| {
            let result =
                execute_search(&reader, &fields, &index, "aliqua", 20, 0).unwrap();
            let _ = result;
        });
    });
}

criterion_group!(benches, bench_search_10k_docs);
criterion_main!(benches);
