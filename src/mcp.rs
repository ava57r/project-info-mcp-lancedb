use async_trait::async_trait;
use kameo::actor::ActorRef;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::server::ServerHandler;
use mcp_sdk_rs::types::{
    ClientCapabilities, Implementation, ListToolsResult, MessageContent, ServerCapabilities, Tool,
    ToolResult, ToolSchema,
};

use crate::actors::UpsertMessage;
use crate::actors::memory::MemoryActor;

pub struct MemoryToolHandler {
    pub actor: ActorRef<MemoryActor>,
}

#[async_trait]
impl ServerHandler for MemoryToolHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, McpError> {
        Ok(ServerCapabilities {
            tools: Some(serde_json::json!({})),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        match method {
            "tools/list" => {
                let tool = Tool {
                    name: "upsert_project_info".to_string(),
                    description: "Upsert project info".to_string(),
                    input_schema: Some(ToolSchema {
                        properties: Some(serde_json::json!({
                            "info_id": {
                                "type": "string",
                                "description": "Record ID"
                            },
                            "content": {
                                "type": "string",
                                "description": "Subtask or context text"
                            },
                            "category": {
                                "type": "string",
                                "description": "Record category"
                            },
                        })),
                        required: Some(vec![
                            "info_id".to_string(),
                            "content".to_string(),
                            "category".to_string(),
                        ]),
                    }),
                    annotations: None,
                };
                let result = ListToolsResult {
                    tools: vec![tool],
                    next_cursor: None,
                };
                Ok(serde_json::to_value(result)?)
            }
            "tools/call" => {
                let params = params.unwrap_or_else(|| serde_json::json!({}));
                let tool_name = params
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if tool_name != "upsert_project_info" {
                    return Err(McpError::protocol(
                        ErrorCode::InvalidParams,
                        format!("unknown tool: {tool_name}"),
                    ));
                }
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let msg = UpsertMessage {
                    id: required_arg(&args, "info_id")?,
                    content: required_arg(&args, "content")?,
                    category: required_arg(&args, "category")?,
                };
                let text = match self.actor.ask(msg).await {
                    Ok(text) => text,
                    Err(_) => "❌ Failed to send message to Kameo actor".to_string(),
                };
                let result = ToolResult {
                    content: vec![MessageContent::Text { text }],
                    structured_content: None,
                };
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(McpError::protocol(
                ErrorCode::MethodNotFound,
                format!("method not found: {method}"),
            )),
        }
    }
}

fn required_arg(args: &serde_json::Value, key: &str) -> Result<String, McpError> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            McpError::protocol(ErrorCode::InvalidParams, format!("missing argument: {key}"))
        })
}
