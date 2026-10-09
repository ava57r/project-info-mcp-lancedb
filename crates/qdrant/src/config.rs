//! Runtime configuration loaded from environment variables.

use std::env;

use crate::helpers::{DEFAULT_COLLECTION, DEFAULT_PROJECT};

/// Runtime configuration loaded from environment variables, with fallback defaults.
pub struct Config {
    /// URL of the Qdrant server (`QDRANT_URL`).
    pub qdrant_url: String,
    /// API key for the Qdrant server (`QDRANT_API_KEY`, optional).
    pub qdrant_api_key: Option<String>,
    /// Name of the Qdrant collection (`QDRANT_COLLECTION`).
    pub collection_name: String,
    /// URL of the OpenAI-compatible embeddings endpoint (`EMBEDDINGS_URL`).
    pub embeddings_url: String,
    /// Embedding model name (`EMBEDDINGS_MODEL`).
    pub model: String,
    /// Expected embedding vector dimension (`VECTOR_DIMENSION`).
    pub vector_dimension: usize,
    /// Default project name used to scope records (`PROJECT_NAME`).
    ///
    /// Multiple projects can share one Qdrant collection: every point carries
    /// a `project` payload field and all reads/writes are filtered by it. The value
    /// can be overridden per tool call via the optional `project` argument.
    pub project: String,
}

impl Config {
    /// Loads configuration from environment variables, falling back to defaults when unset or invalid.
    pub fn get_from_env() -> Config {
        let qdrant_url =
            env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

        let qdrant_api_key = env::var("QDRANT_API_KEY").ok();

        let collection_name =
            env::var("QDRANT_COLLECTION").unwrap_or_else(|_| DEFAULT_COLLECTION.to_string());

        let embeddings_url = env::var("EMBEDDINGS_URL")
            .unwrap_or_else(|_| "http://localhost:8002/v1/embeddings".to_string());

        let model = env::var("EMBEDDINGS_MODEL").unwrap_or_else(|_| "qwen3-embed".to_string());

        let vector_dimension = env::var("VECTOR_DIMENSION")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);

        let project = env::var("PROJECT_NAME")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PROJECT.to_string());

        Config {
            qdrant_url,
            qdrant_api_key,
            collection_name,
            embeddings_url,
            model,
            vector_dimension,
            project,
        }
    }
}
