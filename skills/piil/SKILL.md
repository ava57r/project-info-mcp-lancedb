---
name: piil
description: This skill provides the AI agent with a high-performance, persistent long-term memory system optimized for local code repositories, enabling lightning-fast hybrid retrieval while conserving context window tokens and local compute resources
---

# Project info in LanceDb 

## Available Tools

### 1. `upsert_project_info`
*   **Purpose:** Securely inserts or updates an atomic piece of project context, documentation, or code contracts.
*   **Parameters:**
    *   `info_id` (string): Unique identifier — snake_case for concepts (e.g., `jwt_validation_logic`). Relative file paths are reserved for `save_file_description`, because ids are unique across the whole table.
    *   `content` (string): The distilled code snippet, architectural summary, or text block to remember.
    *   `category` (string): Strict isolation scope. Must be one of: `architecture`, `file`, `code_contract`, `todo`, `changelog`.
*   **Optimization Note:** The underlying Rust actor computes a SHA-256 hash before executing. If the data is identical, the GPU text embedding inference is skipped automatically.

### 2. `save_file_description`
*   **Purpose:** Stores a short description of a file's content in memory; the relative file path is the unique id (stored under the `file` category).
*   **Parameters:**
    *   `file_path` (string): Unique key — relative path to the file (e.g., `src/auth.rs`). Re-saving the same path overwrites the previous description.
    *   `description` (string): 1–3 sentences: file purpose, key functions/types it defines, how it is used.
*   **Optimization Note:** Same SHA-256 dedup as `upsert_project_info` — unchanged descriptions skip embedding.

### 3. `hybrid_search_memory`
*   **Purpose:** Executes an ultra-fast hybrid search combining dense semantic vectors and exact keyword matches (BM25) over the stored project repository knowledge.
*   **Parameters:**
    *   `query` (string): Natural language query or exact function/variable name.
    *   `category` (string, optional): Filters the search strictly to a specific metadata category to narrow scope.
    *   `limit` (integer): Maximum number of records to return (e.g., 10).

### 4. `optimize_database`
*   **Purpose:** Triggers file compaction, merges small Arrow record batches, and garbage-collects historical timeline versions within the LanceDB table to optimize disk I/O and maintain low-latency lookups.

### 5. `memory_stats`
*   **Purpose:** Reports memory usage statistics — total record count, per-category breakdown, and content size (total/avg chars) — so you can watch how full the memory is without reading every record. Takes no arguments (call with `{}`).
*   **When to use:** at session start to gauge memory size, before a big file-catalog walk to see what's already stored, or when deciding whether to compact (`optimize_database`) or prune stale records.

## Operational Rules & Behavioral Guidelines

### 1. Token Economy (Proactive Context Offloading)
*   **Do Not Feed Entire Files Repeatedly:** Instead of keeping large tracking files, markdown schemas, or structural indices constantly inside your active system prompt, offload them using `upsert_project_info`.
*   **On-Demand Retrieval:** When starting a task in an area of the codebase not currently visible in your workspace context, call `hybrid_search_memory` first to fetch only relevant definitions.

### 2. Categorization Protocol
*   Categorize data with precision to maintain efficient SQL metadata filtering on the LanceDB engine:
    *   Use `file` for per-file content descriptions written via `save_file_description` (id = relative file path).
    *   Use `architecture` for configuration formats, core dependencies, API endpoint signatures, and ADRs.
    *   Use `code_contract` for internal types, interfaces, traits, and shared state structures.
    *   Use `todo` to capture structural bugs, tech debt, and immediate feature requirements.
    *   Use `changelog` to summarize your own work at the end of a session (files modified, logic added, and architectural impacts).

### 3. How to Save All Files Info (`save_file_description`)
*   **When (if needed):** on first onboarding to an unfamiliar project, or after a refactor changes a file's purpose or public symbols. Skip it when the files are already inside your context window — store only what you would otherwise have to re-read later.
*   **How:** walk the project's source tree and call `save_file_description` once per source file:
    *   `file_path` — relative path from the project root; it is the unique id (e.g., `src/auth.rs`). Re-saving the same path overwrites its previous description.
    *   `description` — 1–3 sentences: what the file does, which key functions/structs/traits it defines, and what it depends on.
*   **Scope:** only real project sources; skip generated and vendored trees (`target/`, `node_modules/`, `dist/`, lock files). Keep descriptions concise — every changed description costs one embedding call (unchanged ones are skipped via SHA-256 dedup).

### 4. How to Search Function/Type Usage in the Project
*   Call `hybrid_search_memory` with:
    *   `query` — the exact symbol name (e.g., `validate_jwt`), optionally with context ("who calls validate_jwt").
    *   `category` — `"file"` to search only the stored file descriptions.
    *   `limit` — how many candidate files to inspect (e.g., `10`).
*   The reply lists matches as `[category] file_path (distance)` followed by the stored description. Treat the top hits as candidates: open those files and grep for the symbol to confirm exact usages.
*   If you need types and contracts instead of files, repeat with `category` `code_contract` or `architecture`; if the file catalog is empty, fall back to ripgrep over the repository.

### 5. Precision Token Matching
*   When a user asks about specific system internals (e.g., *"Where do we validate JWT tokens?"*), do not guess. Invoke `hybrid_search_memory` with the method name or keyword. The hybrid FTS (Full-Text Search) engine will locate exact lexical matches, while the vector engine fetches surrounding semantic contexts.
*   To find usages of a function, type, or constant across the project, call `hybrid_search_memory` with `category: "file"` and the symbol name as `query`: the reply lists matching file paths with their stored descriptions — open the top hits and grep for the symbol to confirm exact usages.

### 6. How to Watch Memory Usage (`memory_stats`)
*   Call `memory_stats` (no arguments) to see total record count, per-category breakdown, and content size (total/avg chars) — no embedding inference, so it is cheap.
*   **When:** at session start to gauge what's already stored, before a file-catalog walk to avoid re-saving, or when deciding whether to run `optimize_database` / prune stale records.
