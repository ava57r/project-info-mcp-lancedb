use async_trait::async_trait;
use kameo::actor::ActorRef;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::types::{MessageContent, ToolResult};
use serde::{Deserialize, Serialize};

use crate::actors::UpsertMessage;
use crate::actors::memory::MemoryActor;
use crate::mcp::{McpTool, PropertyDescription};

pub struct UpsertProjectInfo {
    upsert_properties_description: UpsertPropertiesDescription,
}

impl UpsertProjectInfo {
    pub fn new() -> Self {
        UpsertProjectInfo {
            upsert_properties_description: UpsertPropertiesDescription::new(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPropertiesDescription {
    pub info_id: PropertyDescription,
    pub content: PropertyDescription,
    pub category: PropertyDescription,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProperties {
    pub info_id: String,
    pub content: String,
    pub category: String,
}

impl UpsertPropertiesDescription {
    pub fn new() -> Self {
        UpsertPropertiesDescription {
            info_id: PropertyDescription {
                type_field: "string".to_string(),
                description: "Record ID, unique key (for example, path to file or task ID)".to_string(),
            },
            content: PropertyDescription {
                type_field: "string".to_string(),
                description: "Discrete Facts or short content text".to_string(),
            },
            category: PropertyDescription {
                type_field: "string".to_string(),
                description: "Record category (for example, 'architecture', 'todo', 'api', 'changelog', etc.)".to_string(),
            },
        }
    }

    pub fn required(&self) -> Vec<String> {
        vec![
            "info_id".to_string(),
            "content".to_string(),
            "category".to_string(),
        ]
    }
}

#[async_trait]
impl McpTool for UpsertProjectInfo {
    fn name(&self) -> String {
        "upsert_project_info".to_string()
    }
    fn description(&self) -> String {
        "Upsert project info".to_string()
    }
    fn properties(&self) -> Option<serde_json::Value> {
        Some(serde_json::to_value(&self.upsert_properties_description).expect("Must serialize"))
    }
    fn required_properties(&self) -> Option<Vec<String>> {
        Some(self.upsert_properties_description.required())
    }

    async fn call(
        &self,
        actor: ActorRef<MemoryActor>,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let Ok(args) = serde_json::from_value::<UpsertProperties>(
            arguments.unwrap_or_else(|| serde_json::json!({})),
        ) else {
            return Err(McpError::protocol(
                ErrorCode::InvalidParams,
                "Invalid arguments".to_string(),
            ));
        };
        let msg = UpsertMessage {
            id: args.info_id,
            content: args.content,
            category: args.category,
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
