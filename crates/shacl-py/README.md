# shacl-rust (Python)

Python bindings for [shacl-rust](https://github.com/ensaremirerol/shacl-rust),
a SHACL validator for RDF graphs written in Rust. Passes all 120 applicable
tests of the W3C SHACL test suite.

```bash
pip install shacl-rust
```

```python
import shacl_rust

shapes = """
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:minInclusive 0 ; ] .
"""

data = """
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:alice a ex:Person ; ex:age "-3"^^xsd:integer .
"""

report = shacl_rust.validate(data, shapes)
print(report["conforms"])          # False
print(report["results"][0])        # violation details

# Fast path for large graphs (experimental interned index):
shacl_rust.conforms(data, shapes, experimental_index=True)
```

Large graphs can be streamed from disk or any binary file-like object, so
the serialized text is never fully held in memory:

```python
report = shacl_rust.validate_file("data.ttl", "shapes.ttl")

import gzip
with gzip.open("data.ttl.gz", "rb") as f:
    ok = shacl_rust.conforms_file(f, "shapes.ttl", data_format="turtle",
                                  experimental_index=True)
```

Formats: `turtle`/`ttl` (default), `nt`, `nq`, `rdf`, `jsonld`, `trig` via the
`data_format=` / `shapes_format=` keyword arguments (inferred from path
extensions for `validate_file`/`conforms_file`).

### Diagnostics

Pass `diagnostics=True` to `validate`/`validate_file` to get rustc-style
diagnostics (shape lints, then violation diagnostics) alongside the report:

```python
report = shacl_rust.validate(data, shapes, diagnostics=True)
print(report["conforms"])       # False
print(report["diagnostics"][0]["code"])      # e.g. "V0007"
print(report["diagnostics"][0]["severity"])  # "error", "warning", or "info"
print(report["diagnostics"][0]["title"])     # e.g. "value violates sh:minInclusive"
```

Each entry in `report["diagnostics"]` is a dict with `code`, `severity`,
`title`, `constraint_component`, `snippets`, `expected`, `actual`, `notes`,
`help`, `focus_node`, `source_shape`, `path`, and `verdict`. When
`diagnostics` is `False` or omitted (the default), the `"diagnostics"` key
is absent from the result dict entirely.
