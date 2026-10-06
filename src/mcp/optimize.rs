use async_trait::async_trait;
use kameo::actor::ActorRef;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::{MessageContent, ToolResult};

use crate::actors::OptimizeMessage;
use crate::actors::memory::MemoryActor;
use crate::mcp::McpTool;

pub struct OptimizeProjectInfo {}

impl OptimizeProjectInfo {
    pub fn new() -> Self {
        OptimizeProjectInfo {}
    }
}

#[async_trait]
impl McpTool for OptimizeProjectInfo {
    fn name(&self) -> String {
        "optimize_database".to_string()
    }
    fn description(&self) -> String {
        "Optimize database".to_string()
    }

    fn properties(&self) -> Option<serde_json::Value> {
        None
    }
    fn required_properties(&self) -> Option<Vec<String>> {
        None
    }

    async fn call(
        &self,
        actor: ActorRef<MemoryActor>,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        if arguments.is_some() {
            return Err(McpError::protocol(
                ErrorCode::InvalidParams,
                "Invalid arguments".to_string(),
            ));
        };
        let msg = OptimizeMessage {};
        let text = match actor.ask(msg).await {
            Ok(text) => text,
            Err(_) => "❌ Failed to send message to Kameo actor".to_string(),
        };
        let result = ToolResult {
            content: vec![MessageContent::Text { text }],
            structured_content: None,
        };
        Ok(serde_json::to_value(result)?)
    }
}
