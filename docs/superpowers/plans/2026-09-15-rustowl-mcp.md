# rustowl-mcp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a high-performance Model Context Protocol (MCP) server in Rust that interfaces with RustOwl (`cargo-owlsp` / `rustowl`) to provide AI agents with structured inspection of Rust variable lifetimes, borrow ranges, mutations, moves, and borrow-checker conflicts.

**Architecture:** A standalone native binary (`rustowl-mcp`) communicating over stdio JSON-RPC following the MCP 2024-11-05 protocol. The server manages an internal persistent `cargo owlsp` LSP child process per target Cargo workspace, communicates via JSON-RPC (`rustowl/cursor`), and normalizes RustOwl decorations into structured, token-efficient JSON reports for LLMs. Includes a Nix Flake for declarative builds.

**Tech Stack:** Rust (Edition 2021), `tokio` (async runtime & process management), `serde` / `serde_json` (JSON serialization/deserialization), `clap` (CLI args), `anyhow` / `thiserror` (error handling).

**Spec:** In-line architectural design established in Spike findings:
1. Wrap `cargo owlsp` LSP daemon via stdio.
2. Expose MCP tools:
   - `rustowl_inspect_cursor`: query variable lifetime, immutable/mutable borrows, moves, and conflicts at `(path, line, col)`.
   - `rustowl_inspect_line`: scan identifiers on a line and return annotations across all variables on that line.
   - `rustowl_file_overview`: collect all borrow conflicts and active loan boundaries in a file.
3. Package with Nix Flake (`flake.nix`) for seamless NixOS declarative integration.

## Global Constraints

- Must compile cleanly with stable Rust (`cargo build --release`).
- Strictly adhere to standard MCP stdio protocol format (`Content-Length` or newline-delimited JSON-RPC).
- Zero panics on malformed inputs or missing compiler tools; return clear structured error responses.
- Output JSON must be concise and free of raw terminal escape codes or visual formatting noise.

---

### Task 1: Project Scaffolding and Flake Setup

**Files:**
- Create: `/home/user/projects/rustowl-mcp/Cargo.toml`
- Create: `/home/user/projects/rustowl-mcp/flake.nix`
- Create: `/home/user/projects/rustowl-mcp/.gitignore`
- Create: `/home/user/projects/rustowl-mcp/src/main.rs`

**Interfaces:**
- Produces: Base Cargo workspace and Nix development environment with dependencies:
  - `tokio` (full)
  - `serde`, `serde_json` (derive)
  - `clap` (derive)
  - `anyhow`, `thiserror`

- [ ] **Step 1: Create `.gitignore`**
- [ ] **Step 2: Create `Cargo.toml` with dependencies**
- [ ] **Step 3: Create skeleton `src/main.rs`**
- [ ] **Step 4: Create `flake.nix` supporting `nix build` and `nix develop`**
- [ ] **Step 5: Verify build with `cargo check`**

---

### Task 2: MCP Protocol Models & Dispatcher

**Files:**
- Create: `/home/user/projects/rustowl-mcp/src/mcp/types.rs`
- Create: `/home/user/projects/rustowl-mcp/src/mcp/server.rs`
- Test: `/home/user/projects/rustowl-mcp/tests/mcp_protocol_test.rs`

**Interfaces:**
- Produces:
  - `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`
  - `McpTool`, `McpToolCall`, `McpToolResult`
  - `McpServer::handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse`

- [ ] **Step 1: Write tests for MCP `initialize`, `tools/list`, and `tools/call` parsing**
- [ ] **Step 2: Implement protocol types (`types.rs`) adhering to MCP 2024-11-05 spec**
- [ ] **Step 3: Implement `McpServer` request handler and tool registry (`server.rs`)**
- [ ] **Step 4: Run tests and ensure 100% pass**

---

### Task 3: RustOwl LSP Client Adapter (`cargo owlsp` bridge)

**Files:**
- Create: `/home/user/projects/rustowl-mcp/src/owl/protocol.rs`
- Create: `/home/user/projects/rustowl-mcp/src/owl/client.rs`
- Test: `/home/user/projects/rustowl-mcp/tests/owl_client_test.rs`

