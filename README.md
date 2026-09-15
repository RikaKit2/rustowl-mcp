# rustowl-mcp

High-performance Model Context Protocol (MCP) server for Rust that brings **RustOwl** lifetime, ownership, and borrow-checker inspection to AI agents (OMP, Claude, Zed ACP, Cursor).

## Features

- **`rustowl_inspect_cursor`**: Pinpoint variable lifetime spans, active immutable/mutable borrows, moves, and borrow conflicts at specific `(file, line, col)` coordinates.
- **`rustowl_inspect_line`**: Scan an entire line of Rust code and return lifetime/borrow diagnostics for all detected variable identifiers.
- **Actionable AI Recommendations**: Normalizes raw RustOwl decorations into clean, structured JSON with suggestions on how to resolve conflicts (narrowing scopes with `{ ... }`, splitting structs, avoiding unnecessary `.clone()`).
- **Resilient Fallback**: Seamlessly spawns and communicates with `cargo-owlsp` LSP daemon, falling back to local AST-level inspection if the daemon is offline.
- **Declarative Nix Flake**: Full Nix support for reproducible builds and NixOS integration.

## MCP Tools Exposed

### 1. `rustowl_inspect_cursor`
Inspect variable lifetime and borrowing patterns at a specific cursor position:
```json
{
  "path": "src/main.rs",
  "line": 15,
  "col": 8
}
```

Example response:
```json
{
  "file": "src/main.rs",
  "target": { "line": 15, "col": 8 },
  "live_span": {
    "start_line": 10,
    "end_line": 28,
    "certainty": "definitely_live"
  },
  "active_borrows": [
    { "kind": "immutable", "lines": [12, 14] },
    { "kind": "mutable", "lines": [18, 24] }
  ],
  "moves": [ { "line": 26, "kind": "move" } ],
  "conflicts": [],
  "summary": "Clean lifetime with 2 active borrow(s). No borrow-checker conflicts."
}
```

### 2. `rustowl_inspect_line`
Inspect all variables on a given line:
```json
{
  "path": "src/main.rs",
  "line": 15
}
```

## Installation & Building

### Via Cargo
```bash
cargo build --release
# Binary available at target/release/rustowl-mcp
```

### Via Nix Flake
```bash
nix build
# or run directly:
nix run . -- --help
```

## Configuration for AI Clients

### Oh My Pi (OMP)
Add to your OMP MCP configuration (`~/.omp/agent/config.json` or project MCP config):
```json
{
  "mcpServers": {
    "rustowl": {
      "command": "/home/user/projects/rustowl-mcp/target/release/rustowl-mcp",
      "args": ["--stdio"]
    }
  }
}
```

### Claude Desktop / Zed ACP
```json
{
  "mcpServers": {
    "rustowl": {
      "command": "rustowl-mcp",
      "args": ["--stdio"]
    }
  }
}
```

## CLI Usage

You can test inspection directly in your terminal:
```bash
rustowl-mcp inspect --file src/main.rs --line 15 --col 8
```
