//! Actor managing project info records in the LanceDB table.

pub mod delete;
pub mod list;
pub mod optimize;
pub mod reopen;
pub mod search;
pub mod stats;
pub mod structured;
pub mod upsert;

use kameo::actor::{Actor, ActorRef};
use lancedb::table::Table;

use crate::actors::embedding::EmbeddingActor;

/// Kameo actor managing project info records in a LanceDB table.
pub struct ProjectInfoActor {
    table: Table,
    embed_actor_ref: ActorRef<EmbeddingActor>,
    vector_dimension: usize,
    default_project: String,
    db_dir: String,
}

impl ProjectInfoActor {
    /// Creates the actor with the given LanceDB table, embedding actor reference, and vector dimension.
    pub fn new(
        table: Table,
        embed_actor_ref: ActorRef<EmbeddingActor>,
        vector_dimension: usize,
        default_project: String,
        db_dir: String,
    ) -> Self {
        let default_project = normalize_project(&default_project);
        ProjectInfoActor {
            table,
            embed_actor_ref,
            vector_dimension,
            default_project,
            db_dir,
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
