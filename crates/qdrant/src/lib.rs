use kameo::actor::{ActorRef, Spawn};
use piim_common::actors::{
    embedding::{self, config::EmbeddingBackend, fastembed, http},
    project_info::ProjectInfoActor,
};
use qdrant_client::Qdrant;
use rust_mcp_sdk::error::SdkResult;

use crate::config::Config;

mod config;
mod helpers;
mod store;

pub async fn init_project_actor() -> SdkResult<ActorRef<ProjectInfoActor>> {
    let config = Config::get_from_env();

    // Initialize Qdrant client
    let mut client_builder = Qdrant::from_url(&config.qdrant_url);
    if let Some(ref api_key) = config.qdrant_api_key {
        client_builder = client_builder.api_key(api_key.clone());
    }
    let client =
        client_builder
            .build()
            .map_err(|e| rust_mcp_sdk::error::McpSdkError::Internal {
                description: e.to_string(),
            })?;

    // Ensure the collection exists with proper vector config
    helpers::ensure_collection(
        &client,
        &config.collection_name,
        &config.model,
        config.vector_dimension,
    )
    .await
    .map_err(|e| rust_mcp_sdk::error::McpSdkError::Internal {
        description: e.to_string(),
    })?;

    // Create the appropriate embedding actor based on the selected backend.
    let embed_actor = match &config.embedding_backend {
        EmbeddingBackend::Http => Box::new(http::EmbeddingActor::new(
            config.embeddings_url.clone(),
            config.model.clone(),
            None,
        )) as Box<dyn embedding::Embedder>,
        EmbeddingBackend::Fastembed(model) => {
            let actor = fastembed::EmbeddingActor::new_fastembed(model.clone()).map_err(|e| {
                rust_mcp_sdk::error::McpSdkError::Internal {
                    description: format!("Failed to initialize local embedding model: {e}"),
                }
            })?;

            Box::new(actor) as Box<dyn embedding::Embedder>
        }
    };

    let store = store::QdrantStore::new(client, config.collection_name, config.model.clone());

    Ok(ProjectInfoActor::spawn(ProjectInfoActor::new(
        Box::new(store),
        embed_actor,
        config.project,
    )))
}
