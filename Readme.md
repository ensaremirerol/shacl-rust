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


## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
