//! Message handler listing stored points with filters (REST API / dashboard).

use arrow_array::RecordBatch;
use arrow_array::{Array, cast::AsArray};
use kameo::message::{Context, Message};
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use tokio_stream::StreamExt;

use crate::actors::ListPointsMessage;
use crate::actors::PointRecord;
use crate::actors::project_info::ProjectInfoActor;
use crate::helpers::escape_literal;

impl Message<ListPointsMessage> for ProjectInfoActor {
    type Reply = Result<(Vec<PointRecord>, usize), String>;

    /// Scans the table (project/category predicate + offset/limit + substring filter) and replies with points + total.
    async fn handle(
        &mut self,
        msg: ListPointsMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let count_filter = predicate(&project, msg.category.as_deref(), None);
        let total = self
            .table
            .count_rows(count_filter)
            .await
            .map_err(|e| format!("Error counting rows: {e}"))? as usize;

        let mut query = self.table.query().select(Select::Columns(vec![
            "id".to_string(),
            "project".to_string(),
            "content".to_string(),
            "category".to_string(),
            "file_hash".to_string(),
            "timestamp".to_string(),
        ]));
        if let Some(filter) = predicate(&project, msg.category.as_deref(), None) {
            query = query.only_if(filter);
        }
        let mut stream = query
            .execute()
            .await
            .map_err(|e| format!("Error scanning table: {e}"))?;

        let mut all = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            collect_points(&batch, &mut all);
        }

        // Substring filter on id + content (case-insensitive).
        let has_filter = msg.query.as_deref().is_some_and(|q| !q.trim().is_empty());
        let filtered: Vec<PointRecord> = match msg.query {
            Some(q) if !q.trim().is_empty() => {
                let needle = q.to_lowercase();
                all.into_iter()
                    .filter(|p| {
                        p.id.to_lowercase().contains(&needle)
                            || p.content.to_lowercase().contains(&needle)
                    })
                    .collect()
            }
            _ => all,
        };
        let filtered_total = if has_filter { filtered.len() } else { total };
        let page: Vec<PointRecord> = filtered
            .into_iter()
            .skip(msg.offset)
            .take(msg.limit.max(1))
            .collect();
        Ok((page, filtered_total))
    }
}

/// Builds an optional SQL predicate for project/category scoping.
fn predicate(project: &str, category: Option<&str>, _unused: Option<()>) -> Option<String> {
    let mut filters = Vec::new();
    if project != "*" {
        filters.push(format!("project = {}", escape_literal(project)));
    }
    if let Some(cat) = category.filter(|c| !c.trim().is_empty()) {
        filters.push(format!("category = {}", escape_literal(cat)));
    }
    if filters.is_empty() {
        None
    } else {
        Some(filters.join(" AND "))
    }
}

fn string_value(batch: &RecordBatch, column: &str, row: usize) -> String {
    batch
        .schema()
        .index_of(column)
        .ok()
        .and_then(|idx| {
            let values = batch.column(idx).as_string::<i32>();
            (row < values.len()).then(|| values.value(row).to_string())
        })
        .unwrap_or_default()
}

fn int_value(batch: &RecordBatch, column: &str, row: usize) -> i64 {
    batch.schema().index_of(column).ok().map_or(0, |idx| {
        if let Some(arr) = batch
            .column(idx)
            .as_any()
            .downcast_ref::<arrow_array::Int64Array>()
            && row < arr.len()
        {
            return arr.value(row);
        }
        0
    })
}

/// Copies readable columns out of a batch.
fn collect_points(batch: &RecordBatch, out: &mut Vec<PointRecord>) {
    for row in 0..batch.num_rows() {
        out.push(PointRecord {
            id: string_value(batch, "id", row),
            project: string_value(batch, "project", row),
            content: string_value(batch, "content", row),
            category: string_value(batch, "category", row),
            file_hash: string_value(batch, "file_hash", row),
            timestamp: int_value(batch, "timestamp", row),
        });
    }
}
