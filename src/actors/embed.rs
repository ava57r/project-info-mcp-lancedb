use kameo::actor::{Actor, ActorRef};
use kameo::message::{Context, Message};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::actors::EmbeddingMessage;

pub const EMBED_FORMAT: &str = "float";

pub struct EmbeddingActor {
    http_client: Client,
    embed_url: String,
    model_name: String,
    pooling: Option<String>, // "LAST" or "MEAN"
}

impl EmbeddingActor {
    pub fn new(
        http_client: Client,
        embed_url: String,
        model_name: String,
        pooling: Option<String>,
    ) -> Self {
        EmbeddingActor {
            http_client,
            embed_url,
            model_name,
            pooling,
        }
    }
}

#[derive(Serialize)]
struct EmbedParams {
    pooling: String, // "LAST" or "MEAN"
}

#[derive(Serialize)]
struct Req<'a> {
    input: &'a str,
    model: &'a str,
    encoding_format: &'a str,
    params: Option<EmbedParams>,
}

#[derive(Deserialize)]
struct Data {
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct Res {
    data: Vec<Data>,
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
            .post(&self.embed_url)
            .json(&Req {
                input: &msg.query,
                model: &self.model_name,
                encoding_format: EMBED_FORMAT,
                params: self.pooling.as_ref().map(|p| EmbedParams {
                    pooling: p.to_string(),
                }),
            })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let parsed: Res = response.json().await.map_err(|e| e.to_string())?;

        if let Some(d) = parsed.data.first() {
            Ok(d.embedding.clone())
        } else {
            Err("Model returned empty vectors array".to_string())
        }
    }
}
