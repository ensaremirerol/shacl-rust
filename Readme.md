# SHACL Rust

This repository contains a Rust implementation of the SHACL (Shapes Constraint Language) specification.

## How to Use

- **Library**: Use the `shacl` crate in your Rust projects for SHACL validation and processing.
- **WASM Bindings**: The `shacl-wasm` crate provides WebAssembly bindings for use in web applications and npm packages.
- **Command-Line Tool**: The `shacl-cli` crate offers a CLI for validating RDF data against SHACL shapes. You can install it with `cargo install shacl-cli`. You can then use it like this:

To parse the shapes graph:

```bash
shacl-validator parse shapes.ttl
```

To validate a data graph against the shapes:

```bash
shacl-validator validate shapes.ttl data.ttl ... # First file is shapes, rest are data graphs. Data graphs are merged and validated together.
```

You can use `shacl-validator --help` for general usage information and `shacl-validator <command> --help` for command-specific options.

## Diagnostics

`shacl-validator validate` prints rustc-style diagnostics to **stderr** by
default: shape lints run first, then every violation gets a compact,
annotated rendering with a data-graph snippet, a shapes-graph snippet, and
expected/actual/help lines. The classic validation report keeps going to
**stdout** unchanged, so piping stdout to a file or another tool is
unaffected.

```
error[V0007]: value violates sh:minInclusive
  data graph: triple of focus node <http://example.org/alice>
   |
   |  <http://example.org/alice> <http://example.org/age> "-3"^^<http://www.w3.org/2001/XMLSchema#integer> .
   |                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this value is less than the required minimum
   |
  shapes graph: declared by <http://example.org/PersonShape>
   |
   |  [] sh:path <http://example.org/age> ;
   |     sh:minInclusive "0"^^<http://www.w3.org/2001/XMLSchema#integer> .
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ constraint declared here
   |
   = component: <http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent>
   = expected: a literal >= "0"^^<http://www.w3.org/2001/XMLSchema#integer>
   = actual:   "-3"^^<http://www.w3.org/2001/XMLSchema#integer>
   = note: focus node selected by sh:targetClass <http://example.org/Person>
   = help: change the value to satisfy the bound, or relax sh:minInclusive on the shape

error: 1 error, 0 warnings
```

### `validate` flags

| Flag | Values | Default | Description |
| --- | --- | --- | --- |
| `--diagnostics <MODE>` | `text`, `json`, `none` | `text` | Diagnostics output on stderr: human-readable text, NDJSON (one object per line), or disabled entirely |
| `--skip-lint` | flag | off | Skip shape lints during validation |
| `--deny-warnings` | flag | off | Exit with code 2 when shape lints report warnings or errors |

### Other diagnostics subcommands

- `shacl-validator lint <shapes.ttl>` — lint a shapes graph for common mistakes (13 rules) without validating any data.
- `shacl-validator explain V0007` — print a longer explanation of a diagnostic code (validator codes `V0001`-`V00xx`, lint codes `L0001`-`L00xx`).
- `shacl-validator why <shapes.ttl> <data.ttl> --focus <iri> [--shape <iri>]` — explain why a specific focus node does or does not conform, optionally restricted to one shape.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
