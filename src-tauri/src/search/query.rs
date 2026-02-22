use crate::search::schema::SchemaFields;
use serde::Serialize;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::snippet::SnippetGenerator;
use tantivy::schema::Value;
use tantivy::{Index, IndexReader, TantivyDocument, Term};

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

pub fn execute_search(
    reader: &IndexReader,
    fields: &SchemaFields,
    index: &Index,
    query_str: &str,
    limit: usize,
    offset: usize,
) -> Result<SearchResponse, String> {
    let tokens: Vec<&str> = query_str.split_whitespace().collect();

    let tag_filters: Vec<&str> = tokens
        .iter()
        .filter(|t| t.starts_with('#') && t.len() > 1)
        .copied()
        .collect();

    let free_text: String = tokens
        .iter()
        .filter(|t| !t.starts_with('#') || t.len() <= 1)
        .copied()
        .collect::<Vec<&str>>()
        .join(" ");

    let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

    // Free text query across filename_stem (boosted) and body
    let free_text_trimmed = free_text.trim();
    if !free_text_trimmed.is_empty() {
        let mut query_parser =
            QueryParser::for_index(index, vec![fields.filename_stem, fields.body]);
        query_parser.set_field_boost(fields.filename_stem, 3.0);
        query_parser.set_field_boost(fields.body, 1.0);

        let parsed = query_parser
            .parse_query(free_text_trimmed)
            .map_err(|e| format!("Failed to parse query: {e}"))?;
        subqueries.push((Occur::Must, parsed));
    }

    // Tag filters — each tag must be present
    for tag in &tag_filters {
        let tag_value = &tag[1..]; // strip leading #
        let term = Term::from_field_text(fields.tags, tag_value);
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        subqueries.push((Occur::Must, Box::new(term_query)));
    }

    if subqueries.is_empty() {
        return Ok(SearchResponse {
            hits: Vec::new(),
            query: query_str.to_string(),
        });
    }

    let combined_query = BooleanQuery::new(subqueries);

    let searcher = reader.searcher();
    let top_docs = searcher
        .search(&combined_query, &TopDocs::with_limit(offset + limit))
        .map_err(|e| format!("Search failed: {e}"))?;

    // Build snippet generator for the body field
    let snippet_generator = SnippetGenerator::create(&searcher, &combined_query, fields.body)
        .map_err(|e| format!("Failed to create snippet generator: {e}"))?;

    let mut hits = Vec::new();
    for (score, doc_address) in top_docs.into_iter().skip(offset) {
        let doc: TantivyDocument = searcher
            .doc(doc_address)
            .map_err(|e| format!("Failed to retrieve document: {e}"))?;

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
            .filter_map(|v| v.as_str().map(|s: &str| s.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::schema::{build_schema, open_or_create_index};
    use tantivy::doc;
    use tempfile::tempdir;

    fn setup_test_index() -> (Index, IndexReader, SchemaFields) {
        let dir = tempdir().unwrap();
        let (schema, fields) = build_schema();
        let index = open_or_create_index(dir.path(), &schema).unwrap();

        {
            let mut writer = index.writer(15_000_000).unwrap();
            writer
                .add_document(doc!(
                    fields.path => "notes/hello-world.md",
                    fields.filename_stem => "hello world",
                    fields.body => "This is a test note about hello world",
                    fields.tags => "greeting",
                    fields.tags => "test",
                    fields.modified_date => 1700000000i64
                ))
                .unwrap();
            writer
                .add_document(doc!(
                    fields.path => "notes/rust-tips.md",
                    fields.filename_stem => "rust tips",
                    fields.body => "Some tips about Rust programming language",
                    fields.tags => "rust",
                    fields.tags => "programming",
                    fields.modified_date => 1700001000i64
                ))
                .unwrap();
            writer.commit().unwrap();
        }

        let reader = index.reader().unwrap();
        (index, reader, fields)
    }

    #[test]
    fn empty_query_returns_empty() {
        let (index, reader, fields) = setup_test_index();
        let result = execute_search(&reader, &fields, &index, "  ", 10, 0).unwrap();
        assert!(result.hits.is_empty());
    }

    #[test]
    fn free_text_search_finds_match() {
        let (index, reader, fields) = setup_test_index();
        let result = execute_search(&reader, &fields, &index, "hello", 10, 0).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].path, "notes/hello-world.md");
    }

    #[test]
    fn tag_filter_narrows_results() {
        let (index, reader, fields) = setup_test_index();
        let result = execute_search(&reader, &fields, &index, "#rust", 10, 0).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].filename_stem, "rust tips");
    }

    #[test]
    fn combined_text_and_tag() {
        let (index, reader, fields) = setup_test_index();
        let result = execute_search(&reader, &fields, &index, "tips #programming", 10, 0).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].path, "notes/rust-tips.md");
    }

    #[test]
    fn offset_skips_results() {
        let (index, reader, fields) = setup_test_index();
        let all = execute_search(&reader, &fields, &index, "test tips", 10, 0).unwrap();
        let skipped = execute_search(&reader, &fields, &index, "test tips", 10, all.hits.len()).unwrap();
        assert!(skipped.hits.is_empty());
    }
}
