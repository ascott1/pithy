use std::path::Path;
use tantivy::schema::*;
use tantivy::Index;

#[derive(Clone)]
pub struct SchemaFields {
    pub path: Field,
    pub filename_stem: Field,
    pub body: Field,
    pub tags: Field,
    pub modified_date: Field,
}

pub fn build_schema() -> (Schema, SchemaFields) {
    let mut builder = Schema::builder();

    let path = builder.add_text_field("path", STRING | STORED);
    let filename_stem = builder.add_text_field("filename_stem", TEXT | STORED);
    let body = builder.add_text_field("body", TEXT | STORED);
    let tags = builder.add_text_field("tags", STRING | STORED);
    let modified_date = builder.add_i64_field(
        "modified_date",
        NumericOptions::default()
            .set_fast()
            .set_stored()
            .set_indexed(),
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

pub fn open_or_create_index(index_dir: &Path, schema: &Schema) -> Result<Index, String> {
    std::fs::create_dir_all(index_dir)
        .map_err(|e| format!("Failed to create index directory: {e}"))?;

    match Index::open_in_dir(index_dir) {
        Ok(index) => Ok(index),
        Err(_open_err) => {
            // If opening failed, try creating. If that also fails due to a stale
            // lock from a previous crash, remove the lock and retry once.
            match Index::create_in_dir(index_dir, schema.clone()) {
                Ok(index) => Ok(index),
                Err(_create_err) => {
                    let lock_path = index_dir.join(".tantivy-writer.lock");
                    if lock_path.exists() {
                        eprintln!(
                            "Removing stale Tantivy lock file (likely from a previous crash)"
                        );
                        let _ = std::fs::remove_file(&lock_path);
                    }
                    // Final attempt — if this fails, propagate the error
                    Index::create_in_dir(index_dir, schema.clone())
                        .map_err(|e| format!("Failed to create search index: {e}"))
                }
            }
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
        assert_eq!(
            schema.get_field_name(fields.modified_date),
            "modified_date"
        );
    }

    #[test]
    fn open_or_create_creates_new_index() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path().join("search");
        let (schema, _fields) = build_schema();

        let index = open_or_create_index(&index_dir, &schema);
        assert!(index.is_ok(), "Should create a new index");
        assert!(index_dir.join("meta.json").exists());
    }

    #[test]
    fn open_or_create_reopens_existing() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path().join("search");
        let (schema, _fields) = build_schema();

        let index1 = open_or_create_index(&index_dir, &schema).unwrap();
        let index2 = open_or_create_index(&index_dir, &schema).unwrap();

        assert_eq!(
            index1.schema().num_fields(),
            index2.schema().num_fields(),
            "Reopened index should have the same schema"
        );
    }
}
