# Rustc-Style Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rich, stable-coded diagnostics for shacl-rust — shape lints, annotated violation explanations, an `explain` registry, and a `why` conformance tracer — rendered for terminals and as NDJSON from one shared model.

**Architecture:** A post-processing layer (`src/diagnostics/`) over the finished `ValidationReport` plus the shapes/data graphs (spec "Approach A"). The validation hot path is untouched; diagnostics derive everything by querying the graphs per violation. Two pure renderers consume one `Diagnostic` struct.

**Tech Stack:** Rust 2021, oxigraph 0.5 model API, clap 4 (CLI), existing test style (integration tests in `tests/`, golden files under `tests/fixtures/`).

**Spec:** `docs/superpowers/specs/2026-07-23-diagnostics-design.md` — read it first.

## Global Constraints

- After every task: `cargo test` green (including 120/120 conformance), `cargo clippy --all-targets --all-features` zero warnings, `cargo fmt --all -- --check` clean.
- `cargo check -p shacl-wasm --target wasm32-unknown-unknown` must pass after tasks touching `src/` (diagnostics module is native+wasm neutral: no rayon, no I/O).
- No new dependencies in any `Cargo.toml`.
- The validation hot path (`src/validation/`, `src/core/`, `src/indexed_graph.rs`) may only gain accessor methods and `pub(crate)` visibility promotions — no behavioral changes.
- Report output on stdout/`-o` must remain byte-identical to today; all diagnostics go to stderr.
- Codes are stable API: use exactly the codes in the table below; never renumber.
- Commit after every task with the given message, ending with:
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`
- Do NOT push — the user reviews locally first.
- Line numbers reference pre-task file state; anchor on quoted code, not numbers.

## Code table (authoritative)

V-codes, one per constraint component (`http://www.w3.org/ns/shacl#` prefix elided):

| Code | Component | Title |
|---|---|---|
| V0000 | (any non-sh: component) | custom constraint component violated |
| V0001 | ClassConstraintComponent | value is not an instance of the required class |
| V0002 | DatatypeConstraintComponent | value does not have the required datatype |
| V0003 | NodeKindConstraintComponent | value has the wrong node kind |
| V0004 | MinCountConstraintComponent | too few values for property |
| V0005 | MaxCountConstraintComponent | too many values for property |
| V0006 | MinExclusiveConstraintComponent | value is not greater than the exclusive minimum |
| V0007 | MinInclusiveConstraintComponent | value violates sh:minInclusive |
| V0008 | MaxExclusiveConstraintComponent | value is not less than the exclusive maximum |
| V0009 | MaxInclusiveConstraintComponent | value violates sh:maxInclusive |
| V0010 | MinLengthConstraintComponent | value is shorter than sh:minLength |
| V0011 | MaxLengthConstraintComponent | value is longer than sh:maxLength |
| V0012 | PatternConstraintComponent | value does not match sh:pattern |
| V0013 | LanguageInConstraintComponent | language tag not permitted by sh:languageIn |
| V0014 | UniqueLangConstraintComponent | language tag used more than once |
| V0015 | EqualsConstraintComponent | values differ from sh:equals property |
| V0016 | DisjointConstraintComponent | value shared with sh:disjoint property |
| V0017 | LessThanConstraintComponent | value not less than sh:lessThan property value |
| V0018 | LessThanOrEqualsConstraintComponent | value not <= sh:lessThanOrEquals property value |
| V0019 | HasValueConstraintComponent | required value missing |
| V0020 | InConstraintComponent | value not in the allowed list |
| V0021 | NodeConstraintComponent | value does not conform to the referenced node shape |
| V0022 | QualifiedMinCountConstraintComponent | too few values conform to the qualified shape |
| V0023 | QualifiedMaxCountConstraintComponent | too many values conform to the qualified shape |
| V0024 | AndConstraintComponent | value fails a conjunct of sh:and |
| V0025 | OrConstraintComponent | value matches no disjunct of sh:or |
| V0026 | XoneConstraintComponent | value does not match exactly one shape of sh:xone |
| V0027 | NotConstraintComponent | value matches the negated shape |
| V0028 | ClosedConstraintComponent | property not allowed on closed shape |
| V0029 | SPARQLConstraintComponent | SPARQL constraint produced a violation |

L-codes: as in the spec table (L0001 Error missing sh:path … L0012 Info deactivated). Copy severities exactly from the spec.

---

### Task 1: Diagnostic model, sorting, and ValidationResult accessors

**Files:**
- Create: `src/diagnostics/mod.rs`
- Modify: `src/lib.rs` (add `pub mod diagnostics;`)
- Modify: `src/validation/report.rs` (accessors only)
- Test: in-module `#[cfg(test)]` in `src/diagnostics/mod.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces (later tasks rely on these exact items):
  - `pub struct Diagnostic { pub code: &'static str, pub severity: DiagnosticSeverity, pub title: String, pub constraint_component: Option<String>, pub snippets: Vec<Snippet>, pub expected: Option<String>, pub actual: Option<String>, pub notes: Vec<String>, pub help: Option<String>, pub focus_node: Option<String>, pub source_shape: Option<String>, pub path: Option<String>, pub verdict: Option<Verdict> }`
  - `pub enum DiagnosticSeverity { Error, Warning, Info }` (Ord: Error < Warning < Info)
  - `pub enum SnippetOrigin { DataGraph, ShapesGraph }`
  - `pub struct Snippet { pub origin: SnippetOrigin, pub turtle: String, pub highlight: String, pub annotation: String }`
  - `pub enum Verdict { Conforms, Violates, NotTargeted, Vacuous }`
  - `pub fn sort_diagnostics(diags: &mut [Diagnostic])` — by (code, focus_node)
  - On `ValidationResult<'a>`: `pub fn focus_node(&self) -> TermRef<'a>`, `pub fn source_shape(&self) -> NamedOrBlankNodeRef<'a>`, `pub fn severity(&self) -> NamedNodeRef<'a>`, `pub fn source_constraint_component(&self) -> Option<NamedNodeRef<'a>>`, `pub fn result_path(&self) -> Option<&Path<'a>>`, `pub fn value(&self) -> Option<TermRef<'a>>`, `pub fn messages(&self) -> &[String]`

- [ ] **Step 1: Write the failing test** (bottom of the new `src/diagnostics/mod.rs`, with the types above it still absent — write the test module first, watch it fail to compile, then add types):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn d(code: &'static str, focus: &str) -> Diagnostic {
        Diagnostic {
            code,
            severity: DiagnosticSeverity::Warning,
            title: String::new(),
            constraint_component: None,
            snippets: Vec::new(),
            expected: None,
            actual: None,
            notes: Vec::new(),
            help: None,
            focus_node: Some(focus.to_string()),
            source_shape: None,
            path: None,
            verdict: None,
        }
    }

    #[test]
    fn sorts_by_code_then_focus() {
        let mut v = vec![d("V0007", "b"), d("L0001", "z"), d("V0007", "a")];
        sort_diagnostics(&mut v);
        let keys: Vec<_> = v.iter().map(|x| (x.code, x.focus_node.clone().unwrap())).collect();
        assert_eq!(
            keys,
            vec![
                ("L0001", "z".to_string()),
                ("V0007", "a".to_string()),
                ("V0007", "b".to_string())
            ]
        );
    }

    #[test]
    fn severity_orders_error_first() {
        assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Info);
    }
}
```

- [ ] **Step 2: Run** `cargo test --lib diagnostics` — expect compile FAILURE (types not defined).

- [ ] **Step 3: Implement the model** (top of `src/diagnostics/mod.rs`):

```rust
//! Rustc-style diagnostics: a shared model rendered for terminals and as
//! NDJSON. Built as a post-processing layer over validation output; see
//! docs/superpowers/specs/2026-07-23-diagnostics-design.md.

