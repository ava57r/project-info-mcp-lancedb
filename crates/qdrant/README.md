# piil-qdrant

MCP server for storing and searching project information in Qdrant with vector search.

## Prerequisites

- **Rust** (compiler)
- **Qdrant** — vector database server (local or remote)
- **Embeddings API** — endpoint for generating vector embeddings

## Quick Start

### 1. Run Qdrant

Locally via Docker:

```bash
docker run -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage:z \
  qdrant/qdrant
```

Or connect to an existing remote server.

### 2. Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `QDRANT_URL` | Yes | `http://localhost:6334` | Qdrant server URL. |
| `QDRANT_API_KEY` | No | — | API key for authentication. |
| `QDRANT_COLLECTION` | No | `project_memory` | Name of the Qdrant collection. |
| `EMBEDDINGS_URL` | Yes | `http://localhost:8002/v1/embeddings` | Embeddings API URL (OpenAI-compatible). |
| `EMBEDDINGS_MODEL` | No | `qwen3-embed` | Embedding model name. |
| `VECTOR_DIMENSION` | No | `1024` | Expected embedding vector size. |
| `PROJECT_NAME` | No | `default` | Default project scope. Overridable per tool call. |

Example:

```bash
export QDRANT_URL="http://localhost:6334"
export EMBEDDINGS_URL="http://localhost:8002/v1/embeddings"
export EMBEDDINGS_MODEL="qwen3-embed"
export VECTOR_DIMENSION=1024
export PROJECT_NAME="my-project"
```

### 3. Build

```bash
cargo build --release -p piil-qdrant
```

### 4. Run

```bash
cargo run -p piil-qdrant
```

The server connects to Qdrant, creates the collection if it doesn't exist, and starts listening via **stdio**.

## Using as an MCP Server

Start it through an MCP client — the server communicates over stdio using the MCP protocol.

## Client setup (opencode)

Add to `opencode.json`:

```json
{
  "mcp": {
    "piil-qdrant": {
      "type": "local",
      "command": ["/full/path/to/target/release/piil-qdrant"],
      "environment": {
        "QDRANT_URL": "http://localhost:6334",
        "EMBEDDINGS_URL": "http://localhost:8002/v1/embeddings",
        "EMBEDDINGS_MODEL": "qwen3-embed",
        "VECTOR_DIMENSION": "1024",
        "PROJECT_NAME": "my-project"
      },
      "enabled": true
    }
  }
}
```

> Use an absolute `command` path. Ensure Qdrant is running at `QDRANT_URL` and `VECTOR_DIMENSION` matches your embedding model. Optional: set `QDRANT_API_KEY` and `QDRANT_COLLECTION` if using authentication or a custom collection name.

## Client setup (Zed)

Add to `~/.config/zed/settings.json`:

```json
{
  "mcp": {
    "piil-qdrant": {
      "command": "/full/path/to/target/release/piil-qdrant",
      "args": [],
      "env": {
        "QDRANT_URL": "http://localhost:6334",
        "EMBEDDINGS_URL": "http://localhost:8002/v1/embeddings",
        "EMBEDDINGS_MODEL": "qwen3-embed",
        "VECTOR_DIMENSION": "1024",
        "PROJECT_NAME": "my-project"
      }
    }
  }
}
```

Or via `~/.config/zed/mcp.json`:

```json
{
  "mcp": {
    "piil-qdrant": {
      "command": "/full/path/to/target/release/piil-qdrant",
      "args": [],
      "env": {
        "QDRANT_URL": "http://localhost:6334",
        "EMBEDDINGS_URL": "http://localhost:8002/v1/embeddings",
        "EMBEDDINGS_MODEL": "qwen3-embed",
        "VECTOR_DIMENSION": "1024",
        "PROJECT_NAME": "my-project"
      }
    }
  }
}
```

> Use an absolute `command` path. Ensure Qdrant is running at `QDRANT_URL` and `VECTOR_DIMENSION` matches your embedding model. Optional: set `QDRANT_API_KEY` and `QDRANT_COLLECTION` if using authentication or a custom collection name.

## Structure

```
crates/qdrant/
├── src/
│   ├── main.rs          # Entry point, server initialization
│   ├── config.rs        # Configuration from environment variables
│   ├── helpers.rs       # Helper functions (collection creation)
│   ├── actors/          # Actors (embedding, project_info)
│   └── mcp/             # MCP tool handlers
├── Cargo.toml
└── README.md
```

## Architecture

```
┌──────────────┐     stdio (MCP)     ┌─────────────────┐
│  MCP Client  │ ◄─────────────────► │  piil-qdrant    │
│              │                     │  (MCP Server)    │
└──────────────┘                     └────────┬────────┘
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    │                         │                         │
                    ▼                         ▼                         ▼
            ┌──────────────┐       ┌─────────────────┐       ┌──────────────┐
            │ Embedding    │       │ ProjectInfo     │       │   Qdrant     │
            │ Actor        │──────►│ Actor           │──────►│ (vector DB)  │
            │ (LLM API)    │       │ (memory actor)  │       │              │
            └──────────────┘       └─────────────────┘       └──────────────┘
```

## License

MIT
