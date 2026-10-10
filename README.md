# PIIM (Project Info In MCP)

A persistent project-memory MCP server in Rust. Stores discrete facts (architecture notes, TODOs, API contracts, changelogs) in a vector database with hybrid search, exposed to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io) over stdio.

This repository is a Cargo workspace with two storage backends sharing one core:

- **`piim-lance`** — embedded [LanceDB](https://lancedb.com) storage ([`crates/lancedb`](crates/lancedb), see its [README](crates/lancedb/README.md)) with ANN vector + full-text search, REST API, and dashboard.
- **`piim-qdrant`** — remote [Qdrant](https://qdrant.tech) storage ([`crates/qdrant`](crates/qdrant), see its [README](crates/qdrant/README.md)).

Both binaries are driven by **`piim-common`** ([`crates/common`](crates/common)): `kameo` actors (`ProjectInfoActor` + `EmbeddingActor`), the MCP tool layer, and the `MemoryStore` backend trait. Embeddings come from any OpenAI-compatible API (e.g. Qwen3-embed, TEI, vLLM).

## Features

- **Multi-project memory** — one database serves several projects: every record carries a `project` key (`id`, `project`, `content`, `vector`, `category`, `file_hash`, `timestamp`). The active project defaults to `PROJECT_NAME` (`default`) and can be overridden per tool call via the optional `project` argument (`"*"` searches/stats across all projects).
- **Persistent memory** — `project_memory` table/collection with the schema above (backend-specific creation details live in each crate README).
- **Hybrid search** — vector search over embeddings + full-text search on `content`, with optional `category` filter, always scoped to the active project (or `"*"` for all).
- **Content-aware upsert** — SHA-256 `file_hash` check skips embedding inference when content is unchanged; otherwise delete + re-insert. Dedup key is `(id, project)`, so the same file path can exist in many projects.
- **File catalog** — `save_file_description` stores one `file`-category record per source file *per project*; the `(file path, project)` pair is the unique id.
- **Pluggable storage backend** — the MCP surface is backed by the `MemoryStore` trait (`piim-common::store`); `crates/lancedb` provides `LanceStore` and `crates/qdrant` provides `QdrantStore`. Query construction, error messages, and maintenance ops (`optimize`, `reopen`) live in each implementation; the actor and MCP layers stay backend-agnostic.
- **Actor isolation** — `ProjectInfoActor` owns the storage backend; `EmbeddingActor` owns the HTTP client for the embeddings API.
- **Zero-config defaults** — works out of the box against `http://localhost:8002/v1/embeddings`.

## Prerequisites

- Rust 1.85+ (edition 2024 workspace; `cargo build`)
- A running OpenAI-compatible embeddings endpoint, e.g.:
  ```bash
  curl -X POST http://localhost:8002/v1/embeddings \
    -H 'Content-Type: application/json' \
    -d '{"input":"hello","model":"qwen3-embed","encoding_format":"float"}'
  ```

## Build & Run

```bash
cargo build --release                # whole workspace (piim-lance + piim-qdrant)
./target/release/piim-lance          # LanceDB backend (stdio MCP + HTTP dashboard)
./target/release/piim-qdrant         # Qdrant backend (stdio MCP)
```

Build a single binary:

```bash
cargo build --release -p piim-lance      # LanceDB only
cargo build --release -p piim-qdrant     # Qdrant only
```

Run `piim-lance` with custom config (see [crates/lancedb/README.md](crates/lancedb/README.md)):

```bash
LANCEDB_PATH=./.opencode_memory/lance_db \
EMBEDDINGS_URL=http://localhost:8002/v1/embeddings \
EMBEDDINGS_MODEL=qwen3-embed \
VECTOR_DIMENSION=1024 \
PROJECT_NAME=my-project \
./target/release/piim-lance
```

## Configuration

Shared env vars (used by both backends): `EMBEDDINGS_URL` (`http://localhost:8002/v1/embeddings`), `EMBEDDINGS_MODEL` (`qwen3-embed`), `VECTOR_DIMENSION` (`1024`), `PROJECT_NAME` (`default`). Backend-specific storage settings:

- `piim-lance`: `LANCEDB_PATH`, `HTTP_PORT`, `SNAPSHOT_DIR` — see [crates/lancedb/README.md](crates/lancedb/README.md).
- `piim-qdrant`: `QDRANT_URL`, `QDRANT_API_KEY`, `QDRANT_COLLECTION` — see [crates/qdrant/README.md](crates/qdrant/README.md).

## Client setup (opencode)

See the backend READMEs for copy-paste configs: [piim-lance](crates/lancedb/README.md#client-setup-opencode) and [piim-qdrant](crates/qdrant/README.md).

## Tools

Both backends expose the same 6 MCP tools (`upsert_project_info`, `save_file_description`, `save_function_description`, `hybrid_search_memory`, `optimize_database`, `memory_stats`). See [crates/lancedb/README.md](crates/lancedb/README.md#tools) for the argument table.

## Architecture

```text
┌───────────────────────  piim-common (crates/common)  ────────────────────────┐
│ stdin (JSON-RPC) → StdioTransport → MemoryToolHandler  (6 MCP tools)         │
│                                        │ tools/call                          │
│                                        ▼                                     │
│                          ProjectInfoActor (kameo)                            │
│                            ├─ generic over the MemoryStore trait             │
│                            └─ asks EmbeddingActor ──POST EMBEDDINGS_URL──▶   │
│                                                             embeddings API   │
└────────────────────────────────┬─────────────────────────────────────────────┘
                                 │ MemoryStore (crates/common/src/store.rs)
                 ┌───────────────┴───────────────┐
                 ▼                               ▼
   crates/lancedb  (piim-lance)         crates/qdrant  (piim-qdrant)
   LanceStore: table open/create,     QdrantStore: client, points & filters,
   Arrow records, hybrid query,       hybrid/structured search, counts,
   hash dedup, FTS index, optimize,   payload stats, optimize, collection
   snapshots, REST API + dashboard    creation
```

| Crate | Package | Role |
|---|---|---|
| [`crates/common`](crates/common) | `piim-common` | Shared core: `MemoryStore` trait, `CommonConfig` (shared env parsing), `ProjectInfoActor` + all message handlers, `EmbeddingActor`, MCP server + tool definitions, search/stats text formatting. |
| [`crates/lancedb`](crates/lancedb) | `piim-lance` | LanceDB backend binary (see its [README](crates/lancedb/README.md)). |
| [`crates/qdrant`](crates/qdrant) | `piim-qdrant` | Qdrant backend binary. |

Backend-specific behavior — query construction, error messages, `optimize`/`reopen` wording, the empty-stats noun ("table" vs "collection") — lives entirely inside each `MemoryStore` implementation, so adding a backend means implementing the trait plus a thin `main.rs`/`config.rs`.

### `piim-lance` (LanceDB) module map

- `crates/lancedb/src/main.rs` — stdio wiring, table open/create (+ fail-fast check for the `project` column on old tables), actor spawn, MCP `Server::start()`, HTTP dashboard.
- `crates/lancedb/src/config.rs` — `Config::get_from_env()` with defaults (see [crates/lancedb/README.md](crates/lancedb/README.md)).
- `crates/lancedb/src/store.rs` — `LanceStore` (`MemoryStore` impl): hybrid/vector queries, hash-dedup precheck, FTS index creation, column-projection stats.
- `crates/lancedb/src/helpers.rs` — `table_schema()` + `build_arrow_record()` (validates `vector.len() == VECTOR_DIMENSION`).
- `crates/lancedb/src/http/` — REST routes, dashboard, snapshots (static assets baked in via `include_str!`).
- `crates/common/src/actors/` — `EmbeddingActor` and the backend-agnostic `ProjectInfoActor` handlers.

## Development

```bash
cargo fmt --all             # format all workspace members
cargo clippy --all-targets  # lint (must be clean)
cargo test --all            # tests across the workspace
```