mod derive;
mod explain_pass;
mod lint;
mod registry;
mod render_json;
mod render_text;

pub use derive::from_report;
pub use explain_pass::explain_conformance;
pub use lint::lint_shapes;
pub use registry::{entry, RegistryEntry};
pub use render_json::{diagnostic_to_json, render_ndjson};
pub use render_text::render_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetOrigin {
    DataGraph,
    ShapesGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Conforms,
    Violates,
    NotTargeted,
    Vacuous,
}

/// A quoted, annotated piece of reconstructed Turtle. `highlight` is the
/// exact substring of `turtle` the renderer underlines; `annotation` is the
/// caret-line message.
#[derive(Debug, Clone)]
pub struct Snippet {
    pub origin: SnippetOrigin,
    pub turtle: String,
    pub highlight: String,
    pub annotation: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub title: String,
    pub constraint_component: Option<String>,
    pub snippets: Vec<Snippet>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub focus_node: Option<String>,
    pub source_shape: Option<String>,
    pub path: Option<String>,
    pub verdict: Option<Verdict>,
}

/// Deterministic output order: code, then focus node.
pub fn sort_diagnostics(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        a.code
            .cmp(b.code)
            .then_with(|| a.focus_node.cmp(&b.focus_node))
    });
}
```

Until Tasks 2–7 exist, stub the submodules so the crate compiles: create each of `derive.rs`, `explain_pass.rs`, `lint.rs`, `registry.rs`, `render_json.rs`, `render_text.rs` containing only the item the re-export needs, e.g. `registry.rs`:

```rust
pub struct RegistryEntry;
pub fn entry(_code: &str) -> Option<&'static RegistryEntry> { None }
```

and analogous minimal stubs (`from_report`, `lint_shapes`, `explain_conformance`, `render_text`, `render_ndjson`, `diagnostic_to_json`) returning empty values with `todo!()`-free bodies (empty `Vec`/`String`/`serde_json::Value::Null`) so nothing panics if called. Each later task replaces its stub wholesale.

Add to `src/lib.rs` after `pub mod core;`: `pub mod diagnostics;`

Add the accessors to `src/validation/report.rs` (inside `impl<'a> ValidationResult<'a>`, after `with_details`):

```rust
    pub fn focus_node(&self) -> TermRef<'a> {
        self.focus_node
    }

    pub fn source_shape(&self) -> NamedOrBlankNodeRef<'a> {
        self.source_shape
    }

    pub fn severity(&self) -> NamedNodeRef<'a> {
        self.severity
    }

    pub fn source_constraint_component(&self) -> Option<NamedNodeRef<'a>> {
        self.source_constraint_component
    }

    pub fn result_path(&self) -> Option<&Path<'a>> {
        self.result_path.as_ref()
    }

    pub fn value(&self) -> Option<TermRef<'a>> {
        self.value
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
```

- [ ] **Step 4: Run** `cargo test --lib diagnostics` — expect both tests PASS. Run `cargo test` — everything green.

- [ ] **Step 5: Commit**

```bash
git add src/diagnostics src/lib.rs src/validation/report.rs
git commit -m "feat: diagnostic model and ValidationResult accessors"
```

---

### Task 2: Code registry with explain entries

**Files:**
- Replace: `src/diagnostics/registry.rs`
- Test: in-module tests

**Interfaces:**
- Consumes: code table above.
- Produces:
  - `pub struct RegistryEntry { pub code: &'static str, pub title: &'static str, pub component: Option<&'static str>, pub spec_ref: &'static str, pub explanation: &'static str, pub failing_example: &'static str, pub fixed_example: &'static str }`
  - `pub fn entry(code: &str) -> Option<&'static RegistryEntry>`
  - `pub fn code_for_component(component_iri: &str) -> &'static str` (V0000 for unknown)
  - `pub fn all_entries() -> &'static [RegistryEntry]`

- [ ] **Step 1: Failing tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_component_maps_to_a_specific_code() {
        let iri = "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";
        assert_eq!(code_for_component(iri), "V0007");
        assert_eq!(code_for_component("http://example.org/custom"), "V0000");
    }

    #[test]
    fn every_code_has_a_complete_entry() {
        for e in all_entries() {
            assert!(!e.title.is_empty(), "{} missing title", e.code);
            assert!(!e.explanation.is_empty(), "{} missing explanation", e.code);
            assert!(!e.failing_example.is_empty(), "{} missing example", e.code);
            assert!(!e.fixed_example.is_empty(), "{} missing fix", e.code);
            assert!(!e.spec_ref.is_empty(), "{} missing spec ref", e.code);
        }
    }

    #[test]
    fn v_codes_carry_component_and_l_codes_do_not() {
        for e in all_entries() {
            if e.code.starts_with('V') && e.code != "V0000" {
                assert!(e.component.is_some(), "{}", e.code);
            }
            if e.code.starts_with('L') {
                assert!(e.component.is_none(), "{}", e.code);
            }
        }
    }

    #[test]
    fn lookup_by_code() {
        assert_eq!(entry("V0007").unwrap().code, "V0007");
        assert!(entry("V9999").is_none());
    }
}
```

- [ ] **Step 2: Run** `cargo test --lib registry` — FAIL (stub in place).

- [ ] **Step 3: Implement.** Structure:

```rust
pub struct RegistryEntry {
    pub code: &'static str,
    pub title: &'static str,
    pub component: Option<&'static str>,
    pub spec_ref: &'static str,
    pub explanation: &'static str,
    pub failing_example: &'static str,
    pub fixed_example: &'static str,
}

