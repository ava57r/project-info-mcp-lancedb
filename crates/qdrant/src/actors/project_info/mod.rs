//! Actor managing project info records in Qdrant.

pub mod delete;
pub mod list;
pub mod optimize;
pub mod reopen;
pub mod search;
pub mod stats;
pub mod structured;
pub mod upsert;

use kameo::actor::{Actor, ActorRef};
use qdrant_client::Qdrant;

use crate::actors::embedding::EmbeddingActor;

/// Kameo actor managing project info records in a Qdrant collection.
pub struct ProjectInfoActor {
    client: Qdrant,
    embed_actor_ref: ActorRef<EmbeddingActor>,
    collection_name: String,
    vector_name: String,
    default_project: String,
}

impl ProjectInfoActor {
    /// Creates the actor with the given Qdrant client, embedding actor reference, vector name, and default project.
    pub fn new(
        client: Qdrant,
        embed_actor_ref: ActorRef<EmbeddingActor>,
        collection_name: String,
        vector_name: String,
        default_project: String,
    ) -> Self {
        let default_project = normalize_project(&default_project);
        ProjectInfoActor {
            client,
            embed_actor_ref,
            collection_name,
            vector_name,
            default_project,
        }
    }

    /// Resolves the effective project scope: explicit value wins, otherwise the actor default.
    pub fn resolve_project(&self, project: &str) -> String {
        let trimmed = project.trim();
        if trimmed.is_empty() {
            self.default_project.clone()
        } else {
            trimmed.to_string()
        }
    }
}

/// Normalizes a project name, falling back to the default when blank.
pub fn normalize_project(project: &str) -> String {
    let trimmed = project.trim();
    if trimmed.is_empty() {
        crate::helpers::DEFAULT_PROJECT.to_string()
    } else {
        trimmed.to_string()
    }
}

impl Actor for ProjectInfoActor {
    type Args = ProjectInfoActor;

    type Error = anyhow::Error;

    /// Creates the actor from the arguments provided at spawn time.
    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}
