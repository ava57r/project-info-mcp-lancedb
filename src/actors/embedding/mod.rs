mod request;
mod response;

use kameo::actor::{Actor, ActorRef};
use kameo::message::{Context, Message};
use reqwest::Client;

use crate::actors::EmbeddingMessage;
use crate::actors::embedding::request::EmbeddingParams;
use request::EmbeddingRequest;
use response::EmbeddingResponse;

pub const ENCODING_FORMAT: &str = "float";

pub struct EmbeddingActor {
    http_client: Client,
    embeddings_url: String,
    model: String,
    pooling: Option<String>, // "LAST" or "MEAN"
}

impl EmbeddingActor {
    pub fn new(
        http_client: Client,
        embeddings_url: String,
        model: String,
        pooling: Option<String>,
    ) -> Self {
        EmbeddingActor {
            http_client,
            embeddings_url,
            model,
            pooling,
        }
    }
}

impl Actor for EmbeddingActor {
    type Args = EmbeddingActor;

    type Error = anyhow::Error;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

impl Message<EmbeddingMessage> for EmbeddingActor {
    type Reply = Result<Vec<f32>, String>;

    async fn handle(
        &mut self,
        msg: EmbeddingMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let response = self
            .http_client
            .post(&self.embeddings_url)
            .json(&EmbeddingRequest {
                input: &msg.query,
                model: &self.model,
                encoding_format: ENCODING_FORMAT,
                params: self.pooling.as_ref().map(|p| EmbeddingParams {
                    pooling: p.to_string(),
                }),
            })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let parsed: EmbeddingResponse = response.json().await.map_err(|e| e.to_string())?;

        if let Some(d) = parsed.data.first() {
            Ok(d.embedding.clone())
        } else {
            Err("Model returned empty vectors array".to_string())
        }
    }
}