static ENTRIES: &[RegistryEntry] = &[
    RegistryEntry {
        code: "V0007",
        title: "value violates sh:minInclusive",
        component: Some("http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MinInclusiveConstraintComponent",
        explanation: "sh:minInclusive requires every value node to be a literal \
that compares greater than or equal to the given bound. Values are compared by \
type: numerics numerically, xsd:dateTime by timeline, other literals lexically; \
a value that cannot be compared to the bound (wrong type, non-literal) also \
violates.",
        failing_example: "ex:alice ex:age \"-3\"^^xsd:integer .\n# with: sh:path ex:age ; sh:minInclusive 0",
        fixed_example: "ex:alice ex:age \"3\"^^xsd:integer .",
    },
    // ... one entry per code in the code table ...
];

pub fn all_entries() -> &'static [RegistryEntry] {
    ENTRIES
}

pub fn entry(code: &str) -> Option<&'static RegistryEntry> {
    ENTRIES.iter().find(|e| e.code == code)
}

pub fn code_for_component(component_iri: &str) -> &'static str {
    ENTRIES
        .iter()
        .find(|e| e.component == Some(component_iri))
        .map(|e| e.code)
        .unwrap_or("V0000")
}
```

Write **all** entries: V0000–V0029 (component IRIs and titles exactly from the code table; `spec_ref` is `https://www.w3.org/TR/shacl/#<ComponentName>`; explanation = 2–4 sentences describing the component's textual definition from the SHACL spec, mentioning the parameter(s); failing/fixed examples = 1–3 lines of Turtle each, minimal, in the V0007 style above) and L0001–L0012 (component `None`; `spec_ref` points at the relevant spec anchor, e.g. L0001 → `https://www.w3.org/TR/shacl/#property-shapes`, L0002 → `#PatternConstraintComponent`, L0004/L0011/L0012 → `#shapes`, L0010 → `#ClosedConstraintComponent`; explanation states what the lint detects and why the construct is broken or inert; examples show a bad shape and its fix). This is mechanical writing, ~40 entries; do not skip any — the completeness test enforces presence, and `explain` output quality is the product.

- [ ] **Step 4: Run** `cargo test --lib registry` — PASS; `cargo test` green.

- [ ] **Step 5: Commit** — `feat: diagnostic code registry with explain entries`

---

### Task 3: Text renderer

**Files:**
- Replace: `src/diagnostics/render_text.rs`
- Test: `tests/diagnostics_render.rs` + golden files `tests/fixtures/diagnostics/text_basic.expected`

**Interfaces:**
- Consumes: `Diagnostic`, `Snippet`, `DiagnosticSeverity`, `SnippetOrigin` from Task 1.
- Produces: `pub fn render_text(diags: &[Diagnostic], color: bool) -> String` — includes the closing summary line. Colors: ANSI red/yellow/cyan for error/warning/info headers and carets only, plain otherwise. `color: false` in all tests.

Reference layout (single diagnostic, `color: false`) — this exact text is the golden file `text_basic.expected`:

```
error[V0007]: value violates sh:minInclusive
  data graph: triple of focus node <http://example.org/alice>
   |
   |  <http://example.org/alice> <http://example.org/age> "-3"^^<http://www.w3.org/2001/XMLSchema#integer> .
   |                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this value is less than the required minimum
   |
  shapes graph: declared by <http://example.org/PersonShape>
   |
   |  [] sh:path <http://example.org/age> ;
   |     sh:minInclusive "0"^^<http://www.w3.org/2001/XMLSchema#integer> .
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ constraint declared here
   |
   = component: <http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent>
   = expected: a literal >= "0"^^<http://www.w3.org/2001/XMLSchema#integer>
   = actual:   "-3"^^<http://www.w3.org/2001/XMLSchema#integer>
   = note: focus node selected by sh:targetClass <http://example.org/Person>
   = help: change the value to satisfy the bound, or relax sh:minInclusive on the shape

error: 1 error, 0 warnings
```

Rendering rules:
- Header: `{severity}[{code}]: {title}` where severity renders `error|warning|info`.
- Each snippet: origin line (`data graph: triple of focus node {focus}` / `shapes graph: declared by {source_shape}`), then `   |` fence, each turtle line prefixed `   |  `, and beneath the line containing `highlight`, a caret line: spaces to the highlight's column, `^` repeated `highlight.chars().count()` times, space, annotation. Highlight matching: first line of `turtle` containing `highlight` as substring; if absent, the annotation renders as `   = note:` fallback instead of carets.
- Then `= component:`, `= expected:`, `= actual:`, one `= note:` per note, `= help:` — each omitted when `None`/empty.
- Verdict (why-mode) renders as first note: `verdict: conforms|violates|not targeted|vacuously conforms`.
- Summary: counts by severity: `error: {e} error(s), {w} warning(s)` — when e==0: `warning: ...` header word; exact pluralization: use `1 error`, `2 errors` (plain `s` rule).

- [ ] **Step 1: Failing test** in `tests/diagnostics_render.rs`:

