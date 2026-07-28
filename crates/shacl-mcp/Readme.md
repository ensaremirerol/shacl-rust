# shacl-mcp

MCP (Model Context Protocol) server for SHACL validation.

## Overview

This crate provides an MCP server that exposes SHACL validation functionality through the Model Context Protocol. It allows AI assistants and other MCP clients to validate RDF data against SHACL shapes, parse shapes, and lint RDF graphs.

## Features

The server exposes the following tools. Parameter names are snake_case, matching the JSON sent over the wire (not the camelCase shown in older versions of this doc).

### Supplying graphs: inline, by path, or merged from several files

Every tool that takes a data or shapes graph accepts the content two ways, and you can mix them freely across calls:

- **Inline**: `data_graph`/`shapes_graph` as a string, with `data_format`/`shapes_format` required (there's no filename to infer it from).
- **By path**: `data_path`/`shapes_path` pointing at a file on disk the server process can read. `data_format`/`shapes_format` is optional here — it's inferred from the file extension if omitted.
- **Shapes only, merged from several sources**: `shapes_graphs` (array of inline strings) and/or `shapes_paths` (array of file paths) — useful when shapes are split across a base vocabulary and project-specific extensions. All given shapes sources (`shapes_graph`, `shapes_path`, `shapes_graphs`, `shapes_paths`) are parsed and merged into one shapes graph before validation.

`data_graph`/`data_path` are mutually exclusive, as are `shapes_graph`/`shapes_path` (either can still combine with `shapes_graphs`/`shapes_paths`). Passing neither, or both, is a clear error rather than silently picking one.

### validate_graphs

Validate RDF data against SHACL shapes and return a validation report.

**Parameters:**
- `data_graph` / `data_path`, `data_format` (see above)
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format` (see above)
- `output_format`: Format of the output report ('text', 'json', or RDF format like 'ttl')

**Returns:** Validation report in the specified format

### validate_graphs_conforms

Check if RDF data conforms to SHACL shapes (returns only boolean result).

**Parameters:**
- `data_graph` / `data_path`, `data_format`
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format`

**Returns:** `{ "conforms": true/false }`

### lint_graph

Validate RDF graph syntax.

**Parameters:**
- `graph` / `graph_path`: RDF graph, inline or by file path
- `format`: Format of the graph (e.g., 'ttl', 'nt', 'jsonld'); inferred from `graph_path`'s extension if omitted

**Returns:** `{ "valid": true }` or error

### parse_shapes_graph

Parse SHACL shapes graph and return parsed shape information.

**Parameters:**
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format`

**Returns:** Parsed shapes metadata including shape count and details

### validate_diagnostics

Validate RDF data against SHACL shapes and return rich rustc-style diagnostics: shape lint findings plus per-violation diagnostics, each carrying a title, human-readable help text, annotated Turtle snippets from the data/shapes graphs, and (on the first occurrence of a given diagnostic code in the response) a `reference` object with the code's spec link and a failing/fixed Turtle example pair.

**Parameters:**
- `data_graph` / `data_path`, `data_format`
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format`
- `skip_lint` (default `false`): skip the 12 semantic shape-lint rules and only return validation diagnostics

**Returns:** `{ "summary": { "conforms", "violation_count", "errors", "warnings", "info", "diagnostic_count" }, "diagnostics": [...] }` — check `summary` first to short-circuit when nothing fired.

### lint_shacl_shapes

Run the 12 semantic shape-lint rules against a SHACL shapes graph, independent of any data graph — catches structural problems (e.g. a malformed shape) before spending a `validate_diagnostics` call on it.

**Parameters:**
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format`

**Returns:** `{ "summary": { "errors", "warnings", "info", "diagnostic_count" }, "diagnostics": [...] }`

### explain_diagnostic_code

Look up a diagnostic code (e.g. `'V0007'`) in the registry and return its title, spec reference, explanation, and a failing/fixed Turtle example pair. Rarely needed standalone — `validate_diagnostics` and `lint_shacl_shapes` already embed the same information under a `reference` key on each code's first occurrence. Reach for this when you need a code's explanation *before* it's actually triggered, e.g. while writing shapes.

**Parameters:**
- `code`: A diagnostic code, e.g. `'V0007'` or `'L0003'`

**Returns:** `{ "code", "title", "component", "spec_ref", "explanation", "failing_example", "fixed_example" }`

### why_conformance

Trace why a focus node does or does not conform to SHACL shapes, constraint by constraint. Use this specifically when a shape *should* have fired for a node but `validate_diagnostics` came back empty (or conforms unexpectedly) — it reports every applicable shape/constraint's verdict (`conforms`/`violates`/`not-targeted`/`vacuous`) for that node, which pinpoints things `validate_diagnostics` can't show on its own: a target that silently didn't match, a constraint that silently short-circuited, or a query that silently returned no rows.

**Parameters:**
- `data_graph` / `data_path`, `data_format`
- `shapes_graph` / `shapes_path` / `shapes_graphs` / `shapes_paths`, `shapes_format`
- `focus_node`: IRI of the focus node to trace (angle brackets optional)
- `shape` (optional): IRI of a single shape to restrict the trace to

**Returns:** Array of per-constraint diagnostics for the focus node

## Installation

### Building from Source

Build the MCP server:

```bash
cargo build --release -p shacl-mcp
```

The binary will be located at `target/release/shacl-mcp`.

### Installation via Cargo

You can also install the MCP server globally using Cargo:

```bash
cargo install shacl-mcp
```

## Usage

The MCP server communicates via JSON-RPC 2.0 over stdin/stdout. It's designed to be used by MCP clients such as Claude Desktop or other AI assistants.
