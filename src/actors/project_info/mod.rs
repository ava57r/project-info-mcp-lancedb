//! Actor managing project info records in the LanceDB table.

pub mod optimize;
pub mod search;
pub mod upsert;

use kameo::actor::{Actor, ActorRef};
use lancedb::table::Table;

use crate::actors::embedding::EmbeddingActor;

/// Kameo actor managing project info records in a LanceDB table.
pub struct ProjectInfoActor {
    table: Table,
    embed_actor_ref: ActorRef<EmbeddingActor>,
    vector_dimension: usize,
}

impl ProjectInfoActor {
    /// Creates the actor with the given LanceDB table, embedding actor reference, and vector dimension.
    pub fn new(
        table: Table,
        embed_actor_ref: ActorRef<EmbeddingActor>,
        vector_dimension: usize,
    ) -> Self {
        ProjectInfoActor {
            table,
            embed_actor_ref,
            vector_dimension,
        }
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
