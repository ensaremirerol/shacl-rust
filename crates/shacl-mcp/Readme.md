# shacl-mcp

MCP (Model Context Protocol) server for SHACL validation.

## Overview

This crate provides an MCP server that exposes SHACL validation functionality through the Model Context Protocol. It allows AI assistants and other MCP clients to validate RDF data against SHACL shapes, parse shapes, and lint RDF graphs.

## Features

The server exposes the following tools:

### validate_graphs

Validate RDF data against SHACL shapes and return a validation report.

**Parameters:**
- `dataGraph`: RDF data graph as a string
- `shapesGraph`: SHACL shapes graph as a string
- `dataFormat`: Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')
- `shapesFormat`: Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')
- `outputFormat`: Format of the output report ('text', 'json', or RDF format like 'ttl')

**Returns:** Validation report in the specified format

### validate_graphs_conforms

Check if RDF data conforms to SHACL shapes (returns only boolean result).

**Parameters:**
- `dataGraph`: RDF data graph as a string
- `shapesGraph`: SHACL shapes graph as a string
- `dataFormat`: Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')
- `shapesFormat`: Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')

**Returns:** `{ "conforms": true/false }`

### lint_graph

Validate RDF graph syntax.

**Parameters:**
- `graph`: RDF graph as a string
- `format`: Format of the graph (e.g., 'ttl', 'nt', 'jsonld')

**Returns:** `{ "valid": true }` or error

### parse_shapes_graph

Parse SHACL shapes graph and return parsed shape information.

**Parameters:**
- `shapesGraph`: SHACL shapes graph as a string
- `shapesFormat`: Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')

**Returns:** Parsed shapes metadata including shape count and details

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
