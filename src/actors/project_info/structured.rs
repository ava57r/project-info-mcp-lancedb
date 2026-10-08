//! Structured stats / search handlers for the REST API + dashboard.

use std::collections::BTreeMap;

use arrow_array::RecordBatch;
use arrow_array::{Array, cast::AsArray};
use kameo::message::{Context, Message};
use lancedb::index::{Index, scalar::FtsIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use tokio_stream::StreamExt;

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{
    EmbeddingMessage, SearchHit, StatsData, StatsStructuredMessage, StructuredSearchMessage,
};
use crate::helpers::escape_literal;

impl Message<StatsStructuredMessage> for ProjectInfoActor {
    type Reply = Result<StatsData, String>;

    /// Counts rows and aggregates per-category / per-project stats in a single scan.
    async fn handle(
        &mut self,
        msg: StatsStructuredMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let count_filter =
            (project != "*").then(|| format!("project = {}", escape_literal(&project)));
        let total = self
            .table
            .count_rows(count_filter)
            .await
            .map_err(|e| format!("Error counting rows: {e}"))? as usize;

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
        let mut per_project: BTreeMap<String, usize> = BTreeMap::new();
        let mut content_chars: usize = 0;
        let mut scanned: usize = 0;
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            scanned += batch.num_rows();
            accumulate(
                &batch,
                &mut per_category,
                &mut content_chars,
                &mut per_project,
            );
        }
        Ok(StatsData {
            total,
            scanned,
            content_chars,
            avg_chars: content_chars / scanned.max(1),
            per_category,
            per_project: if project == "*" {
                per_project
            } else {
                BTreeMap::new()
            },
        })
    }
}

impl Message<StructuredSearchMessage> for ProjectInfoActor {
    type Reply = Result<Vec<SearchHit>, String>;

    /// Embeds the query and runs hybrid vector search, replying with structured hits.
    async fn handle(
        &mut self,
        msg: StructuredSearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
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
        let mut qb = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| e.to_string())?
            .limit(msg.limit.max(1));
        let mut filters = Vec::new();
        if project != "*" {
            filters.push(format!("project = {}", escape_literal(&project)));
        }
        if let Some(cat) = msg.category.filter(|c| !c.trim().is_empty()) {
            filters.push(format!("category = {}", escape_literal(&cat)));
        }
        if !filters.is_empty() {
            qb = qb.only_if(filters.join(" AND "));
        }
        let mut stream = qb
            .execute()
            .await
            .map_err(|e| format!("Error during hybrid search: {e}"))?;
        let mut hits = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch.map_err(|e| e.to_string())?;
            collect_hits(&batch, &mut hits);
        }
        Ok(hits)
    }
}

fn accumulate(
    batch: &RecordBatch,
    per_category: &mut BTreeMap<String, usize>,
    content_chars: &mut usize,
    per_project: &mut BTreeMap<String, usize>,
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
        if let Some(idx) = project_idx {
            let values = batch.column(idx).as_string::<i32>();
            if row < values.len() {
                *per_project
                    .entry(values.value(row).to_string())
                    .or_insert(0) += 1;
            }
        }
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

fn distance_value(batch: &RecordBatch, row: usize) -> Option<f32> {
    let idx = batch.schema().index_of("_distance").ok()?;
    let col = batch.column(idx);
    if let Some(arr) = col.as_any().downcast_ref::<arrow_array::Float32Array>() {
        return (row < arr.len() && arr.is_valid(row)).then(|| arr.value(row));
    }
    if let Some(arr) = col.as_any().downcast_ref::<arrow_array::Float64Array>() {
        return (row < arr.len() && arr.is_valid(row)).then(|| arr.value(row) as f32);
    }
    None
}

fn collect_hits(batch: &RecordBatch, out: &mut Vec<SearchHit>) {
    for row in 0..batch.num_rows() {
        out.push(SearchHit {
            id: string_value(batch, "id", row),
            project: string_value(batch, "project", row),
            category: string_value(batch, "category", row),
            content: string_value(batch, "content", row),
            distance: distance_value(batch, row),
        });
    }
}
