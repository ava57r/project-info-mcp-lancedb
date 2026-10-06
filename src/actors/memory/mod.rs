pub mod optimize;
pub mod search;
pub mod upsert;

use kameo::actor::{Actor, ActorRef};
use lancedb::table::Table;

use crate::actors::embed::EmbeddingActor;

pub struct MemoryActor {
    table: Table,
    embed_actor_ref: ActorRef<EmbeddingActor>,
    vector_dimension: usize,
}

impl MemoryActor {
    pub fn new(
        table: Table,
        embed_actor_ref: ActorRef<EmbeddingActor>,
        vector_dimension: usize,
    ) -> Self {
        MemoryActor {
            table,
            embed_actor_ref,
            vector_dimension,
        }
    }
}

impl Actor for MemoryActor {
    type Args = MemoryActor;

    type Error = anyhow::Error;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}
