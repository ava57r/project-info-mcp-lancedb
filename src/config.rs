use std::env;

pub struct Config {
    pub db_dir: String,
    pub embed_url: String,
    pub model_name: String,
    pub vector_dimension: usize,
}

impl Config {
    pub fn get_from_env() -> Config {
        let db_dir = env::var("KB_STORAGE_PATH")
            .unwrap_or_else(|_| "./.opencode_memory/kameo_db".to_string());

        let embed_url = env::var("EMBED_API_BASE")
            .unwrap_or_else(|_| "http://localhost:8002/v1/embeddings".to_string());

        let model_name = env::var("EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".to_string());

        let vector_dimension = env::var("VECTOR_DIMENSION")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024);

        Config {
            db_dir,
            embed_url,
            model_name,
            vector_dimension,
        }
    }
}
