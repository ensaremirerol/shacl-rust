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

Formats: `turtle`/`ttl` (default), `nt`, `nq`, `rdf`, `jsonld`, `trig` via the
`data_format=` / `shapes_format=` keyword arguments.
