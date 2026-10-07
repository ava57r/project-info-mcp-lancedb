//! Runtime configuration loaded from environment variables.

use std::env;

/// Runtime configuration loaded from environment variables, with fallback defaults.
pub struct Config {
    /// Path to the LanceDB database directory (`LANCEDB_PATH`).
    pub db_dir: String,
    /// URL of the OpenAI-compatible embeddings endpoint (`EMBEDDINGS_URL`).
    pub embeddings_url: String,
    /// Embedding model name (`EMBEDDINGS_MODEL`).
    pub model: String,
    /// Expected embedding vector dimension (`VECTOR_DIMENSION`).
    pub vector_dimension: usize,
}

impl Config {
    /// Loads configuration from environment variables, falling back to defaults when unset or invalid.
    pub fn get_from_env() -> Config {
        let db_dir =
            env::var("LANCEDB_PATH").unwrap_or_else(|_| "./.opencode_memory/lance_db".to_string());

        let embeddings_url = env::var("EMBEDDINGS_URL")
            .unwrap_or_else(|_| "http://localhost:8002/v1/embeddings".to_string());

        let model = env::var("EMBEDDINGS_MODEL").unwrap_or_else(|_| "qwen3-embed".to_string());

        let vector_dimension = env::var("VECTOR_DIMENSION")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);

        Config {
            db_dir,
            embeddings_url,
            model,
            vector_dimension,
        }
    }
}
