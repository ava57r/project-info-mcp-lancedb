use arrow_array::cast::AsArray;
use arrow_array::{Array, Float32Array, Float64Array, RecordBatch};
use kameo::message::{Context, Message};
use lancedb::index::{Index, scalar::FtsIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase};
use tokio_stream::StreamExt;

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{EmbeddingMessage, SearchMessage};

/// Single search hit extracted from a result batch.
struct SearchMatch {
    id: String,
    category: String,
    content: String,
    distance: Option<f32>,
}

impl Message<SearchMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Embeds the query, runs hybrid vector + full-text search, and replies with formatted matches.
    async fn handle(
        &mut self,
        msg: SearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let query_vector = self
            .embed_actor_ref
            .ask(EmbeddingMessage { query: msg.query })
            .await
            .map_err(|e| e.to_string())?;

        let _ = self
            .table
            .create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await;

        let mut query_builder = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| e.to_string())?
            .limit(msg.limit);
        if let Some(cat) = msg.category {
            query_builder = query_builder.only_if(format!("category = '{}'", cat));
        }

        let mut stream = query_builder
            .execute()
            .await
            .map_err(|e| format!("Error during hybrid search: {}", e))?;

        let mut found = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            collect_matches(&batch, &mut found);
        }

        Ok(format_matches(&found))
    }
}

/// Copies readable columns (`id`, `category`, `content`, `_distance`) out of a batch.
fn collect_matches(batch: &RecordBatch, found: &mut Vec<SearchMatch>) {
    for row in 0..batch.num_rows() {
        found.push(SearchMatch {
            id: string_value(batch, "id", row).unwrap_or_default(),
            category: string_value(batch, "category", row).unwrap_or_default(),
            content: string_value(batch, "content", row).unwrap_or_default(),
            distance: distance_value(batch, row),
        });
    }
}

fn string_value(batch: &RecordBatch, column: &str, row: usize) -> Option<String> {
    let idx = batch.schema().index_of(column).ok()?;
    let values = batch.column(idx).as_string::<i32>();
    if row >= values.len() {
        return None;
    }
    Some(values.value(row).to_string())
}

fn distance_value(batch: &RecordBatch, row: usize) -> Option<f32> {
    let idx = batch.schema().index_of("_distance").ok()?;
    let column = batch.column(idx);
    if let Some(values) = column.as_any().downcast_ref::<Float32Array>()
        && row < values.len()
    {
        return Some(values.value(row));
    }
    if let Some(values) = column.as_any().downcast_ref::<Float64Array>()
        && row < values.len()
    {
        return Some(values.value(row) as f32);
    }
    None
}

fn format_matches(found: &[SearchMatch]) -> String {
    if found.is_empty() {
        return "🔍 No matches found.".to_string();
    }
    let mut out = format!("🔍 Found {} match(es):\n", found.len());
    for (i, m) in found.iter().enumerate() {
        let distance = m
            .distance
            .map(|d| format!(" (distance: {d:.4})"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}. [{}] {}{}\n   {}\n",
            i + 1,
            m.category,
            m.id,
            distance,
            m.content
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{Float32Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    fn file_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("category", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("_distance", DataType::Float32, true),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["src/auth.rs", "src/main.rs"])),
                Arc::new(StringArray::from(vec!["file", "file"])),
                Arc::new(StringArray::from(vec![
                    "JWT validation helpers",
                    "stdio wiring and actor spawn",
                ])),
                Arc::new(Float32Array::from(vec![0.125, 0.5])),
            ],
        )
        .expect("valid batch")
    }

    #[test]
    fn collects_rows_with_distance() {
        let mut found = Vec::new();
        collect_matches(&file_batch(), &mut found);

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].id, "src/auth.rs");
        assert_eq!(found[0].category, "file");
        assert_eq!(found[0].content, "JWT validation helpers");
        assert_eq!(found[0].distance, Some(0.125));
        assert_eq!(found[1].id, "src/main.rs");
    }

    #[test]
    fn tolerates_missing_distance_column() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("category", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["jwt_validation_logic"])),
                Arc::new(StringArray::from(vec!["todo"])),
                Arc::new(StringArray::from(vec!["Fix flaky expiry test"])),
            ],
        )
        .expect("valid batch");

        let mut found = Vec::new();
        collect_matches(&batch, &mut found);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "jwt_validation_logic");
        assert_eq!(found[0].distance, None);
    }

    #[test]
    fn formats_matches_and_empty_result() {
        assert_eq!(format_matches(&[]), "🔍 No matches found.");

        let mut found = Vec::new();
        collect_matches(&file_batch(), &mut found);
        let text = format_matches(&found);

        assert!(text.contains("Found 2 match(es)"));
        assert!(text.contains("1. [file] src/auth.rs (distance: 0.1250)"));
        assert!(text.contains("JWT validation helpers"));
        assert!(text.contains("2. [file] src/main.rs (distance: 0.5000)"));
    }
}
