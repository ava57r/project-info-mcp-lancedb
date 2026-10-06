pub mod embed;
pub mod memory;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UpsertMessage {
    pub id: String,
    pub content: String,
    pub category: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchMessage {
    pub query: String,
    pub category: Option<String>,
    pub limit: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingMessage {
    pub query: String,
}

#[derive(Serialize, Deserialize)]
pub struct OptimizeMessage {}