**Interfaces:**
- Produces:
  - `RustOwlDecoration`: `{ range: Range, kind: DecorationKind }`
  - `DecorationKind`: `DefinitelyLive`, `MaybeInitialized`, `ImmBorrow`, `MutBorrow`, `Move`, `Call`, `Outlive`, `SharedMut`
  - `RustOwlClient`:
    - `spawn(workspace_root: &Path) -> Result<Self>`
    - `cursor(&mut self, file: &Path, line: u32, col: u32) -> Result<Vec<RustOwlDecoration>>`

- [ ] **Step 1: Write serialization/deserialization tests for `rustowl/cursor` request and response**
- [ ] **Step 2: Implement `RustOwlDecoration` and `DecorationKind` parsing**
- [ ] **Step 3: Implement `RustOwlClient` managing background process with stdio JSON-RPC**
- [ ] **Step 4: Add fallback mock/offline mode for testing without global `cargo-owlsp`**
- [ ] **Step 5: Run tests and ensure clean passage**

---

### Task 4: High-Level Analysis Engine & Semantic Normalizer

**Files:**
- Create: `/home/user/projects/rustowl-mcp/src/analysis/mod.rs`
- Create: `/home/user/projects/rustowl-mcp/src/analysis/report.rs`
- Test: `/home/user/projects/rustowl-mcp/tests/analysis_report_test.rs`

**Interfaces:**
- Consumes: `Vec<RustOwlDecoration>` from Task 3
- Produces:
  - `VariableLifetimeReport`:
    - `live_span`: `{ start_line, end_line, certainty }`
    - `active_borrows`: `Vec<{ kind: "immutable" | "mutable", lines: [start, end] }>`
    - `moves`: `Vec<{ line: u32 }>`
    - `conflicts`: `Vec<{ kind: String, lines: [start, end], advice: String }>`
  - `normalize_decorations(decorations: &[RustOwlDecoration], target_line: u32) -> VariableLifetimeReport`

- [ ] **Step 1: Write unit tests verifying transformation of raw decorations into high-level reports**
- [ ] **Step 2: Implement conflict detection heuristics (e.g. `outlive` → actionable hint)**
- [ ] **Step 3: Implement line-scanning helper to auto-detect variable columns on a given line**
- [ ] **Step 4: Run tests and verify normalization logic**

---

### Task 5: Integration of Tools into MCP Server & CLI Runner

**Files:**
- Modify: `/home/user/projects/rustowl-mcp/src/main.rs`
- Modify: `/home/user/projects/rustowl-mcp/src/mcp/server.rs`
- Test: `/home/user/projects/rustowl-mcp/tests/e2e_mcp_test.rs`

**Interfaces:**
- Exposes tools via MCP:
  - `rustowl_inspect_cursor`: `{ path: string, line: number, col: number }`
  - `rustowl_inspect_line`: `{ path: string, line: number, variable?: string }`
- CLI Flags:
  - `rustowl-mcp --stdio`: run MCP server over stdio
  - `rustowl-mcp inspect --file <path> --line <n> --col <n>`: direct CLI query for testing

- [ ] **Step 1: Wire MCP tools to `RustOwlClient` and `AnalysisEngine`**
- [ ] **Step 2: Implement stdio main loop handling stdin/stdout streaming**
- [ ] **Step 3: Implement direct CLI inspect command for manual verification**
- [ ] **Step 4: Write end-to-end integration test simulating an AI agent calling the MCP tools**
- [ ] **Step 5: Run tests and verify full pipeline**

---

### Task 6: NixOS Integration & Agent Rules Update

**Files:**
- Update: `/home/user/nixos-config/flake.nix` (add `rustowl-mcp` input if desired)
- Update: `/home/user/.omp/agent/AGENTS.md` (add Rust borrow-checking rules)
- Create: `/home/user/projects/rustowl-mcp/README.md` (installation & MCP client configuration)

**Interfaces:**
- Produces:
  - Ready-to-use declaration in NixOS and OMP
  - Rules ensuring the agent uses `rustowl-mcp` on borrow checker errors

- [ ] **Step 1: Write `README.md` with configuration instructions for OMP, Claude, and Zed**
- [ ] **Step 2: Add borrow checker analysis rule to `~/.omp/agent/AGENTS.md`**
- [ ] **Step 3: Verify Nix build of `rustowl-mcp` with `nix build` or `cargo build --release`**
