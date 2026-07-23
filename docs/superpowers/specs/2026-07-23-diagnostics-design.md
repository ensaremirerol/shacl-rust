# Rustc-Style Diagnostics for shacl-rust — Design

**Date:** 2026-07-23
**Status:** approved pending user review

## Goal

Make shacl-rust as attentive as the Rust compiler: every problem — in the
shapes graph or in the data — is reported with a stable error code, the
offending content quoted and annotated, what was expected vs. what was found,
and an actionable fix suggestion. The output must serve two consumers equally:
humans in a terminal and LLMs/agents consuming structured JSON, from one
shared diagnostic model (rustc's `--error-format=json` architecture).

## Decisions (settled during brainstorming)

1. **Consumers:** both human terminal and machine JSON, one model, two
   renderers.
2. **No file positions.** Diagnostics quote reconstructed Turtle content;
   the (subject, predicate, object) triple identifies the location — the
   consumer greps. No span tracking is added to parsing.
3. **Scope:** both a shapes-graph **linter** (problems in shapes themselves)
   and **rich violation diagnostics** (shape context quoted inside every data
   violation).
4. **When lints run:** on every `validate` (before results) *and* via a
   dedicated `lint` subcommand; `--skip-lint` opts out during validate.
5. **Codes + explain:** every diagnostic kind has a stable code and
   `shacl-validator explain <CODE>` prints spec-grounded documentation.
   Explanations include the SHACL constraint component IRI.
6. **Conformance explanation:** an opt-in `why` subcommand explains why a
   given focus node did or did not fail a shape (targeting trace, per-
   constraint verdicts, vacuous-pass detection).
7. **Architecture: Approach A** — a post-processing diagnostic layer over the
   finished `ValidationReport` plus graphs. The validation hot path is not
   touched. Escape hatch: if post-hoc derivation is provably lossy for some
   constraint, that one constraint may later capture extra context at
   evaluation time (Approach C, per-constraint).

## Module layout

```
src/diagnostics/
  mod.rs          // Diagnostic, Snippet, DiagnosticCode, DiagnosticSeverity;
                  // from_report(), lint_shapes(), explain_conformance()
  registry.rs     // static registry: code -> {title, component IRI, spec ref,
                  // explanation, failing example, fixed example}
  derive.rs       // per-component derivation of expected/actual/help from
                  // shapes graph + result
  lint.rs         // lint rules over shapes graph + parsed shapes
  explain_pass.rs // the `why` evaluation trace
  render_text.rs  // terminal renderer (ANSI when tty, NO_COLOR respected)
  render_json.rs  // NDJSON renderer, stable schema
```

## The Diagnostic model

```rust
pub struct Diagnostic {
    pub code: DiagnosticCode,          // V#### (validation) | L#### (lint)
    pub severity: DiagnosticSeverity,  // Error | Warning | Info
    pub title: String,                 // "value violates sh:minInclusive"
    pub constraint_component: Option<String>, // full IRI; None for lints
    pub snippets: Vec<Snippet>,        // annotated reconstructed Turtle
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub notes: Vec<String>,            // focus node, path, targeting, ...
    pub help: Option<String>,          // actionable suggestion
    pub focus_node: Option<String>,
    pub source_shape: Option<String>,
    pub path: Option<String>,
    pub verdict: Option<Verdict>,      // `why` mode only:
                                       // Conforms | Violates | NotTargeted | Vacuous
}

pub struct Snippet {
    pub origin: SnippetOrigin,         // DataGraph | ShapesGraph
    pub turtle: String,                // pretty-printed reconstruction
    pub annotation: String,            // caret-line message on offending term
}
```

Severity mapping: `sh:Violation → Error`, `sh:Warning → Warning`,
`sh:Info → Info`; lints per the table below; `why` traces are Info.

### Terminal rendering (reference layout)

```
error[V0007]: value violates sh:minInclusive
  data graph: triple of focus node <http://example.org/alice>
   |
   |  ex:alice ex:age "-3"^^xsd:integer .
   |                  ^^^^^^^^^^^^^^^^^ this value is less than the required minimum
   |
  shapes graph: declared by <http://example.org/PersonShape>
   |
   |  ex:PersonShape sh:property [
   |      sh:path ex:age ;
   |      sh:minInclusive 0 ;
   |      ^^^^^^^^^^^^^^^^^ constraint declared here
   |  ] .
   |
   = component: sh:MinInclusiveConstraintComponent
   = expected: a literal >= "0"^^xsd:integer
   = actual:   "-3"^^xsd:integer
   = note: focus node selected by sh:targetClass ex:Person
   = help: change the value to satisfy >= 0, or relax sh:minInclusive on ex:PersonShape
```

Diagnostics go to **stderr**; the validation report stays on stdout/`-o`
unchanged (existing pipelines and the benchmark contract are untouched).
A summary line closes the stream:
`error: data does not conform: 3 errors, 2 warnings (shapes: 1 lint warning)`.

### JSON rendering

NDJSON on stderr, one object per diagnostic:

```json
{"code":"V0007","severity":"error","title":"value violates sh:minInclusive",
 "constraint_component":"http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent",
 "snippets":[{"origin":"data","turtle":"...","annotation":"..."}],
 "expected":"...","actual":"...","notes":["..."],"help":"...",
 "focus_node":"http://example.org/alice","source_shape":"...","path":"...",
 "verdict":null}
```

Selected by `--diagnostics <text|json|none>` (default `text`; `none`
reproduces today's behavior exactly).

## Codes and the explain registry

- `V####` — one per constraint component (~30). `L####` — lints. Codes are
  stable API once released; new codes append, none are reused.
- `registry.rs` maps every code to: title template, constraint component IRI
  (V-codes), SHACL spec section reference, long-form explanation, a minimal
  failing example, and its fixed version.
- `shacl-validator explain V0007` prints the entry, leading with the
  component IRI and its parameters.
- Derivation (`derive.rs`) is table-driven by `sourceConstraintComponent`
  IRI: each entry computes expected/actual/help from the constraint
  declaration (looked up via the result's source shape in the shapes graph)
  plus the result's value/path. Components without a specialized entry get a
  generic-but-complete rendering (SPARQL constraints show query + solution
  bindings). Custom (non-`sh:`) components map to a generic `V0000` with the
  component IRI carried in the field.

## Lint rules v1

| Code | Severity | Rule |
|---|---|---|
| L0001 | Error | Property shape without `sh:path` |
| L0002 | Error | Invalid `sh:pattern` regex (today silently never fires) |
| L0003 | Error | SPARQL constraint/target query fails to parse |
| L0004 | Warning | Unknown `sh:`-namespace term, did-you-mean via Levenshtein <= 2 |
| L0005 | Warning | Path-requiring constraint on a node shape (ignored today) |
| L0006 | Warning | Unsatisfiable bounds: minCount>maxCount, minLength>maxLength, minInclusive>maxInclusive |
| L0007 | Warning | `sh:class`/`sh:datatype`/`sh:nodeKind` object has wrong term kind |
| L0008 | Warning | Empty `sh:in` / `sh:languageIn` list |
| L0009 | Warning | Non-comparable bound on `sh:minInclusive`/co. |
| L0010 | Warning | `sh:ignoredProperties` without `sh:closed` |
| L0011 | Info | Dead shape: no targets and referenced by no other shape |
| L0012 | Info | `sh:deactivated` shape reminder |

Severity policy: Error-level lints fail the `lint` subcommand (exit 1) but do
**not** change `validate`'s exit code (stays conformance-driven);
`--deny-warnings` on validate escalates lint warnings to exit 2.

## The `why` subcommand (conformance explanation)

```
shacl-validator why <SHAPES> <DATA> --focus <node> [--shape <shape-iri>]
```

For the focus node against one shape (or all shapes when `--shape` omitted),
emits Info diagnostics tracing evaluation:

- **Targeting:** which target selected the node, or why none did
  ("not selected: node lacks rdf:type ex:Person; nearest target is
  sh:targetClass ex:Person on ex:PersonShape").
- **Per constraint:** value nodes the path resolved to and the verdict with
  expected/actual. Vacuous passes are called out explicitly:
  "sh:minInclusive 0 evaluated against 0 value nodes (path ex:age resolved
  to nothing) — vacuously conforms; add sh:minCount 1 if values are
  required."

Implemented as a dedicated pass in `explain_pass.rs` reusing existing
target/path-resolution primitives (small `pub(crate)` visibility promotions
in `validation/`); the validate hot path is untouched. JSON carries
`verdict: conforms | violates | not-targeted | vacuous`.

## CLI summary

- `validate ... [--diagnostics text|json|none] [--skip-lint] [--deny-warnings]`
  — lints first, then violation diagnostics, all on stderr; report on
  stdout/`-o` as today; exit 0/1 by conformance, 2 with `--deny-warnings`
  and lint warnings present.
- `lint <SHAPES_FILE> [--diagnostics ...]` — shapes only; exit 1 on
  Error-level lints.
- `explain <CODE>` — registry entry.
- `why <SHAPES> <DATA> --focus <node> [--shape <iri>]` — conformance trace.

## Library and bindings

- `diagnostics::from_report(&report, &dataset, &shapes) -> Vec<Diagnostic>`
- `diagnostics::lint_shapes(&shapes_graph, &shapes) -> Vec<Diagnostic>`
- `diagnostics::explain_conformance(&dataset, &shapes, focus, shape) -> Vec<Diagnostic>`
- Renderers are pure `&[Diagnostic] -> String`.
- Python: `validate(..., diagnostics=True)` adds a `diagnostics` list (same
  JSON schema, lint diagnostics followed by violation diagnostics) to the
  returned dict. wasm: deferred (nothing blocks it).

## Performance

Derivation runs only per violation; the lint pass runs once per invocation
over the (small) shapes graph; `--skip-lint` removes even that. Zero cost on
the conforming hot path. `why` is a separate command and pays its own cost.

## Testing

- Golden-file snapshots (`tests/fixtures/diagnostics/*.expected`) for
  terminal and JSON rendering: at least one fixture per V-code family and
  one per L-code; plain string comparison, no new dev-deps.
- Registry completeness test: every constraint component maps to a code and
  every code has an explain entry (build fails when a new constraint forgets
  its diagnostic).
- Determinism: diagnostics sorted by code, then focus node.
- `why` fixtures covering: targeted+conforms, targeted+violates,
  not-targeted, vacuous pass.

## Non-goals (v1)

- File line/column positions.
- Machine-applicable auto-fixes (help text suggests; never edits).
- SARIF output.
- wasm exposure of diagnostics.
- Internationalization.
