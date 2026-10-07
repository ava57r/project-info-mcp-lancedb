use serde::Serialize;

#[derive(Serialize)]
pub struct EmbeddingParams {
    pub pooling: String, // "LAST" or "MEAN"
}

#[derive(Serialize)]
pub struct EmbeddingRequest<'a> {
    pub input: &'a str,
    pub model: &'a str,
    pub encoding_format: &'a str,
    pub params: Option<EmbeddingParams>,
}
