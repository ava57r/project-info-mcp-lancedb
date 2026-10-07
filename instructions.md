# Instructions for Using PIIL - Project info in LanceDB MCP

You have access to an ultra-fast, local vector database (LanceDB) powered by Kameo actors. 
Use it strategically to minimize context window usage and speed up inference according to the following rules:

### 1. When to Save Information (`upsert_project_info`):
*   **Task Completion:** After successfully completing a major task, record a brief summary of what was changed and why.
*   **Architectural Decisions:** When discovering or defining core architectural decisions (ADRs), API contracts, or project code style guidelines.
*   **Module Overviews:** When analyzing an unfamiliar module, save a brief summary explaining its purpose and responsibilities.
*   **Naming Convention (`info_id`):** IDs are unique across the whole table, so a path can hold only one record. Use snake_case conceptual ids for notes (e.g., `jwt_validation_logic`); reserve relative file paths for `save_file_description` records (see section 3).

### 2. How to Categorize Data (`category`):
*   `architecture` — Database schemas, API contracts, module dependencies, and ADRs.
*   `code_contract` — Interfaces, core structures, data types, and exchange protocols.
*   `file` — One record per project file: a short description of the file's content (written by `save_file_description`, `info_id` = relative file path).
*   `todo` — Technical debt, planned features, and discovered bugs.
*   `changelog` — A historical log of tasks you have completed (what was changed, how, and why).

### 3. How to Save All Files Info (`save_file_description`):
*   **When:** on first onboarding to a project, and whenever refactoring changes a file's purpose or public symbols.
*   **How:** walk the project's source tree and call `save_file_description` once per file:
    *   `file_path` — relative path from the project root; it is the unique id (e.g., `src/auth.rs`). Saving the same path again overwrites its previous description (SHA-256 dedup skips embedding when nothing changed).
    *   `description` — 1–3 sentences: what the file does, which key functions/structs/traits it defines, and what it depends on.
*   **Scope:** store only real project sources; skip generated and vendored trees (`target/`, `node_modules/`, `dist/`, lock files).
*   Keep descriptions concise — they are embedded on every change.

### 4. How to Search Function/Type Usage in the Project:
*   To find where a function, type, or constant is used, call `hybrid_search_memory` with:
    *   `query` — the exact symbol name (e.g., `validate_jwt`), optionally with context ("who calls validate_jwt").
    *   `category` — `"file"` to search only the stored file descriptions.
    *   `limit` — how many candidate files to inspect (e.g., `10`).
*   The reply lists matches as `[category] file_path (distance)` followed by the stored description. Treat the top hits as candidates: open those files and grep for the symbol to confirm exact usages.
*   Repeat the search with `category` `architecture` or `code_contract` when you need contracts and ADRs instead of files.

### 5. When to Search Memory (`hybrid_search_memory`):
*   **Context Retrieval:** At the beginning of a session if the user asks you to modify or work on a module whose contents are missing from your current context window.
*   **Constraint Checking:** When you need to recall architectural agreements (always apply the filter `category: "architecture"` to narrow down results).
*   **Exact Matching:** When looking for precise function names, variables, type definitions, or specific error-handling logic.

Optimize local GPU resources: Do not perform repetitive upserts if you know the file or concept contents have not changed.
