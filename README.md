# project-info-mcp-lancedb

A persistent project-memory MCP server in Rust. Stores discrete facts (architecture notes, TODOs, API contracts, changelogs) in [LanceDB](https://lancedb.com) with vector + full-text hybrid search, exposed to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io) over stdio.

Built with `kameo` actors (`MemoryActor` + `EmbeddingActor`), `lancedb` for storage, and any OpenAI-compatible embeddings API (e.g. Qwen3-embed, TEI, vLLM).

## Features

- **Persistent memory** — `project_memory` LanceDB table (`id`, `content`, `vector`, `category`, `file_hash`, `timestamp`).
- **Hybrid search** — ANN vector search over embeddings + FTS index on `content`, with optional `category` filter.
- **Content-aware upsert** — SHA-256 `file_hash` check skips embedding inference when content is unchanged; otherwise delete + re-insert.
- **Actor isolation** — `MemoryActor` owns the LanceDB table; `EmbeddingActor` owns the HTTP client for the embeddings API.
- **Zero-config defaults** — works out of the box against `http://localhost:8002/v1/embeddings`.

## Prerequisites

- Rust 1.80+ (`cargo build`)
- A running OpenAI-compatible embeddings endpoint, e.g.:
  ```bash
  curl -X POST http://localhost:8002/v1/embeddings \
    -H 'Content-Type: application/json' \
    -d '{"input":"hello","model":"qwen3-embed","encoding_format":"float"}'
  ```

## Build & Run

```bash
cargo build --release
./target/release/project-info-mcp-lancedb
```

With custom config:

```bash
KB_STORAGE_PATH=./.opencode_memory/kameo_db \
EMBED_API_BASE=http://localhost:8002/v1/embeddings \
EMBED_MODEL=qwen3-embed \
VECTOR_DIMENSION=1024 \
./target/release/project-info-mcp-lancedb
```

## Configuration

| Env var | Default | Description |
|---|---|---|
| `KB_STORAGE_PATH` | `./.opencode_memory/kameo_db` | LanceDB storage directory (created on first run). |
| `EMBED_API_BASE` | `http://localhost:8002/v1/embeddings` | Embeddings HTTP endpoint (OpenAI-compatible). |
| `EMBED_MODEL` | `qwen3-embed` | Model name sent as `model` in the embedding request. |
| `VECTOR_DIMENSION` | `1024` | Expected embedding size; upsert fails fast on mismatch. |

On startup the server opens the `project_memory` table, or creates it with the Arrow schema from `src/helpers.rs` if missing.

## Client setup (opencode)

Add to `opencode.json`:

```json
{
  "mcp": {
    "project-info-lancedb": {
      "type": "local",
      "command": ["/full/path/to/target/release/project-info-mcp-lancedb"],
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

> Use an absolute `command` path and keep `VECTOR_DIMENSION` in sync with your embedding model, otherwise upserts are rejected.

## Tools

| Tool | Arguments | What it does |
|---|---|---|
| `upsert_project_info` | `info_id: string` (unique key, e.g. file path or task ID) <br> `content: string` (discrete fact / short text) <br> `category: string` (e.g. `architecture`, `todo`, `api`, `changelog`) | Hashes `content` (SHA-256); skips inference if hash matches existing row; otherwise embeds content via `EmbeddingActor` and `add()`s an Arrow record with current unix timestamp. |
| `hybrid_search_memory` | `query: string` <br> `limit: integer` <br> `category?: string` | Embeds `query`, ensures an FTS index on `content`, then runs `nearest_to(vector).limit(n)` with optional `category = '...'` predicate. |
| `optimize_database` | _(none — must be called with no arguments)_ | Runs LanceDB `optimize()` / compaction on the table. |

## Architecture

```text
stdin (JSON-RPC) → StdioTransport → Server(MemoryToolHandler)
                                            │ tools/call
                                            ▼
                                  MemoryActor (kameo)
                                   ├─ owns lancedb Table
                                   └─ asks EmbeddingActor ──POST EMBED_API_BASE──▶ embeddings API
```

- `src/main.rs` — stdio wiring, table open/create, actor spawn, MCP `Server::start()`.
- `src/config.rs` — `Config::get_from_env()` with defaults above.
- `src/actors/embed.rs` — `EmbeddingActor`: `POST {input, model, encoding_format:"float"}` → `Vec<f32>`.
- `src/actors/memory/{mod,upsert,search,optimize}.rs` — LanceDB queries, hash dedup, FTS index creation.
- `src/mcp/{mod,upsert,search,optimize}.rs` — `McpTool` impls (`tools/list`, `tools/call`).
- `src/helpers.rs` — `table_schema()` + `build_arrow_record()` (validates `vector.len() == VECTOR_DIMENSION`).

## Development

```bash
cargo fmt        # format
cargo clippy     # lint (must be clean)
cargo test --all # tests
```
