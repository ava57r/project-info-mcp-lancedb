use std::env;

pub struct Config {
    pub db_dir: String,
    pub embeddings_url: String,
    pub model: String,
    pub vector_dimension: usize,
}

impl Config {
    pub fn get_from_env() -> Config {
        let db_dir =
            env::var("LANCEDB_PATH").unwrap_or_else(|_| "./.opencode_memory/lance_db".to_string());

        let embeddings_url = env::var("EMBEDDINGS_URL")
            .unwrap_or_else(|_| "http://localhost:8002/v1/embeddings".to_string());

        let model = env::var("EMBEDDINGS_MODEL").unwrap_or_else(|_| "qwen3-embed".to_string());

        let vector_dimension = env::var("VECTOR_DIMENSION")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);

        Config {
            db_dir,
            embeddings_url,
            model,
            vector_dimension,
        }
    }
}
