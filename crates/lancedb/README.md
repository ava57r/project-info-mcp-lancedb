# piim-lance

LanceDB storage backend library for PIIM (Project Info In MCP). Provides `LanceStore` and `init_project_actor()` for use with the `piim` binary or any downstream project via the `lance_store` Cargo feature.

Part of the [PIIM (Project Info In MCP)](../../README.md) workspace. Both backends are driven by **`piim-common`** (`crates/common`): `kameo` actors (`ProjectInfoActor` + `EmbeddingActor`), the MCP tool layer, and the `MemoryStore` backend trait. Embeddings come from either an external OpenAI-compatible API (e.g. Qwen3-embed, TEI, vLLM) or local inference via [fastembed-rs](https://github.com/anush008/fastembed-rs) (see [Local embeddings](#local-embeddings-fastembed) below).

## Features

- **Multi-project memory** — one LanceDB database serves several projects: every record carries a `project` column (`id`, `project`, `content`, `vector`, `category`, `file_hash`, `timestamp`). The active project defaults to `PROJECT_NAME` (`default`) and can be overridden per tool call via the optional `project` argument (`"*"` searches/stats across all projects).
- **Persistent memory** — `project_memory` LanceDB table with the schema above (no migration: pre-`project`-column tables must be recreated — delete the table directory or use a fresh `LANCEDB_PATH`).
- **Hybrid search** — ANN vector search over embeddings + FTS index on `content`, with optional `category` filter, always scoped to the active project (or `"*"` for all).
- **Content-aware upsert** — SHA-256 `file_hash` check skips embedding inference when content is unchanged; otherwise delete + re-insert. Dedup key is `(id, project)`, so the same file path can exist in many projects.
- **File catalog** — `save_file_description` stores one `file`-category record per source file *per project*; the `(file path, project)` pair is the unique id.
- **Actor isolation** — `ProjectInfoActor` owns the LanceDB `Table`; `EmbeddingActor` owns embedding inference (HTTP client or local fastembed).
- **Zero-config defaults** — works out of the box against `http://localhost:8002/v1/embeddings`; set `EMBEDDINGS_BACKEND=fastembed` for zero-service local inference.

## Prerequisites

- **Embeddings** — either an external API endpoint or local inference via [fastembed-rs](https://github.com/anush008/fastembed-rs). For the HTTP default:
  ```bash
  curl -X POST http://localhost:8002/v1/embeddings \
    -H 'Content-Type: application/json' \
    -d '{"input":"hello","model":"qwen3-embed","encoding_format":"float"}'
  ```

## Usage

Add `piim-lance` as a dependency and call `init_project_actor()` to get a `ProjectInfoActor` backed by LanceDB:

```rust
use piim_lance::init_project_actor;

let actor = init_project_actor().await?;
// Use the actor with MCP server setup...
```

## Environment Variables

| Env var | Default | Description |
|---|---|---|
| `LANCEDB_PATH` | `./.opencode_memory/lance_db` | LanceDB storage directory (created on first run). |
| `EMBEDDINGS_BACKEND` | `http` | Embedding backend: `http` (external OpenAI-compatible API) or `fastembed` (local inference, no service needed). |
| `EMBEDDINGS_URL` | `http://localhost:8002/v1/embeddings` | Embeddings HTTP endpoint (OpenAI-compatible). Only used when `EMBEDDINGS_BACKEND=http`. |
| `EMBEDDINGS_MODEL` | `qwen3-embed` | Model name: sent as `model` in the HTTP embedding request, or mapped to a local fastembed model (see table below) when `EMBEDDINGS_BACKEND=fastembed`. |
| `VECTOR_DIMENSION` | `1024` | Expected embedding size; upsert fails fast on mismatch. Ignored when `EMBEDDINGS_BACKEND=fastembed` (dimension is auto-detected from the local model). |
| `PROJECT_NAME` | `default` | Default project scope for all records; one shared DB can hold many projects. Overridable per tool call via the optional `project` argument. |
| `HTTP_PORT` | `6333` | Port for the built-in REST API + dashboard (Qdrant-style). Set to `0` or empty to disable HTTP (stdio MCP only). |
| `SNAPSHOT_DIR` | `./.opencode_memory/snapshots` | Directory where `.tar.gz` DB snapshots are stored. |

> Backend selection is parsed centrally in `piim-common` (`EmbeddingBackend::from_env()`): any other `EMBEDDINGS_BACKEND` value falls back to `http`.

## Local embeddings (fastembed)

Set `EMBEDDINGS_BACKEND=fastembed` to run embeddings locally without an external API. Models are downloaded once on first use and cached. `VECTOR_DIMENSION` is auto-detected, so you must create (or recreate) the `project_memory` table with the matching dimension — changing the model later requires a fresh `LANCEDB_PATH`.

**Supported models:**

| Model Name | Dimensions | Params | Notes |
|---|---|---|---|
| `bge-small-en` / `bge-small-en-v1.5` | 384 | 130M | Default ONNX model (fallback for unknown names) |
| `bge-base-en` / `bge-base-en-v1.5` | 768 | 110M | |
| `bge-large-en` / `bge-large-en-v1.5` | 1024 | 335M | |
| `nomic-embed-text` / `nomic-embed-text-v1` | 768 | 27M | |
| `nomic-embed-text-v1.5` | 768 | 27M | |
| `all-MiniLM-L6-v2` / `minilm-l6` | 384 | 22M | |
| `qwen3-embed` / `qwen3-embedding-0.6b` | 1024 | 0.6B | Candle-based, requires `qwen3` feature (enabled by default) |
| `qwen3-embedding-4b` | 4096 | 4B | Candle-based, requires `qwen3` feature |
| `qwen3-embedding-8b` | 4096 | 8B | Candle-based, requires `qwen3` feature |

**Example with local BGE model:**

```bash
export LANCEDB_PATH="./.opencode_memory/lance_db"
export EMBEDDINGS_BACKEND="fastembed"
export EMBEDDINGS_MODEL="bge-small-en"
export PROJECT_NAME="my-project"
```

**Example with local Qwen3 model:**

```bash
export LANCEDB_PATH="./.opencode_memory/lance_db"
export EMBEDDINGS_BACKEND="fastembed"
export EMBEDDINGS_MODEL="qwen3-embed"
export PROJECT_NAME="my-project"
```

> **Note:** The first run downloads the model (~100MB–8GB depending on model). Subsequent runs use the cached model with no network needed.

## Dashboard & REST API (Qdrant-style)

Alongside MCP stdio, `init_project_actor()` can expose an HTTP API + dashboard on `:HTTP_PORT` (default `6333`):

- Dashboard: `http://localhost:6333/dashboard` — collection overview, per-project/category stats, point browser (filter + pagination + delete), hybrid search, upsert form, snapshot manager. Page markup, stylesheet, and client script live in `src/http/static/` (`dashboard.html`, `style.css`, `app.js`), baked into the binary via `include_str!` and served at `/dashboard`, `/static/style.css`, `/static/app.js`.
- `GET /healthz`, `GET /readyz`, `GET /api/version`
- `GET /api/collections`, `GET /api/collections/stats?project=...`
- `GET /api/points?project=&category=&query=&limit=&offset=`
- `POST /api/points/upsert` `{"id","content","category","project?"}` (embeds like the MCP tool)
- `POST /api/points/search` `{"query","category?","limit?","project?"}`
- `DELETE /api/points/:id?project=...`
- `POST /api/optimize`
- Snapshots (`.tar.gz` of the whole `LANCEDB_PATH` dir):
  `GET /api/snapshots`, `POST /api/snapshots`, `POST /api/snapshots/:name/restore`, `DELETE /api/snapshots/:name`, `GET /api/snapshots/:name/download`.
  Restore wipes the DB dir, extracts the archive, and re-opens the actor's table handle.

```bash
curl localhost:6333/healthz
curl -X POST localhost:6333/api/snapshots
curl 'localhost:6333/api/collections/stats?project=*'
```

## Client setup (opencode)

Then add to `opencode.json` (HTTP backend):

```json
{
  "mcp": {
    "piim": {
      "type": "local",
      "command": ["/full/path/to/target/release/piim"],
      "environment": {
        "LANCEDB_PATH": "./.opencode_memory/lance_db",
        "EMBEDDINGS_BACKEND": "http",
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

For local embeddings (fastembed, no external service):

```json
{
  "mcp": {
    "piim": {
      "type": "local",
      "command": ["/full/path/to/target/release/piim"],
      "environment": {
        "LANCEDB_PATH": "./.opencode_memory/lance_db",
        "EMBEDDINGS_BACKEND": "fastembed",
        "EMBEDDINGS_MODEL": "bge-small-en",
        "PROJECT_NAME": "my-project"
      },
      "enabled": true
    }
  }
}
```

> Use an absolute `command` path. For the HTTP backend, keep `VECTOR_DIMENSION` in sync with your embedding model, otherwise upserts are rejected; the fastembed backend auto-detects the dimension from `EMBEDDINGS_MODEL`.

## Client setup (Zed)

Then add to `~/.config/zed/settings.json`:

```json
{
  "mcp": {
    "piim": {
      "command": "/full/path/to/target/release/piim",
      "args": [],
      "env": {
        "LANCEDB_PATH": "./.opencode_memory/lance_db",
        "EMBEDDINGS_BACKEND": "http",
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
    "piim": {
      "command": "/full/path/to/target/release/piim",
      "args": [],
      "env": {
        "LANCEDB_PATH": "./.opencode_memory/lance_db",
        "EMBEDDINGS_BACKEND": "http",
        "EMBEDDINGS_URL": "http://localhost:8002/v1/embeddings",
        "EMBEDDINGS_MODEL": "qwen3-embed",
        "VECTOR_DIMENSION": "1024",
        "PROJECT_NAME": "my-project"
      }
    }
  }
}
```

> Use an absolute `command` path. For the HTTP backend, keep `VECTOR_DIMENSION` in sync with your embedding model, otherwise upserts are rejected; the fastembed backend auto-detects the dimension from `EMBEDDINGS_MODEL`.

## Structure

```
crates/lancedb/
├── src/
│   ├── lib.rs           # Library entry, init_project_actor()
│   ├── config.rs        # Configuration from environment variables
│   ├── helpers.rs       # table_schema() + build_arrow_record()
│   ├── store.rs         # LanceStore (MemoryStore impl): hybrid/vector queries, hash dedup, FTS index, stats
│   ├── http/            # REST routes, dashboard, snapshots (static assets baked in via include_str!)
├── Cargo.toml
└── README.md
```

The embedding actor and MCP handlers are in the shared `piim-common` crate.

### Module map

- `src/lib.rs` — `init_project_actor()`: opens/creates the `project_memory` table (+ fail-fast check for the `project` column on old tables), spawns actors, starts HTTP dashboard.
- `src/config.rs` — `Config::get_from_env()` with defaults above.
- `src/store.rs` — `LanceStore` (`MemoryStore` impl): hybrid/vector queries, hash-dedup precheck, FTS index creation, column-projection stats.
- `src/helpers.rs` — `table_schema()` + `build_arrow_record()` (validates `vector.len() == VECTOR_DIMENSION`).
- `src/http/` — REST routes, dashboard, snapshots (static assets baked in via `include_str!`).

## License

MIT