```rust
use shacl_rust::diagnostics::*;

fn sample() -> Diagnostic {
    Diagnostic {
        code: "V0007",
        severity: DiagnosticSeverity::Error,
        title: "value violates sh:minInclusive".into(),
        constraint_component: Some(
            "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent".into(),
        ),
        snippets: vec![
            Snippet {
                origin: SnippetOrigin::DataGraph,
                turtle: "<http://example.org/alice> <http://example.org/age> \"-3\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                highlight: "\"-3\"^^<http://www.w3.org/2001/XMLSchema#integer>".into(),
                annotation: "this value is less than the required minimum".into(),
            },
            Snippet {
                origin: SnippetOrigin::ShapesGraph,
                turtle: "[] sh:path <http://example.org/age> ;\n   sh:minInclusive \"0\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                highlight: "sh:minInclusive \"0\"^^<http://www.w3.org/2001/XMLSchema#integer> .".into(),
                annotation: "constraint declared here".into(),
            },
        ],
        expected: Some("a literal >= \"0\"^^<http://www.w3.org/2001/XMLSchema#integer>".into()),
        actual: Some("\"-3\"^^<http://www.w3.org/2001/XMLSchema#integer>".into()),
        notes: vec!["focus node selected by sh:targetClass <http://example.org/Person>".into()],
        help: Some("change the value to satisfy the bound, or relax sh:minInclusive on the shape".into()),
        focus_node: Some("<http://example.org/alice>".into()),
        source_shape: Some("<http://example.org/PersonShape>".into()),
        path: Some("<http://example.org/age>".into()),
        verdict: None,
    }
}

#[test]
fn text_rendering_matches_golden_file() {
    let rendered = render_text(&[sample()], false);
    let expected = include_str!("fixtures/diagnostics/text_basic.expected");
    assert_eq!(rendered, expected, "\n--- rendered ---\n{rendered}");
}
```

Create the golden file with exactly the reference layout above (trailing newline after summary line).

- [ ] **Step 2: Run** `cargo test --test diagnostics_render` — FAIL (stub returns empty string).

- [ ] **Step 3: Implement** `render_text` per the rules. Column math uses `chars().count()` on the prefix before the highlight (byte-safe display alignment is out of scope; terms are ASCII-heavy).

