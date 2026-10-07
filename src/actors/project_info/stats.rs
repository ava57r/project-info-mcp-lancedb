//! Message handler reporting usage statistics about the project memory table.

use std::collections::BTreeMap;

use arrow_array::Array;
use arrow_array::RecordBatch;
use arrow_array::cast::AsArray;
use kameo::message::{Context, Message};
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use tokio_stream::StreamExt;

use crate::actors::StatsMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<StatsMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Counts rows and aggregates per-category stats in a single scan of the table.
    async fn handle(
        &mut self,
        _msg: StatsMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let total = self
            .table
            .count_rows(None)
            .await
            .map_err(|e| format!("Error counting rows: {e}"))?;

        if total == 0 {
            return Ok("📊 Memory stats: table is empty (0 records).".to_string());
        }

        let mut stream = self
            .table
            .query()
            .select(Select::Columns(vec![
                "category".to_string(),
                "content".to_string(),
            ]))
            .execute()
            .await
            .map_err(|e| format!("Error scanning table for stats: {e}"))?;

        let mut per_category: BTreeMap<String, usize> = BTreeMap::new();
        let mut content_chars: usize = 0;
        let mut scanned: usize = 0;
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            scanned += batch.num_rows();
            accumulate(&batch, &mut per_category, &mut content_chars);
        }

        let avg_chars = content_chars / scanned.max(1);
        let mut out = format!(
            "📊 Memory stats: {total} record(s), ~{content_chars} content chars (avg {avg_chars}/record)\n"
        );
        out.push_str("Categories:\n");
        for (category, count) in &per_category {
            out.push_str(&format!("- {category}: {count}\n"));
        }
        Ok(out)
    }
}

/// Accumulates per-category counts and content length from one batch.
fn accumulate(
    batch: &RecordBatch,
    per_category: &mut BTreeMap<String, usize>,
    content_chars: &mut usize,
) {
    let schema = batch.schema();
    let category_idx = schema.index_of("category").ok();
    let content_idx = schema.index_of("content").ok();
    for row in 0..batch.num_rows() {
        if let Some(idx) = category_idx {
            let values = batch.column(idx).as_string::<i32>();
            if row < values.len() {
                *per_category
                    .entry(values.value(row).to_string())
                    .or_insert(0) += 1;
            }
        }
        if let Some(idx) = content_idx {
            let values = batch.column(idx).as_string::<i32>();
            if row < values.len() {
                *content_chars += values.value(row).len();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::StringArray;
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    fn stats_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("category", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["file", "todo", "file"])),
                Arc::new(StringArray::from(vec!["aaa", "bb", "c"])),
            ],
        )
        .expect("valid batch")
    }

    #[test]
    fn accumulates_category_counts_and_chars() {
        let mut per_category = BTreeMap::new();
        let mut content_chars = 0;
        accumulate(&stats_batch(), &mut per_category, &mut content_chars);

        assert_eq!(per_category.get("file"), Some(&2));
        assert_eq!(per_category.get("todo"), Some(&1));
        assert_eq!(content_chars, 6);
    }
}
