//! Kameo actors and the messages exchanged between them.

pub mod embedding;
pub mod project_info;

use serde::{Deserialize, Serialize};

/// Message requesting insertion or replacement of a project info record.
#[derive(Serialize, Deserialize)]
pub struct UpsertMessage {
    /// Unique record key (for example, file path or task ID).
    pub id: String,
    /// Content text to store.
    pub content: String,
    /// Record category (for example, `file`, `todo`, `architecture`).
    pub category: String,
}

/// Message requesting a hybrid vector + full-text search over project info.
#[derive(Serialize, Deserialize)]
pub struct SearchMessage {
    /// Search query text.
    pub query: String,
    /// Optional record category filter.
    pub category: Option<String>,
    /// Maximum number of results to return.
    pub limit: usize,
}

/// Message requesting an embedding vector for the given query text.
#[derive(Serialize, Deserialize)]
pub struct EmbeddingMessage {
    /// Text to embed.
    pub query: String,
}

/// Message requesting compaction / optimization of the LanceDB table.
#[derive(Serialize, Deserialize)]
pub struct OptimizeMessage {}
