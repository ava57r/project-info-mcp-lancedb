---
name: piil
description: This skill provides the AI agent with a high-performance, persistent long-term memory system optimized for local code repositories, enabling lightning-fast hybrid retrieval while conserving context window tokens and local compute resources
---

# Project info in LanceDb 

## Available Tools

### 1. `upsert_project_info`
*   **Purpose:** Securely inserts or updates an atomic piece of project context, documentation, or code contracts.
*   **Parameters:**
    *   `info_id` (string): Unique identifier. Use relative paths for files (e.g., `src/auth.rs`) or snake_case for concepts (e.g., `jwt_validation_logic`).
    *   `content` (string): The distilled code snippet, architectural summary, or text block to remember.
    *   `category` (string): Strict isolation scope. Must be one of: `architecture`, `code_contract`, `todo`, `changelog`.
*   **Optimization Note:** The underlying Rust actor computes a SHA-256 hash before executing. If the data is identical, the GPU text embedding inference is skipped automatically.

### 2. `hybrid_search_memory`
*   **Purpose:** Executes an ultra-fast hybrid search combining dense semantic vectors and exact keyword matches (BM25) over the stored project repository knowledge.
*   **Parameters:**
    *   `query` (string): Natural language query or exact function/variable name.
    *   `category` (string, optional): Filters the search strictly to a specific metadata category to narrow scope.
    *   `limit` (integer, optional): Maximum number of relevant Arrow record chunks to return (defaults to 3).

### 3. `optimize_database`
*   **Purpose:** Triggers file compaction, merges small Arrow record batches, and garbage-collects historical timeline versions within the LanceDB table to optimize disk I/O and maintain low-latency lookups.

## Operational Rules & Behavioral Guidelines

### 1. Token Economy (Proactive Context Offloading)
*   **Do Not Feed Entire Files Repeatedly:** Instead of keeping large tracking files, markdown schemas, or structural indices constantly inside your active system prompt, offload them using `upsert_project_info`.
*   **On-Demand Retrieval:** When starting a task in an area of the codebase not currently visible in your workspace context, call `hybrid_search_memory` first to fetch only relevant definitions.

### 2. Categorization Protocol
*   Categorize data with precision to maintain efficient SQL metadata filtering on the LanceDB engine:
    *   Use `architecture` for configuration formats, core dependencies, API endpoint signatures, and ADRs.
    *   Use `code_contract` for internal types, interfaces, traits, and shared state structures.
    *   Use `todo` to capture structural bugs, tech debt, and immediate feature requirements.
    *   Use `changelog` to summarize your own work at the end of a session (files modified, logic added, and architectural impacts).

### 3. Precision Token Matching
*   When a user asks about specific system internals (e.g., *"Where do we validate JWT tokens?"*), do not guess. Invoke `hybrid_search_memory` with the method name or keyword. The hybrid FTS (Full-Text Search) engine will locate exact lexical matches, while the vector engine fetches surrounding semantic contexts.
