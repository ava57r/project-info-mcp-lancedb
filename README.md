# project-info-mcp-lancedb


# opencode-mcp-lancedb

```json
{
  "mcp": {
    "project-kameo-lancedb": {
      "type": "local",
      "command": ["/full/path/to/target/release/openvino-lancedb-kameo-mcp"],
      "environment": {
        "KB_STORAGE_PATH": "./.opencode_memory/kameo_db",
        "EMBED_API_BASE": "http://localhost:8002/v1/embeddings",
        "EMBED_MODEL": "qwen3-embed",
        "VECTOR_DIMENSION": "1024"
      },
      "enabled": true
    }
  }
}
```
