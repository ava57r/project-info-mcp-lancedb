use async_trait::async_trait;
use kameo::actor::ActorRef;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::{MessageContent, ToolResult};
use serde::{Deserialize, Serialize};

use crate::actors::SearchMessage;
use crate::actors::memory::MemoryActor;
use crate::mcp::{McpTool, PropertyDescription};

pub struct SearchProjectInfo {
    search_properties_description: SearchPropertiesDescription,
}

impl SearchProjectInfo {
    pub fn new() -> Self {
        SearchProjectInfo {
            search_properties_description: SearchPropertiesDescription::new(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPropertiesDescription {
    pub query: PropertyDescription,
    pub category: PropertyDescription,
    pub limit: PropertyDescription,
}

impl SearchPropertiesDescription {
    pub fn new() -> Self {
        SearchPropertiesDescription {
            query: PropertyDescription {
                type_field: "string".to_string(),
                description: "Search query".to_string(),
            },
            category: PropertyDescription {
                type_field: "string".to_string(),
                description: "Record category (for example, 'architecture', 'todo', 'api', 'changelog', etc.)".to_string(),
            },
            limit: PropertyDescription {
                type_field: "integer".to_string(),
                description: "Maximum number of results to return".to_string(),
            },
        }
    }

    pub fn required(&self) -> Vec<String> {
        vec!["query".to_string(), "limit".to_string()]
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProperties {
    pub query: String,
    pub category: Option<String>,
    pub limit: usize,
}

#[async_trait]
impl McpTool for SearchProjectInfo {
    fn name(&self) -> String {
        "search_project_info".to_string()
    }
    fn description(&self) -> String {
        "Search project info".to_string()
    }

    fn properties(&self) -> Option<serde_json::Value> {
        Some(serde_json::to_value(&self.search_properties_description).expect("Must serialize"))
    }
    fn required_properties(&self) -> Option<Vec<String>> {
        Some(self.search_properties_description.required())
    }

    async fn call(
        &self,
        actor: ActorRef<MemoryActor>,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let Ok(args) = serde_json::from_value::<SearchProperties>(
            arguments.unwrap_or_else(|| serde_json::json!({})),
        ) else {
            return Err(McpError::protocol(
                ErrorCode::InvalidParams,
                "Invalid arguments".to_string(),
            ));
        };
        let msg = SearchMessage {
            query: args.query,
            category: args.category,
            limit: args.limit,
        };
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
