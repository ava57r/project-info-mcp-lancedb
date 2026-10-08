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
use crate::helpers::escape_literal;

impl Message<StatsMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Counts rows and aggregates per-category stats in a single scan of the table.
    async fn handle(
        &mut self,
        msg: StatsMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let count_filter = if project == "*" {
            None
        } else {
            Some(format!("project = {}", escape_literal(&project)))
        };
        let total = self
            .table
            .count_rows(count_filter)
            .await
            .map_err(|e| format!("Error counting rows: {e}"))?;

        if total == 0 {
            return Ok(format!(
                "📊 Memory stats for project '{project}': table is empty (0 records)."
            ));
        }

        let mut query = self.table.query().select(Select::Columns(vec![
            "project".to_string(),
            "category".to_string(),
            "content".to_string(),
        ]));
        if project != "*" {
            query = query.only_if(format!("project = {}", escape_literal(&project)));
        }
        let mut stream = query
            .execute()
            .await
            .map_err(|e| format!("Error scanning table for stats: {e}"))?;

        let mut per_category: BTreeMap<String, usize> = BTreeMap::new();
        let mut content_chars: usize = 0;
        let mut scanned: usize = 0;
        let all_projects = project == "*";
        let mut per_project: BTreeMap<String, usize> = BTreeMap::new();
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            scanned += batch.num_rows();
            accumulate(
                &batch,
                &mut per_category,
                &mut content_chars,
                all_projects.then_some(&mut per_project),
            );
        }

        let avg_chars = content_chars / scanned.max(1);
        let mut out = format!(
            "📊 Memory stats for project '{project}': {total} record(s), ~{content_chars} content chars (avg {avg_chars}/record)\n"
        );
        if all_projects {
            out.push_str("Projects:\n");
            for (name, count) in &per_project {
                out.push_str(&format!("- {name}: {count}\n"));
            }
        }
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
    mut per_project: Option<&mut BTreeMap<String, usize>>,
) {
    let schema = batch.schema();
    let category_idx = schema.index_of("category").ok();
    let content_idx = schema.index_of("content").ok();
    let project_idx = schema.index_of("project").ok();
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
        if let (Some(idx), Some(per_project)) = (project_idx, per_project.as_mut()) {
            let values = batch.column(idx).as_string::<i32>();
            if row < values.len() {
                *per_project
                    .entry(values.value(row).to_string())
                    .or_insert(0) += 1;
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
            Field::new("project", DataType::Utf8, false),
            Field::new("category", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["proj-a", "proj-a", "proj-b"])),
                Arc::new(StringArray::from(vec!["file", "todo", "file"])),
                Arc::new(StringArray::from(vec!["aaa", "bb", "c"])),
            ],
        )
        .expect("valid batch")
    }

    #[test]
    fn accumulates_category_counts_and_chars() {
        let mut per_category = BTreeMap::new();
        let mut per_project = BTreeMap::new();
        let mut content_chars = 0;
        accumulate(
            &stats_batch(),
            &mut per_category,
            &mut content_chars,
            Some(&mut per_project),
        );

        assert_eq!(per_category.get("file"), Some(&2));
        assert_eq!(per_category.get("todo"), Some(&1));
        assert_eq!(per_project.get("proj-a"), Some(&2));
        assert_eq!(per_project.get("proj-b"), Some(&1));
        assert_eq!(content_chars, 6);
    }

    #[test]
    fn normalizes_blank_project() {
        use crate::actors::project_info::normalize_project;

        assert_eq!(normalize_project(""), crate::helpers::DEFAULT_PROJECT);
        assert_eq!(normalize_project("  "), crate::helpers::DEFAULT_PROJECT);
        assert_eq!(normalize_project(" proj-a "), "proj-a");
    }
}
