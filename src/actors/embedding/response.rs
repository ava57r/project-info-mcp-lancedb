use serde::Deserialize;

#[derive(Deserialize)]
pub struct Data {
    pub embedding: Vec<f32>,
}
#[derive(Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<Data>,
}
