use kameo::actor::ActorRef;
use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};

/// Compact / optimize the LanceDB table (takes no arguments).
#[mcp_tool(name = "optimize_database", description = "Optimize database")]
#[derive(Debug, Default, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct OptimizeProjectInfo {}

pub async fn execute(actor: ActorRef<crate::actors::memory::MemoryActor>) -> String {
    let msg = crate::actors::OptimizeMessage {};
    match actor.ask(msg).await {
        Ok(text) => text,
        Err(e) => format!("❌ Failed to send message to Kameo actor: {e}"),
    }
}
