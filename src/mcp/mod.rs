pub mod optimize;
pub mod search;
pub mod upsert;

use std::collections::HashMap;

use async_trait::async_trait;
use kameo::actor::ActorRef;
use mcp_sdk_rs::ToolSchema;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::server::ServerHandler;
use mcp_sdk_rs::types::{
    ClientCapabilities, Implementation, ListToolsResult, ServerCapabilities, Tool,
};
use serde::{Deserialize, Serialize};

use crate::actors::memory::MemoryActor;
use crate::mcp::optimize::OptimizeProjectInfo;
use crate::mcp::search::SearchProjectInfo;
use crate::mcp::upsert::UpsertProjectInfo;

#[async_trait]
pub trait McpTool {
    fn tool(&self) -> Tool {
        Tool {
            name: self.name(),
            description: self.description(),
            input_schema: Some(ToolSchema {
                properties: self.properties(),
                required: self.required_properties(),
            }),
            annotations: None,
        }
    }
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn properties(&self) -> Option<serde_json::Value>;
    fn required_properties(&self) -> Option<Vec<String>>;
    async fn call(
        &self,
        actor: ActorRef<MemoryActor>,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError>;
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDescription {
    #[serde(rename = "type")]
    pub type_field: String,
    pub description: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

pub struct MemoryToolHandler {
    actor: ActorRef<MemoryActor>,
    tools: HashMap<String, Box<dyn McpTool + Send + Sync>>,
}

impl MemoryToolHandler {
    pub fn new(actor: ActorRef<MemoryActor>) -> Self {
        let upsert = Box::new(UpsertProjectInfo::new()) as Box<dyn McpTool + Send + Sync>;
        let search = Box::new(SearchProjectInfo::new()) as Box<dyn McpTool + Send + Sync>;
        let optimize = Box::new(OptimizeProjectInfo::new()) as Box<dyn McpTool + Send + Sync>;
        Self {
            actor,
            tools: [
                (upsert.name(), upsert),
                (search.name(), search),
                (optimize.name(), optimize),
            ]
            .into_iter()
            .collect(),
        }
    }
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
                let result = ListToolsResult {
                    tools: self.tools.values().map(|tool| tool.tool()).collect(),
                    next_cursor: None,
                };
                Ok(serde_json::to_value(result)?)
            }
            "tools/call" => {
                let params = serde_json::from_value::<ToolCallParams>(
                    params.unwrap_or_else(|| serde_json::json!({})),
                )?;
                let tool = self.tools.get(&params.name).ok_or_else(|| {
                    McpError::protocol(
                        ErrorCode::InvalidParams,
                        format!("unknown tool: {}", params.name),
                    )
                })?;
                tool.call(self.actor.clone(), params.arguments).await
            }
            _ => Err(McpError::protocol(
                ErrorCode::MethodNotFound,
                format!("method not found: {method}"),
            )),
        }
    }
}
