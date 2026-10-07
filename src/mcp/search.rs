//! MCP tool searching project memory with hybrid vector + full-text search.

use kameo::actor::ActorRef;
use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};

/// Search project memory with hybrid vector + full-text search.
#[mcp_tool(name = "hybrid_search_memory", description = "Search project info")]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct SearchProjectInfo {
    /// Search query.
    pub query: String,
    /// Record category filter (for example, 'file', 'architecture', 'todo', 'changelog', etc.).
    pub category: Option<String>,
    /// Maximum number of results to return.
    pub limit: u64,
}

/// Executes the search tool by forwarding the arguments to the project info actor.
pub async fn execute(
    actor: ActorRef<crate::actors::project_info::ProjectInfoActor>,
    args: SearchProjectInfo,
) -> String {
    let msg = crate::actors::SearchMessage {
        query: args.query,
        category: args.category,
        limit: usize::try_from(args.limit).unwrap_or(5),
    };
    match actor.ask(msg).await {
        Ok(text) => text,
        Err(e) => format!("❌ Failed to send message to Kameo actor: {e}"),
    }
}
