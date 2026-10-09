# Instructions for Using PIIM - Project Info in MCP

You have access to an ultra-fast, local vector database (LanceDB) powered by Kameo actors. 
Use it strategically to minimize context window usage and speed up inference according to the following rules:

### 1. When to Save Information (`upsert_project_info`):
*   **Task Completion:** After successfully completing a major task, record a brief summary of what was changed and why.
*   **Architectural Decisions:** When discovering or defining core architectural decisions (ADRs), API contracts, or project code style guidelines.
*   **Module Overviews:** When analyzing an unfamiliar module, save a brief summary explaining its purpose and responsibilities.
*   **Naming Convention (`info_id`):** IDs are unique per project (dedup key is `(info_id, project)`), so the same path/id can exist in many projects. Use snake_case conceptual ids for notes (e.g., `jwt_validation_logic`); reserve relative file paths for `save_file_description` records (see section 3).
*   **Project scope (`project`, optional):** omitting it uses the server's `PROJECT_NAME` default. Always pass the current project name explicitly when it differs from the server default, so records land in the right project.

### 2. How to Categorize Data (`category`):
*   `architecture` — Database schemas, API contracts, module dependencies, and ADRs.
*   `code_contract` — Interfaces, core structures, data types, and exchange protocols.
*   `file` — One record per project file: a short description of the file's content (written by `save_file_description`, `info_id` = relative file path).
*   `function` — One record per function/method: a short description of what it does (written by `save_function_description`, `info_id` = `<file_path>::<function_name>` or `<file_path>::<struct_name>::<function_name>`).
*   `todo` — Technical debt, planned features, and discovered bugs.
*   `changelog` — A historical log of tasks you have completed (what was changed, how, and why).

### 3. How to Save All Files Info (`save_file_description`):
*   **When:** on first onboarding to a project, and whenever refactoring changes a file's purpose or public symbols.
*   **How:** walk the project's source tree and call `save_file_description` once per file:
    *   `file_path` — relative path from the project root; it is the unique id *within the project* (e.g., `src/auth.rs`). Saving the same path again in the same project overwrites its previous description (SHA-256 dedup skips embedding when nothing changed).
    *   `description` — 1–3 sentences: what the file does, which key functions/structs/traits it defines, and what it depends on.
    *   `project` (optional) — project scope; defaults to the server's `PROJECT_NAME`. Pass the current project name explicitly when working outside the default project.
*   **Scope:** store only real project sources; skip generated and vendored trees (`target/`, `node_modules/`, `dist/`, lock files).
*   Keep descriptions concise — they are embedded on every change.

### 4. How to Save Function Info (`save_function_description`):
*   **When:** after onboarding file descriptions, for key public functions/methods whose behavior you would otherwise have to re-read; or whenever a function's signature or behavior changes.
*   **How:** call `save_function_description` once per function:
    *   `file_path` — relative path from the project root (e.g., `src/auth.rs`).
    *   `function_name` — function/method name (e.g., `validate_jwt`).
    *   `struct_name` (optional) — struct/impl name for associated functions/methods (e.g., `AuthService`); omit for free functions.
    *   `description` — 1–3 sentences: what the function does, its parameters/return value, side effects, and how it is used.
    *   `project` (optional) — project scope; defaults to the server's `PROJECT_NAME`.
*   **Unique id:** the tool builds `<file_path>::<function_name>` (e.g., `src/auth.rs::validate_jwt`) or `<file_path>::<struct_name>::<function_name>` (e.g., `src/auth.rs::AuthService::validate`) under the `function` category, scoped per project. Re-saving the same triple in the same project overwrites the previous record (SHA-256 dedup skips embedding when unchanged).
*   **Scope:** only functions whose behavior matters for future work; keep descriptions concise — they are embedded on every change.

### 5. How to Search Function/Type Usage in the Project:
*   To find where a function, type, or constant is used, call `hybrid_search_memory` with:
    *   `query` — the exact symbol name (e.g., `validate_jwt`), optionally with context ("who calls validate_jwt").
    *   `category` — `"function"` for exact function descriptions, `"file"` for candidate files (omit to search both plus everything else).
    *   `limit` — how many candidate files to inspect (e.g., `10`).
    *   `project` (optional) — project scope; defaults to `PROJECT_NAME`. Pass `"*"` only when the user explicitly asks to search across all projects.
*   The reply lists matches as `[project:category] id (distance)` followed by the stored description: file hits show the file path, function hits show `<file_path>::<function>` ids. Treat the top hits as candidates: open those files and grep for the symbol to confirm exact usages.
*   Repeat the search with `category` `architecture` or `code_contract` when you need contracts and ADRs instead of files/functions.

### 6. When to Search Memory (`hybrid_search_memory`):
*   **Context Retrieval:** At the beginning of a session if the user asks you to modify or work on a module whose contents are missing from your current context window.
*   **Constraint Checking:** When you need to recall architectural agreements (always apply the filter `category: "architecture"` to narrow down results).
*   **Exact Matching:** When looking for precise function names, variables, type definitions, or specific error-handling logic.

### 7. How to Watch Memory Usage (`memory_stats`):
*   Call `memory_stats` with optional `project` (defaults to `PROJECT_NAME`; `"*"` aggregates all projects with a per-project breakdown) to see record count, per-category breakdown, and content size (total/avg chars). It runs no embedding inference, so it is cheap — use it at session start to gauge what's stored, before a file-catalog walk to avoid re-saving, or when deciding whether to run `optimize_database` / prune stale records.

### 8. Multi-Project Rules:
*   One database holds many projects — every record is tagged with `project`. Omitting `project` uses the server default (`PROJECT_NAME`); reads/writes never cross projects unless you pass `"*"` (search/stats only).
*   Always pass the current project name explicitly when it differs from the server default. Use the repository/project name you are working in — never invent project names.
*   Use `project: "*"` only for cross-project search/stats on explicit user request; the reply shows each hit's project as `[project:category]`.

Optimize local GPU resources: Do not perform repetitive upserts if you know the file or concept contents have not changed.