- [ ] **Step 4: Run** — PASS (iterate on the golden file only if the *plan's* layout was internally inconsistent; the file is the contract).

- [ ] **Step 5: Commit** — `feat: terminal renderer for diagnostics`

---

### Task 4: JSON renderer

**Files:**
- Replace: `src/diagnostics/render_json.rs`
- Test: extend `tests/diagnostics_render.rs`

**Interfaces:**
- Consumes: Task 1 model.
- Produces: `pub fn diagnostic_to_json(d: &Diagnostic) -> serde_json::Value`, `pub fn render_ndjson(diags: &[Diagnostic]) -> String` (one compact JSON object per line, `\n`-terminated).

Schema (exact keys): `code`, `severity` (`"error"|"warning"|"info"`), `title`, `constraint_component` (string|null), `snippets` (array of `{origin: "data"|"shapes", turtle, highlight, annotation}`), `expected`, `actual` (string|null), `notes` (array), `help` (string|null), `focus_node`, `source_shape`, `path` (string|null), `verdict` (`"conforms"|"violates"|"not-targeted"|"vacuous"`|null).

- [ ] **Step 1: Failing test:**

```rust
#[test]
fn ndjson_schema_is_stable() {
    let line = render_ndjson(&[sample()]);
    let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(v["code"], "V0007");
    assert_eq!(v["severity"], "error");
    assert_eq!(v["snippets"][0]["origin"], "data");
    assert_eq!(v["snippets"][1]["origin"], "shapes");
    assert_eq!(v["verdict"], serde_json::Value::Null);
    assert!(v["constraint_component"].as_str().unwrap().ends_with("MinInclusiveConstraintComponent"));
    assert_eq!(line.matches('\n').count(), 1);
}
```

- [ ] **Step 2: Run** — FAIL. **Step 3:** implement with `serde_json::json!`. **Step 4:** PASS + full suite. **Step 5: Commit** — `feat: NDJSON renderer for diagnostics`

---

### Task 5: Snippet reconstruction and violation derivation (`from_report`)

**Files:**
- Replace: `src/diagnostics/derive.rs`
- Test: `tests/diagnostics_derive.rs`

**Interfaces:**
- Consumes: registry (`code_for_component`, `entry`), model, `ValidationResult` accessors, `ValidationDataset::{data, shapes_graph}`, `DataView` lookups, `Shape` pub fields.
- Produces: `pub fn from_report<'a>(report: &ValidationReport<'a>, dataset: &'a ValidationDataset, shapes: &'a [Shape<'a>]) -> Vec<Diagnostic>` (sorted via `sort_diagnostics`).

Derivation per result:
1. `code = code_for_component(component_iri)` (component absent → V0000 with `constraint_component: None`).
2. `title` from registry entry (fallback: first result message).
3. Severity map: result severity IRI ending `Violation`→Error, `Warning`→Warning, else Info.
4. **Data snippet** (when `value()` and `result_path()` present): `"{focus} {first-path-predicate} {value} ."` with `highlight = value.to_string()`; annotation from a per-component table (below). When no path (node shapes): `"{focus} ."`, highlight = focus. When no value: skip data snippet.
5. **Shapes snippet**: query `dataset.shapes_graph()` for the source shape node's triples via `triples_for_subject`; render up to 8 `predicate object ;` lines as `"{source_shape} {p1} {o1} ;\n    {p2} {o2} ."`; `highlight` = the line whose predicate IRI contains the component's parameter keyword (strip `ConstraintComponent`, lowercase first letter: MinInclusive→`minInclusive`); annotation `"constraint declared here"`. If no matching predicate line, highlight the first line.
6. `expected`/`actual`/`help` from a match on component keyword — implement this table (annotation, expected template, help) for: MinInclusive/MinExclusive/MaxInclusive/MaxExclusive (`expected: a literal {op} {bound}` where bound = the constraint object from the shapes snippet lookup; annotation `this value is {not within} the required bound` — exact strings below), MinCount/MaxCount (expected `at least|at most {n} value(s) for path {path}`, actual = count note from messages), Datatype (`expected: a literal with datatype {dt}`), Class (`expected: an instance of {class}`), Pattern (`expected: a value matching {pattern}`), In (`expected: one of the sh:in list`), Closed (`expected: only declared properties; property {path} is not allowed`), everything else: `expected = None`, `actual = value`, help from registry explanation's first sentence.
   Exact annotation strings for the four range components: `"this value is less than the required minimum"` (MinInclusive), `"this value is not greater than the exclusive minimum"` (MinExclusive), `"this value is greater than the required maximum"` (MaxInclusive), `"this value is not less than the exclusive maximum"` (MaxExclusive). All other components: annotation = title.
7. `actual = value().map(|v| v.to_string())`.
8. `notes`: one per result message (prefixed nothing — raw), plus, when the owning top-level shape (found by scanning `shapes` for the shape whose node == source shape, else whose property_shapes contain it) has targets: `"focus node selected by {target}"` using `Target`'s Display.
9. `path` = result_path Display; `focus_node`/`source_shape` = Display strings.

- [ ] **Step 1: Failing test** in `tests/diagnostics_derive.rs`:

```rust
use shacl_rust::diagnostics::{from_report, DiagnosticSeverity};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::{parse_shapes, validation};

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:age ; sh:minInclusive 0 ; ] .
"#;
const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:alice a ex:Person ; ex:age "-3"^^xsd:integer .
"#;

#[test]
fn min_inclusive_violation_derives_v0007() {
    let data = read_graph_from_string(DATA, "turtle").unwrap();
    let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&shapes_graph).unwrap();
    let dataset = ValidationDataset::from_graphs(data, shapes_graph.clone()).unwrap();
    let report = validation::validate(&dataset, &shapes);

    let diags = from_report(&report, &dataset, &shapes);
    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.code, "V0007");
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert_eq!(d.snippets.len(), 2, "data + shapes snippets");
    assert!(d.snippets[0].turtle.contains("age"));
    assert!(d.snippets[0].highlight.contains("-3"));
    assert!(d.snippets[1].turtle.contains("minInclusive"));
    assert!(d.expected.as_deref().unwrap().contains(">="));
    assert!(d.actual.as_deref().unwrap().contains("-3"));
    assert!(d.notes.iter().any(|n| n.contains("targetClass")));
    assert!(d.focus_node.as_deref().unwrap().contains("alice"));
}

#[test]
fn custom_component_falls_back_to_v0000() {
    // sh:sparql with a component-less constraint yields SPARQLConstraintComponent -> V0029;
    // a synthetic result with a non-sh component is unit-covered in registry —
    // here assert the sparql path maps to V0029.
    let shapes = r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:sparql [ sh:select """SELECT $this WHERE { $this <http://example.org/age> ?a . FILTER(?a < 0) }""" ; ] .
    "#;
    let data = read_graph_from_string(DATA, "turtle").unwrap();
    let sg = read_graph_from_string(shapes, "turtle").unwrap();
    let parsed = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(data, sg.clone()).unwrap();
    let report = validation::validate(&dataset, &parsed);
    let diags = from_report(&report, &dataset, &parsed);
    assert_eq!(diags[0].code, "V0029");
}
```

- [ ] **Step 2: Run** — FAIL. **Step 3:** implement per the numbered derivation rules. **Step 4:** PASS + full suite. **Step 5: Commit** — `feat: derive rich diagnostics from validation reports`

---

### Task 6: Shape linter

**Files:**
- Replace: `src/diagnostics/lint.rs`
- Test: `tests/diagnostics_lint.rs`

**Interfaces:**
- Consumes: model, registry `entry`, `Shape` pub fields (`node`, `path`, `targets`, `constraints`, `property_shapes`, `closed`, `deactivated`), `Constraint` enum variants, shapes `Graph`.
- Produces: `pub fn lint_shapes<'a>(shapes_graph: &'a Graph, shapes: &'a [Shape<'a>]) -> Vec<Diagnostic>` (sorted).

Rule implementations (each pushes a Diagnostic with the L-code, severity per spec table, a ShapesGraph snippet quoting the shape node's relevant triples with the offending line highlighted, and a help string):

- **L0001** (Error): walk `shapes_graph` for subjects of `sh:minCount|sh:maxCount|sh:datatype|...` that are objects of `sh:property` but have no `sh:path` — simpler and sufficient: any object of `sh:property` (named or blank) lacking `sh:path`.
- **L0002** (Error): for every `sh:pattern` literal in the graph, run the same flag-translation as `PatternConstraint::new` and report when `regex::Regex::new` fails; include the regex error text in `actual`.
- **L0003** (Error): every `sh:select`/`sh:ask` literal that `spargebra::SparqlParser` (with the node's `sh:prefixes`, via `utils::parse_shacl_prefixes`) fails to parse; parse error text in `actual`.
- **L0004** (Warning): every predicate IRI starting with `http://www.w3.org/ns/shacl#` not in a whitelist built from `src/vocab/sh.rs` constants (add `pub fn all_terms() -> &'static [NamedNodeRef<'static>]` to `src/vocab/sh.rs` listing every constant; that addition belongs to this task). Did-you-mean: Levenshtein distance ≤ 2 against whitelist local names (implement a 15-line DP levenshtein in `lint.rs`).
- **L0005** (Warning): for each parsed node shape (shape with `path: None`), each constraint where `Constraint::requires_path()` is true.
- **L0006** (Warning): per shape: minCount > maxCount; minLength > maxLength; via `utils::to_comparable`+`compare_comparables`: minInclusive > maxInclusive.
- **L0007** (Warning): objects of `sh:class`/`sh:datatype` that are literals or blank nodes; objects of `sh:nodeKind` not one of the six `sh:` node-kind IRIs.
- **L0008** (Warning): `Constraint::In(list)` with empty list; `LanguageIn` with empty list.
- **L0009** (Warning): min/max-inclusive/exclusive constraints whose bound term is a non-literal, or a literal that `utils::to_comparable` classifies as `Text` with a non-string datatype... keep exactly: bound is not a literal, OR bound literal is ill-formed per `utils::literal_is_ill_formed`.
- **L0010** (Warning): shapes graph has `sh:ignoredProperties` on a node without `sh:closed true`.
- **L0011** (Info): parsed shape with empty `targets` whose `node` is not referenced from any other shape's graph triples via `sh:node|sh:property|sh:and|sh:or|sh:xone|sh:not|sh:qualifiedValueShape` (check membership of the node as object anywhere under those predicates, including inside RDF lists via `utils::parse_rdf_list`).
- **L0012** (Info): `shape.deactivated == true`.

- [ ] **Step 1: Failing tests** — table-driven:

```rust
use shacl_rust::diagnostics::lint_shapes;
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::parse_shapes;

fn lint_codes(shapes_ttl: &str) -> Vec<&'static str> {
    let g = read_graph_from_string(shapes_ttl, "turtle").unwrap();
    let shapes = parse_shapes(&g).unwrap_or_default();
    lint_shapes(&g, &shapes).iter().map(|d| d.code).collect()
}

#[test]
fn lint_rules_fire() {
    let cases: &[(&str, &str)] = &[
        ("L0001", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:minCount 1 ] ."),
        ("L0002", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:pattern \"[unclosed\" ] ."),
        ("L0004", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCont 1 ] ."),
        ("L0005", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:minCount 1 ."),
        ("L0006", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCount 3 ; sh:maxCount 1 ] ."),
        ("L0008", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:in () ] ."),
        ("L0010", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:ignoredProperties ( rdf:type ) ."),
        ("L0011", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 1 ] ."),
        ("L0012", "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
          ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:deactivated true ."),
    ];
    for (code, ttl) in cases {
        let codes = lint_codes(ttl);
        assert!(codes.contains(code), "expected {code}, got {codes:?}");
    }
}

#[test]
fn clean_shapes_produce_no_lints() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:maxCount 5 ] .";
    assert!(lint_codes(ttl).is_empty(), "{:?}", lint_codes(ttl));
}

#[test]
fn did_you_mean_suggests_close_term() {
    let ttl = "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:p ; sh:minCont 1 ] .";
    let g = read_graph_from_string(ttl, "turtle").unwrap();
    let shapes = parse_shapes(&g).unwrap();
    let diags = lint_shapes(&g, &shapes);
    let l4 = diags.iter().find(|d| d.code == "L0004").unwrap();
    assert!(l4.help.as_deref().unwrap().contains("minCount"), "{:?}", l4.help);
}
```

Also add L0003/L0007/L0009 cases to `lint_rules_fire` following the same pattern (SPARQL with syntax error `"SELECT $this WHERE {"`, `sh:datatype "notAnIri"`, `sh:minInclusive ex:NotALiteral`).

- [ ] **Step 2: Run** — FAIL. **Step 3:** implement rules + `sh::all_terms()`. **Step 4:** PASS + full suite (watch: existing test fixtures under `tests/resources/` are *not* linted by any existing test — no interference). **Step 5: Commit** — `feat: shape linter with 12 rules`

---

### Task 7: CLI — `--diagnostics`, `--skip-lint`, `--deny-warnings`, `lint`, `explain`

**Files:**
- Modify: `crates/shacl-cli/src/main.rs`
- Test: manual verification commands (CLI is thin; library carries the logic and its tests)

**Interfaces:**
- Consumes: `diagnostics::{from_report, lint_shapes, render_text, render_ndjson, sort_diagnostics, entry, DiagnosticSeverity}`.
- Produces: CLI surface per spec.

Changes:
1. `Validate` variant gains:
```rust
        /// Diagnostics output on stderr: text, json (NDJSON), or none
        #[arg(long, default_value = "text")]
        diagnostics: String,
        /// Skip shape lints during validation
        #[arg(long)]
        skip_lint: bool,
        /// Exit with code 2 when shape lints report warnings or errors
        #[arg(long)]
        deny_warnings: bool,
```
2. New variants:
```rust
    /// Lint a shapes graph without validating data
    Lint {
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        #[arg(long, default_value = "text")]
        diagnostics: String,
    },
    /// Explain a diagnostic code (e.g. V0007, L0002)
    Explain {
        #[arg(value_name = "CODE")]
        code: String,
    },
```
3. In `validate_command` (both backends — factor a helper `emit_diagnostics(diags: &[Diagnostic], mode: &str)` writing to stderr via `eprint!`): after shapes parse and before/after validation:
```rust
    let mut all_diags = Vec::new();
    if !skip_lint {
        all_diags.extend(shacl_rust::diagnostics::lint_shapes(
            validation_dataset.shapes_graph(),
            &shapes,
        ));
    }
    let report = validate(&validation_dataset, &shapes);
    all_diags.extend(shacl_rust::diagnostics::from_report(
        &report,
        &validation_dataset,
        &shapes,
    ));
    shacl_rust::diagnostics::sort_diagnostics(&mut all_diags);
    match diagnostics_mode {
        "json" => eprint!("{}", shacl_rust::diagnostics::render_ndjson(&all_diags)),
        "none" => {}
        _ => eprint!("{}", shacl_rust::diagnostics::render_text(&all_diags, std::io::IsTerminal::is_terminal(&std::io::stderr()) && std::env::var_os("NO_COLOR").is_none())),
    }
    let lint_warnings = all_diags.iter().any(|d| d.code.starts_with('L'));
    // after report emission, before conformance exit:
    if deny_warnings && lint_warnings {
        std::process::exit(2);
    }
```
   (Exit-code precedence: deny-warnings exit 2 wins over conformance exit 1.)
4. `Lint` command handler: read shapes, `parse_shapes`, `lint_shapes`, render to stderr, exit 1 iff any diag has `DiagnosticSeverity::Error`.
5. `Explain` handler: `entry(&code)` → print `code`, title, `component:` line when present, `spec:` line, blank line, explanation, `== failing example ==` block, `== fix ==` block; unknown code → stderr message, exit 1.

- [ ] **Step 1:** implement (CLI has no test harness; the library functions are already tested).
- [ ] **Step 2: Verify manually:**
```bash
cargo build --release -p shacl-cli
B=/tmp/diagcheck; mkdir -p $B
printf '@prefix ex: <http://example.org/> .\n@prefix sh: <http://www.w3.org/ns/shacl#> .\nex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:age ; sh:minInclusive 0 ; sh:pattern "[bad" ] .\n' > $B/shapes.ttl
printf '@prefix ex: <http://example.org/> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\nex:a a ex:Person ; ex:age "-3"^^xsd:integer .\n' > $B/data.ttl
./target/release/shacl-validator validate $B/shapes.ttl $B/data.ttl -o /dev/null           # expect L0002 + V0007 on stderr, exit 1
./target/release/shacl-validator validate $B/shapes.ttl $B/data.ttl -o /dev/null --diagnostics json 2>&1 | head -2
./target/release/shacl-validator validate $B/shapes.ttl $B/data.ttl -o /dev/null --skip-lint      # V0007 only
./target/release/shacl-validator lint $B/shapes.ttl; echo "exit=$?"                        # L0002, exit 1
./target/release/shacl-validator explain V0007 | head -5
./target/release/shacl-validator explain L0002 | head -5
```
Also confirm stdout report unchanged: `./target/release/shacl-validator validate $B/shapes.ttl $B/data.ttl 2>/dev/null | head -3` shows the classic report.
- [ ] **Step 3:** `cargo test` + clippy + fmt green.
- [ ] **Step 4: Commit** — `feat(shacl-cli): diagnostics output, lint and explain subcommands`

---

### Task 8: `why` conformance tracer

**Files:**
- Replace: `src/diagnostics/explain_pass.rs`
- Modify: `src/validation/mod.rs` (visibility only: `fn resolve_target` → `pub(crate) fn resolve_target`; `Shape::get_value_nodes` → `pub(crate) fn get_value_nodes`)
- Modify: `crates/shacl-cli/src/main.rs` (add `Why` variant + handler)
- Test: `tests/diagnostics_why.rs`

**Interfaces:**
- Consumes: `resolve_target(validation_dataset, target)`, `Shape::get_value_nodes(dataset, focus)`, `Shape::validate` (per-shape public validate exists) — plus model/renderers.
- Produces: `pub fn explain_conformance<'a>(dataset: &'a ValidationDataset, shapes: &'a [Shape<'a>], focus: TermRef<'a>, shape_filter: Option<NamedOrBlankNodeRef<'a>>) -> Vec<Diagnostic>`.

Algorithm per shape (skipping non-matching when `shape_filter` is `Some`):
1. Targeting: for each `shape.targets`, resolve via `resolve_target`; if focus ∈ set → note `"selected by {target}"`. If shape has targets but none selected → one Diagnostic `{verdict: NotTargeted, severity: Info, title: "focus node is not targeted by {shape}"}` with notes listing each target and `help: "add the focus node to a target, e.g. give it rdf:type matching sh:targetClass"`; continue to next shape. Shapes with no targets at all → skip silently (unless `shape_filter` names them — then trace anyway with note `"shape has no targets; evaluated directly"`).
2. For a targeted (or filtered) shape: run `shape.validate_focus_node`-equivalent by calling the public `Shape::validate` against a report… simplest correct route: build a one-shape report via existing public API: `let mut report = ValidationReport::new(); /* need a pub(crate) validate_focus_node */`. Promote `Shape::validate_focus_node` to `pub(crate)` as well and call it directly with a fresh report.
3. Per constraint in `shape.constraints` and each property shape: resolve value nodes via `get_value_nodes`; emit one Info Diagnostic per (shape, constraint):
   - value nodes empty AND constraint is not MinCount → `verdict: Vacuous`, title `"{constraint} vacuously conforms"`, note `"path {path} resolved to 0 value nodes"`, help `"add sh:minCount 1 to {shape} if values are required"`.
   - otherwise, if the focus-node report contains a result matching (this shape or property shape, this constraint's component) → `verdict: Violates`, title `"{constraint} is violated"`, reuse the matching result's message as note.
   - else → `verdict: Conforms`, title `"{constraint} conforms"`, note listing the value nodes checked (up to 5, then `"… and N more"`).
4. Constraint naming in titles: `Constraint`'s existing `Display` impl.
5. Sort output.

CLI `Why` variant:
```rust
    /// Explain why a focus node does or does not fail shapes
    Why {
        #[arg(value_name = "SHAPES_FILE")]
        shapes_file: PathBuf,
        #[arg(value_name = "DATA_FILE")]
        data_file: PathBuf,
        /// Focus node IRI (angle brackets optional)
        #[arg(long)]
        focus: String,
        /// Restrict to one shape IRI
        #[arg(long)]
        shape: Option<String>,
        #[arg(long, default_value = "text")]
        diagnostics: String,
    },
```
Handler: load graphs, parse shapes, trim optional `<...>` from `--focus`/`--shape`, build `NamedNode::new(iri)` (error message on invalid IRI), locate the canonical focus term via `dataset.data().canonical_term(...)` (fall back to the constructed term if absent — still useful for NotTargeted), call `explain_conformance`, render to **stdout** (the trace *is* the product here), exit 0.

- [ ] **Step 1: Failing tests** in `tests/diagnostics_why.rs` covering all four verdicts:

```rust
use oxigraph::model::NamedNodeRef;
use shacl_rust::diagnostics::{explain_conformance, Verdict};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;
use shacl_rust::parse_shapes;

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:age ; sh:minInclusive 0 ; ] .
"#;

fn run(data: &str, focus: &str) -> Vec<(Option<Verdict>, String)> {
    let dg = read_graph_from_string(data, "turtle").unwrap();
    let sg = read_graph_from_string(SHAPES, "turtle").unwrap();
    let shapes = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(dg, sg.clone()).unwrap();
    let focus_nn = NamedNodeRef::new(focus).unwrap();
    let focus_term = dataset
        .data()
        .canonical_term(focus_nn.into())
        .expect("focus in data");
    // Leak the dataset/shapes borrow scope by asserting inside:
    explain_conformance(&dataset, &shapes, focus_term, None)
        .into_iter()
        .map(|d| (d.verdict, d.title))
        .collect()
}

#[test]
fn violating_node_traces_violates() {
    let data = "@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a a ex:Person ; ex:age \"-3\"^^xsd:integer .";
    let out = run(data, "http://example.org/a");
    assert!(out.iter().any(|(v, _)| *v == Some(Verdict::Violates)), "{out:?}");
}

#[test]
fn conforming_node_traces_conforms() {
    let data = "@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a a ex:Person ; ex:age \"3\"^^xsd:integer .";
    let out = run(data, "http://example.org/a");
    assert!(out.iter().any(|(v, _)| *v == Some(Verdict::Conforms)), "{out:?}");
}

#[test]
fn untargeted_node_traces_not_targeted() {
    let data = "@prefix ex: <http://example.org/> .
        ex:a ex:age 3 .";
    let out = run(data, "http://example.org/a");
    assert!(out.iter().any(|(v, _)| *v == Some(Verdict::NotTargeted)), "{out:?}");
}

#[test]
fn missing_path_traces_vacuous() {
    let data = "@prefix ex: <http://example.org/> .
        ex:a a ex:Person .";
    let out = run(data, "http://example.org/a");
    assert!(out.iter().any(|(v, _)| *v == Some(Verdict::Vacuous)), "{out:?}");
}
```

(Note: the closure borrow structure above returns owned data, so no lifetime issues; if the compiler objects to `focus_term` borrowing `dataset` inside `run`, inline the body into each test — mechanical.)

- [ ] **Step 2: Run** — FAIL. **Step 3:** visibility promotions + implement + CLI wiring; manual check:
```bash
./target/release/shacl-validator why $B/shapes.ttl $B/data.ttl --focus http://example.org/a
```
- [ ] **Step 4:** all tests + clippy + fmt + wasm check green. **Step 5: Commit** — `feat: why subcommand explaining conformance per focus node`

---

### Task 9: Python bindings `diagnostics=True`

**Files:**
- Modify: `crates/shacl-py/src/lib.rs`
- Test: manual (build `.so`, run snippet — same procedure as previous py work)

**Interfaces:**
- Consumes: `diagnostics::{from_report, lint_shapes, sort_diagnostics, diagnostic_to_json}`.
- Produces: `validate(..., diagnostics=False)` / `validate_file(..., diagnostics=False)` — when true, returned dict gains `"diagnostics"`: list of dicts (lint first, then violations — achieved by the standard sort, L < V lexically).

Implementation: add `diagnostics: bool` (default false) to both signatures' `#[pyo3(signature = ...)]` and plumb a flag through `run_validation`, which becomes returning `(serde_json::Value, Option<serde_json::Value>)` — the second element a `serde_json::Value::Array` of `diagnostic_to_json` values built while the dataset/shapes are still alive:

```rust
fn run_validation(
    dataset: &ValidationDataset,
    want_diagnostics: bool,
) -> Result<serde_json::Value, ShaclError> {
    let parsed_shapes = parse_shapes(dataset.shapes_graph())?;
    let report = shacl_rust_core::validate(dataset, &parsed_shapes);
    let mut json = report.as_json();
    if want_diagnostics {
        let mut diags =
            shacl_rust_core::diagnostics::lint_shapes(dataset.shapes_graph(), &parsed_shapes);
        diags.extend(shacl_rust_core::diagnostics::from_report(
            &report,
            dataset,
            &parsed_shapes,
        ));
        shacl_rust_core::diagnostics::sort_diagnostics(&mut diags);
        json["diagnostics"] = serde_json::Value::Array(
            diags
                .iter()
                .map(shacl_rust_core::diagnostics::diagnostic_to_json)
                .collect(),
        );
    }
    Ok(json)
}
```
(Adapt call sites; `conforms` helper passes `false`.)

- [ ] **Step 1:** implement. **Step 2: Verify:**
```bash
cargo build --release -p shacl-py
mkdir -p "$CLAUDE_JOB_DIR/tmp/pytest" && cp target/release/libshacl_rust.so "$CLAUDE_JOB_DIR/tmp/pytest/shacl_rust.so"
cd "$CLAUDE_JOB_DIR/tmp/pytest" && python3 -c "
import shacl_rust
shapes='''@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ; sh:targetClass ex:P ; sh:property [ sh:path ex:age ; sh:minInclusive 0 ] .'''
data='''@prefix ex: <http://example.org/> . @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:a a ex:P ; ex:age \"-3\"^^xsd:integer .'''
r = shacl_rust.validate(data, shapes, diagnostics=True)
assert r['conforms'] is False
assert r['diagnostics'][0]['code'].startswith(('L','V')), r['diagnostics']
assert any(d['code']=='V0007' for d in r['diagnostics'])
r2 = shacl_rust.validate(data, shapes)
assert 'diagnostics' not in r2
print('PY DIAGNOSTICS OK')"
```
- [ ] **Step 3:** full checks green. **Step 4: Commit** — `feat(shacl-py): diagnostics list in validate output`

---

### Task 10: Final verification and docs

**Files:**
- Modify: `README.md` (add a "Diagnostics" section: one terminal example (the V0007 layout), the flag table, `lint`/`explain`/`why` one-liners)
- Modify: `crates/shacl-py/README.md` (diagnostics kwarg example)

- [ ] **Step 1:** README sections (copy the reference rendering from Task 3's golden file).
- [ ] **Step 2:** Full gate: `cargo test` (all suites incl. 120/120 conformance + report fidelity), `cargo clippy --all-targets --all-features` (0 warnings), `cargo fmt --all -- --check`, `cargo check -p shacl-wasm --target wasm32-unknown-unknown`, stress test `cargo test --test stress --release`.
- [ ] **Step 3:** Confirm stdout-report byte-compatibility: run any pre-existing fixture through `validate` with `--diagnostics none` and diff stdout against the pre-branch binary if available; at minimum assert the report tests unchanged.
- [ ] **Step 4: Commit** — `docs: diagnostics usage in READMEs`
- [ ] **Step 5:** Do NOT push. Summarize per-task commits for review.

## Self-review

- **Spec coverage:** model+codes (T1/T2), renderers (T3/T4), derivation+snippets+component-in-output (T5), linter 12 rules + severity policy (T6), CLI flags/subcommands/stderr/exit codes (T7), why with 4 verdicts + vacuous detection (T8), Python (T9), docs+gates (T10). Registry explain includes component (T2 fields + T7 explain handler). Non-goals untouched. ✓
- **Placeholders:** registry entries are enumerated with exact field templates and content rules rather than full prose for all 40 (the table carries titles/components/spec-refs; explanations follow the V0007 exemplar) — the completeness test makes omission a build failure, so nothing can silently be skipped. ✓
- **Type consistency:** `Diagnostic` field set identical across T1 (definition), T3/T4 (renderers), T5/T6/T8 (producers), T9 (`diagnostic_to_json`). `render_text(&[Diagnostic], bool)`, `render_ndjson(&[Diagnostic])`, `from_report(&ValidationReport, &ValidationDataset, &[Shape])`, `lint_shapes(&Graph, &[Shape])`, `explain_conformance(&ValidationDataset, &[Shape], TermRef, Option<NamedOrBlankNodeRef>)` used consistently. ✓
