//! Runtime configuration loaded from environment variables.

use std::env;

use crate::helpers::DEFAULT_COLLECTION;
use piim_common::actors::embedding::fastembed::FastembedModel;
use piim_common::config::CommonConfig;

/// Embedding backend selection.
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingBackend {
    /// Use an external OpenAI-compatible HTTP endpoint.
    Http,
    /// Use local inference via fastembed.
    Fastembed(FastembedModel),
}

impl EmbeddingBackend {
    /// Parse backend from `EMBEDDINGS_BACKEND` env var.
    ///
    /// Supports: `http` (default), `fastembed`.
    /// When `fastembed`, the model is resolved from `EMBEDDINGS_MODEL`.
    pub fn from_env() -> Self {
        match env::var("EMBEDDINGS_BACKEND").ok().as_deref() {
            Some("fastembed") => {
                let model_name =
                    env::var("EMBEDDINGS_MODEL").unwrap_or_else(|_| "qwen3-embed".to_string());
                Self::Fastembed(FastembedModel::from_http_name(&model_name))
            }
            Some(_) | None => Self::Http,
        }
    }

    /// Return the vector dimension for this backend.
    pub fn dimension(&self) -> usize {
        match self {
            Self::Http => {
                // For HTTP we fall back to the common config value
                CommonConfig::from_env().vector_dimension
            }
            Self::Fastembed(model) => model.dimension(),
        }
    }
}

/// Runtime configuration loaded from environment variables, with fallback defaults.
pub struct Config {
    /// URL of the Qdrant server (`QDRANT_URL`).
    pub qdrant_url: String,
    /// API key for the Qdrant server (`QDRANT_API_KEY`, optional).
    pub qdrant_api_key: Option<String>,
    /// Name of the Qdrant collection (`QDRANT_COLLECTION`).
    pub collection_name: String,
    /// URL of the OpenAI-compatible embeddings endpoint (`EMBEDDINGS_URL`).
    /// Used only when [`Self::embedding_backend`] is [`EmbeddingBackend::Http`].
    pub embeddings_url: String,
    /// Embedding model name (`EMBEDDINGS_MODEL`).
    pub model: String,
    /// Embedding backend selection.
    pub embedding_backend: EmbeddingBackend,
    /// Expected embedding vector dimension (`VECTOR_DIMENSION`).
    ///
    /// For local backend this is derived from the model automatically,
    /// but kept for backwards compatibility.
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

        let common = CommonConfig::from_env();
        let backend = EmbeddingBackend::from_env();
        let vector_dimension = backend.dimension();

        Config {
            qdrant_url,
            qdrant_api_key,
            collection_name,
            embeddings_url: common.embeddings_url,
            model: common.model,
            embedding_backend: backend,
            vector_dimension,
            project: common.project,
        }
    }
}
