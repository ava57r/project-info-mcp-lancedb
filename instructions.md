# Instructions for Using Long-Term Memory (LanceDB MCP)

You have access to an ultra-fast, local vector database (LanceDB) powered by Kameo actors. 
Use it strategically to minimize context window usage and speed up inference according to the following rules:

### 1. When to Save Information (`upsert_project_info`):
*   **Task Completion:** After successfully completing a major task, record a brief summary of what was changed and why.
*   **Architectural Decisions:** When discovering or defining core architectural decisions (ADRs), API contracts, or project code style guidelines.
*   **Module Overviews:** When analyzing an unfamiliar module, save a brief summary explaining its purpose and responsibilities.
*   **Naming Convention (`info_id`):** Use the relative file path for code files (e.g., `src/auth.rs`) and snake_case for conceptual notes (e.g., `jwt_validation_logic`).

### 2. How to Categorize Data (`category`):
*   `architecture` — Database schemas, API contracts, module dependencies, and ADRs.
*   `code_contract` — Interfaces, core structures, data types, and exchange protocols.
*   `todo` — Technical debt, planned features, and discovered bugs.
*   `changelog` — A historical log of tasks you have completed (what was changed, how, and why).

### 3. When to Search Memory (`hybrid_search_memory`):
*   **Context Retrieval:** At the beginning of a session if the user asks you to modify or work on a module whose contents are missing from your current context window.
*   **Constraint Checking:** When you need to recall architectural agreements (always apply the filter `category: "architecture"` to narrow down results).
*   **Exact Matching:** When looking for precise function names, variables, type definitions, or specific error-handling logic.

Optimize local Intel GPU resources: Do not perform repetitive upserts if you know the file or concept contents have not changed.
