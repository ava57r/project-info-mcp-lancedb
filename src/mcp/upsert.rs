use kameo::actor::ActorRef;
use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};

/// Upsert a discrete project fact into LanceDB memory.
#[mcp_tool(name = "upsert_project_info", description = "Upsert project info")]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct UpsertProjectInfo {
    /// Record ID, unique key (for example, path to file or task ID).
    pub info_id: String,
    /// Discrete facts or short content text.
    pub content: String,
    /// Record category (for example, 'architecture', 'todo', 'api', 'changelog', etc.).
    pub category: String,
}

pub async fn execute(
    actor: ActorRef<crate::actors::project_info::ProjectInfoActor>,
    args: UpsertProjectInfo,
) -> String {
    let msg = crate::actors::UpsertMessage {
        id: args.info_id,
        content: args.content,
        category: args.category,
    };
    match actor.ask(msg).await {
        Ok(text) => text,
        Err(e) => format!("❌ Failed to send message to Kameo actor: {e}"),
    }
}
