pub mod optimize;
pub mod search;
pub mod upsert;

use kameo::actor::{Actor, ActorRef};
use lancedb::table::Table;

use crate::actors::embedding::EmbeddingActor;

pub struct ProjectInfoActor {
    table: Table,
    embed_actor_ref: ActorRef<EmbeddingActor>,
    vector_dimension: usize,
}

impl ProjectInfoActor {
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

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}
