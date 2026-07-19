# shacl-rust Performance Improvements Implementation Plan

> **STATUS: COMPLETED 2026-07-19.** All 9 tasks were implemented, verified (full test suite incl. 120/120 W3C conformance, clippy, wasm), and committed directly in-session — no separate executor needed. Benchmark results and follow-up work (sh:sparql prepared queries, comparison pre-parsing, sh:class subclass fix) are recorded in `2026-07-19-bench-baseline.txt`. The "Out of scope" items at the bottom were either since implemented (1) or investigated and skipped with rationale in the same file (2–4); item 5 was implemented as a correctness fix, and 6–7 were implemented in the follow-up round.

**Goal:** Remove the biggest measured hot spots in the shacl-rust SHACL validator: eager oxigraph store construction, per-node regex compilation, redundant O(n²) scans, per-value linear lookups, per-target SPARQL binding rebuilds, and allocation churn in property-path evaluation.

**Architecture:** All changes are behavior-preserving refactors inside the existing modules. The W3C conformance suite (`tests/conformance.rs`) is the correctness gate for every task; a new criterion benchmark added in Task 2 measures the wins. No public API changes except `ValidationDataset::store()` gaining a `Result` return (one internal caller).

**Tech Stack:** Rust 2021, oxigraph 0.5.5, regex 1.10, rayon 1.10, criterion 0.5 (already in dev-dependencies).

## Global Constraints

- Behavior must not change: after every task, `cargo test` must pass with the same results as before the task.
- The wasm build must keep compiling: `cargo check -p shacl-wasm --target wasm32-unknown-unknown` (the target is installed) must pass after any task that touches `src/validation/dataset.rs`, `src/core/`, or `src/validation/`.
- No new dependencies. Do not add crates to any `Cargo.toml` `[dependencies]` section.
- Commit after every task with the exact commit message given in the task (repo uses conventional commits: `perf:`, `chore:`, `test:`).
- If a step's verification fails, stop and fix within the task before moving on. Never commit a failing state.
- Line numbers in this plan refer to the state of the file *before* that task's edits. Earlier tasks may shift later line numbers slightly — match on the quoted code, not the line number.

## Task Order

Tasks 1–2 set up measurement. Tasks 3–9 are the fixes, ordered easiest-first. Each task is independent; if one gets stuck, skip it, leave its checkboxes unchecked, and continue with the next.

---

### Task 1: Release profile optimization flags

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Interfaces:**
- Consumes: nothing
- Produces: nothing (build configuration only)

- [x] **Step 1: Add the profile section**

In the root `Cargo.toml`, add this block after the `[workspace]` section (after the line `members = ["crates/shacl-cli", "crates/shacl-wasm", "crates/shacl-mcp"]`):

```toml
[profile.release]
lto = "thin"
codegen-units = 1
```

- [x] **Step 2: Verify release build works**

Run: `cargo build --release`
Expected: compiles with no errors (warnings are OK if they existed before).

- [x] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "perf: enable thin LTO and single codegen unit for release builds"
```

---

### Task 2: Criterion benchmark harness

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `benches/validation.rs`

**Interfaces:**
- Consumes: public API `shacl_rust::rdf::read_graph_from_string(&str, &str) -> Result<Graph, ShaclError>`, `shacl_rust::parse_shapes(&Graph) -> Result<Vec<Shape>, ShaclError>`, `shacl_rust::validation::dataset::ValidationDataset::from_graphs(Graph, Graph) -> Result<ValidationDataset, ShaclError>`, `shacl_rust::validation::validate(&ValidationDataset, &[Shape]) -> ValidationReport`
- Produces: benchmark names `full_pipeline_100`, `full_pipeline_1000`, `validate_only_100`, `validate_only_1000` used to compare before/after in later tasks.

- [x] **Step 1: Register the bench target**

In the root `Cargo.toml`, add at the end of the file:

```toml
[[bench]]
name = "validation"
harness = false
```

- [x] **Step 2: Write the benchmark**

Create `benches/validation.rs` with exactly:

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::{parse_shapes, validation};
use std::hint::black_box;

fn generate_data(num_persons: usize) -> String {
    let mut ttl = String::from(
        "@prefix ex: <http://example.org/> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    );
    for i in 0..num_persons {
        ttl.push_str(&format!(
            "ex:person{i} rdf:type ex:Person ;\n\
             \tex:name \"Person {i}\" ;\n\
             \tex:age \"{}\"^^xsd:integer ;\n\
             \tex:email \"person{i}@example.org\" ;\n\
             \tex:knows ex:person{} .\n",
            20 + (i % 60),
            (i + 1) % num_persons,
        ));
    }
    ttl
}

const SHAPES: &str = r#"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
        sh:path ex:name ;
        sh:minCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path ex:age ;
        sh:datatype xsd:integer ;
        sh:minInclusive 0 ;
        sh:maxInclusive 150 ;
    ] ;
    sh:property [
        sh:path ex:email ;
        sh:pattern "^[^@]+@[^@]+\\.[a-z]+$" ;
    ] ;
    sh:property [
        sh:path ex:knows ;
        sh:class ex:Person ;
    ] .
"#;

fn bench_validation(c: &mut Criterion) {
    for &size in &[100usize, 1000] {
        let data_ttl = generate_data(size);
        let data_graph = read_graph_from_string(&data_ttl, "turtle").unwrap();
        let shapes_graph = read_graph_from_string(SHAPES, "turtle").unwrap();

        // Measures parsing + dataset construction + validation.
        c.bench_function(&format!("full_pipeline_{size}"), |b| {
            b.iter(|| {
                let shapes = parse_shapes(&shapes_graph).unwrap();
                let dataset = validation::dataset::ValidationDataset::from_graphs(
                    data_graph.clone(),
                    shapes_graph.clone(),
                )
                .unwrap();
                let report = validation::validate(&dataset, &shapes);
                black_box(report.get_results().len())
            })
        });

        // Measures validation only, with shapes and dataset prepared once.
        let shapes = parse_shapes(&shapes_graph).unwrap();
        let dataset = validation::dataset::ValidationDataset::from_graphs(
            data_graph.clone(),
            shapes_graph.clone(),
        )
        .unwrap();
        c.bench_function(&format!("validate_only_{size}"), |b| {
            b.iter(|| {
                let report = validation::validate(&dataset, &shapes);
                black_box(report.get_results().len())
            })
        });
    }
}

criterion_group!(benches, bench_validation);
criterion_main!(benches);
```

- [x] **Step 3: Run the benchmark and record the baseline**

Run: `cargo bench --bench validation`
Expected: all four benchmarks run and print timings. Copy the four median times into a new file `docs/superpowers/plans/2026-07-19-bench-baseline.txt` (plain text, one line per benchmark). Criterion also stores results in `target/criterion/` and will print change percentages on later runs automatically.

Note: the benchmark data conforms to the shapes (0 violations expected), so this measures the conforming fast path — which is the common production case.

- [x] **Step 4: Make sure the bench file is not published with the crate**

In the root `Cargo.toml`, the `exclude` list under `[package]` must contain `"benches/*"`. Add it if missing:

```toml
exclude = [
    "tests/*",
    "web/*",
    ".vscode/*",
    "examples/*",
    ".github/*",
    ".devcontainer/*",
    "benches/*",
]
```

Note: because the bench is excluded from the published crate, the `[[bench]]` section must stay — cargo needs it locally, and `cargo publish` validates the manifest against the packaged file list. If `cargo publish --dry-run` is part of CI and complains about the missing bench file, add `bench = false` under `[lib]` instead of excluding, and report this in your summary.

- [x] **Step 5: Verify the test suite still passes**

Run: `cargo test`
Expected: same pass/fail results as on `main` before this task (all green).

- [x] **Step 6: Commit**

```bash
git add Cargo.toml benches/validation.rs docs/superpowers/plans/2026-07-19-bench-baseline.txt
git commit -m "test: add criterion benchmark for validation pipeline"
```

---

### Task 3: Remove O(n²) scans in subclass/subproperty traversal

**Files:**
- Modify: `src/utils.rs:26-28` and `src/utils.rs:100-102`

**Interfaces:**
- Consumes: nothing new
- Produces: unchanged signatures `is_subclass_of`, `is_subproperty_of`

Both functions do a BFS with `while let Some(current) = to_visit.pop()`. Inside the loop they call `to_visit.contains(&class)` — a linear scan of the frontier on every iteration, making the traversal O(V²). The scan is redundant: the `if current == class { return true; }` check at the top of the loop already detects the target when it is popped.

- [x] **Step 1: Delete the redundant scan in `is_subclass_of`**

In `src/utils.rs`, inside `is_subclass_of`, delete these three lines (they appear right before the closing `}` of the `while` loop):

```rust
        if to_visit.contains(&class) {
            return true;
        }
```

- [x] **Step 2: Delete the redundant scan in `is_subproperty_of`**

In `src/utils.rs`, inside `is_subproperty_of`, delete these three lines (same shape, referencing `property`):

```rust
        if to_visit.contains(&property) {
            return true;
        }
```

- [x] **Step 3: Verify**

Run: `cargo test`
Expected: all tests pass, same as baseline.

- [x] **Step 4: Commit**

```bash
git add src/utils.rs
git commit -m "perf: remove redundant O(n^2) frontier scans in subclass/subproperty traversal"
```

---

### Task 4: Direct indexed rdf:type lookup in sh:class

**Files:**
- Modify: `src/validation/constraints/class.rs:25-27`

**Interfaces:**
- Consumes: `Graph::objects_for_subject_predicate` (oxigraph, already used elsewhere in this repo, e.g. `src/utils.rs:19`)
- Produces: unchanged `Validate` impl for `ClassConstraint`

The current membership test iterates *every* triple of the value node and filters for `rdf:type` in code. Oxigraph has a direct (subject, predicate) index lookup.

- [x] **Step 1: Replace the scan with an indexed lookup**

In `src/validation/constraints/class.rs`, replace:

```rust
                let is_instance = data_graph
                    .triples_for_subject(value_as_node)
                    .any(|triple| triple.predicate == TYPE && triple.object == self.0.into());
```

with:

```rust
                let is_instance = data_graph
                    .objects_for_subject_predicate(value_as_node, TYPE)
                    .any(|object| object == self.0.into());
```

- [x] **Step 2: Verify**

Run: `cargo test`
Expected: all tests pass, same as baseline (this is a pure lookup-strategy change; the matched set is identical).

- [x] **Step 3: Commit**

```bash
git add src/validation/constraints/class.rs
git commit -m "perf: use indexed rdf:type lookup in sh:class instead of scanning all subject triples"
```

Note for the summary report: `sh:class` currently matches only the *direct* `rdf:type`, ignoring `rdfs:subClassOf` — that is a spec-compliance question, deliberately out of scope for this perf plan. Mention it in your final summary so the maintainer can decide.

---

### Task 5: HashSet membership for sh:in

**Files:**
- Modify: `src/validation/constraints/sh_in.rs`

**Interfaces:**
- Consumes: `InConstraint<'a>(pub Vec<TermRef<'a>>)` from `src/core/constraints.rs:83` (unchanged)
- Produces: unchanged `Validate` impl for `InConstraint`

`self.0.contains(&value_node)` is a linear scan per value node — O(values × list length). Build a `HashSet` once per call instead. (`TermRef` is `Copy + Eq + Hash`.)

- [x] **Step 1: Replace the linear scan**

In `src/validation/constraints/sh_in.rs`, add to the imports at the top:

```rust
use std::collections::HashSet;
```

Then inside `validate`, replace:

```rust
        let mut violations = Vec::new();

        for &value_node in value_nodes {
            if !self.0.contains(&value_node) {
```

with:

```rust
        let mut violations = Vec::new();
        let allowed: HashSet<TermRef<'a>> = self.0.iter().copied().collect();

        for &value_node in value_nodes {
            if !allowed.contains(&value_node) {
```

- [x] **Step 2: Verify**

Run: `cargo test`
Expected: all tests pass, same as baseline.

- [x] **Step 3: Commit**

```bash
git add src/validation/constraints/sh_in.rs
git commit -m "perf: use HashSet membership test in sh:in constraint"
```

---

### Task 6: Compile sh:pattern regex once at parse time

**Files:**
- Modify: `src/core/constraints.rs:55-59` (struct) and the file's imports
- Modify: `src/parser/constraints/pattern.rs:16-24`
- Modify: `src/validation/constraints/pattern.rs:20-42`

**Interfaces:**
- Consumes: nothing new (`regex` is already a dependency)
- Produces: `PatternConstraint` gains field `pub compiled: Option<regex::Regex>` and constructor `PatternConstraint::new(pattern: String, flags: Option<String>) -> PatternConstraint`. Equality (`PartialEq`/`Eq`) compares only `pattern` and `flags`.

Today `Regex::new` runs inside `validate()`, i.e. once per focus node. Regex compilation is orders of magnitude more expensive than matching. Move compilation to parse time. Preserve the existing behavior for invalid regexes: they produce **no violations** (the `Ok(violations)` early-return), so an invalid pattern compiles to `compiled: None` and `validate` returns empty.

`regex::Regex` does not implement `PartialEq`/`Eq`, and `Constraint` derives both — so `PatternConstraint` needs manual impls that ignore the compiled field.

- [x] **Step 1: Change the struct and add the constructor**

In `src/core/constraints.rs`, replace:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternConstraint {
    pub pattern: String,
    pub flags: Option<String>,
}
```

with:

```rust
#[derive(Debug, Clone)]
pub struct PatternConstraint {
    pub pattern: String,
    pub flags: Option<String>,
    /// Compiled at parse time; `None` if `pattern` is not a valid regex.
    pub compiled: Option<regex::Regex>,
}

impl PatternConstraint {
    pub fn new(pattern: String, flags: Option<String>) -> Self {
        let regex_pattern = if let Some(ref f) = flags {
            let mut pattern_with_flags = String::from("(?");
            if f.contains('i') {
                pattern_with_flags.push('i');
            }
            if f.contains('m') {
                pattern_with_flags.push('m');
            }
            if f.contains('s') {
                pattern_with_flags.push('s');
            }
            pattern_with_flags.push(')');
            pattern_with_flags.push_str(&pattern);
            pattern_with_flags
        } else {
            pattern.clone()
        };
        let compiled = regex::Regex::new(&regex_pattern).ok();
        Self {
            pattern,
            flags,
            compiled,
        }
    }
}

impl PartialEq for PatternConstraint {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.flags == other.flags
    }
}

impl Eq for PatternConstraint {}
```

(The flag-translation logic is moved verbatim from `src/validation/constraints/pattern.rs:22-38` — including the existing quirk that `flags: Some("")` produces the invalid pattern `(?)…` and therefore `compiled: None`. Keep that behavior.)

- [x] **Step 2: Use the constructor in the parser**

In `src/parser/constraints/pattern.rs`, replace:

```rust
            Ok(vec![Constraint::Pattern(PatternConstraint {
                pattern,
                flags,
            })])
```

with:

```rust
            Ok(vec![Constraint::Pattern(PatternConstraint::new(
                pattern, flags,
            ))])
```

- [x] **Step 3: Use the precompiled regex in the validator**

In `src/validation/constraints/pattern.rs`, delete the import `use regex::Regex;` and replace everything from `let regex_pattern = if let Some(ref f) = self.flags {` through `let Ok(re) = Regex::new(&regex_pattern) else {\n            return Ok(violations);\n        };` (lines 22-42) with:

```rust
        let Some(re) = self.compiled.as_ref() else {
            return Ok(violations);
        };
```

The rest of the function (the `for &value_node in value_nodes` loop using `re.is_match`) stays unchanged.

- [x] **Step 4: Check for other construction sites**

Run: `grep -rn "PatternConstraint {" src/ tests/ crates/`
Expected: no matches outside `src/core/constraints.rs` (the struct definition and constructor). If a match appears, convert it to `PatternConstraint::new(...)`.

- [x] **Step 5: Verify**

Run: `cargo test`
Expected: all tests pass, same as baseline.

Run: `cargo bench --bench validation`
Expected: `validate_only_*` benchmarks improve (the benchmark shapes include a `sh:pattern`); criterion prints the change vs. the stored baseline.

- [x] **Step 6: Commit**

```bash
git add src/core/constraints.rs src/parser/constraints/pattern.rs src/validation/constraints/pattern.rs
git commit -m "perf: compile sh:pattern regex once at parse time instead of per focus node"
```

---

### Task 7: Build the oxigraph store lazily, in one transaction

**Files:**
- Modify: `src/validation/dataset.rs` (whole file, replacement below)
- Modify: `src/validation/constraints/sparql.rs:75`

**Interfaces:**
- Consumes: `Store::extend` (oxigraph, single-transaction bulk insert), `std::sync::OnceLock`
- Produces: `ValidationDataset::store()` now returns `Result<Arc<Store>, ShaclError>` (was `Arc<Store>`). `from_graphs` keeps its signature. Everything else unchanged.

`from_graphs` currently copies the entire data graph and shapes graph into an oxigraph `Store` one quad at a time (each insert is a full transaction). That store is read in exactly one place — `src/validation/constraints/sparql.rs:75` — so for shapes graphs without SPARQL constraints (the common case) the whole store build is wasted work and doubles memory. Fix both: build the store only on first `store()` call, and use one `extend` transaction instead of per-quad inserts.

- [x] **Step 1: Replace `src/validation/dataset.rs`**

Replace the entire file content with:

```rust
use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use oxigraph::{
    model::{Graph, GraphNameRef, NamedNodeRef, QuadRef},
    store::Store,
};

use crate::err::ShaclError;

pub const SHAPES_GRAPH_IRI: &str = "urn:shacl:shapes-graph";

#[derive(Clone)]
pub struct ValidationDataset {
    // Built lazily on first `store()` call: only SPARQL-based constraints read it,
    // and building it copies both graphs into the store. Shared across clones so
    // it is built at most once per dataset.
    store: Arc<OnceLock<Arc<Store>>>,
    data_graph: Graph,
    shapes_graph: Graph,
}

impl ValidationDataset {
    pub fn from_graphs(data_graph: Graph, shapes_graph: Graph) -> Result<Self, ShaclError> {
        Ok(Self {
            store: Arc::new(OnceLock::new()),
            data_graph,
            shapes_graph,
        })
    }

    pub fn store(&self) -> Result<Arc<Store>, ShaclError> {
        if let Some(store) = self.store.get() {
            return Ok(Arc::clone(store));
        }
        let built = Arc::new(Self::build_store(&self.data_graph, &self.shapes_graph)?);
        // Under contention another thread may have won the race; get_or_init
        // returns the stored value either way.
        Ok(Arc::clone(self.store.get_or_init(|| built)))
    }

    fn build_store(data_graph: &Graph, shapes_graph: &Graph) -> Result<Store, ShaclError> {
        let store = Store::new()
            .map_err(|e| ShaclError::Io(format!("Failed to create validation store: {}", e)))?;

        let shapes_graph_name = NamedNodeRef::new_unchecked(SHAPES_GRAPH_IRI);
        let quads = data_graph
            .iter()
            .map(|triple| {
                QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::DefaultGraph,
                )
            })
            .chain(shapes_graph.iter().map(|triple| {
                QuadRef::new(
                    triple.subject,
                    triple.predicate,
                    triple.object,
                    GraphNameRef::NamedNode(shapes_graph_name),
                )
            }));

        store.extend(quads).map_err(|e| {
            ShaclError::Io(format!(
                "Failed to load graphs into validation store: {}",
                e
            ))
        })?;

        Ok(store)
    }

    pub fn data_graph(&self) -> &Graph {
        &self.data_graph
    }

    pub fn shapes_graph(&self) -> &Graph {
        &self.shapes_graph
    }
}

impl Deref for ValidationDataset {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.data_graph
    }
}
```

- [x] **Step 2: Update the one caller of `store()`**

In `src/validation/constraints/sparql.rs`, replace:

```rust
        let store = validation_dataset.store();
```

with:

```rust
        let store = validation_dataset.store()?;
```

(The surrounding function returns `Result<Vec<ValidationResult<'a>>, ShaclError>`, so `?` works.)

- [x] **Step 3: Check for other callers**

Run: `grep -rn "\.store()" src/ crates/`
Expected: only the `sparql.rs` call and the definition in `dataset.rs`. If another caller appears, apply the same `?` (or propagate the error appropriately).

- [x] **Step 4: Verify native and wasm**

Run: `cargo test`
Expected: all tests pass — the conformance suite includes SPARQL-constraint tests, which exercises the lazy path.

Run: `cargo check -p shacl-wasm --target wasm32-unknown-unknown`
Expected: compiles (dataset.rs is shared with the wasm build).

Run: `cargo bench --bench validation`
Expected: `full_pipeline_*` benchmarks improve substantially (the benchmark shapes contain no SPARQL constraints, so the store is now never built).

- [x] **Step 5: Commit**

```bash
git add src/validation/dataset.rs src/validation/constraints/sparql.rs
git commit -m "perf: build SPARQL store lazily and load it in a single transaction"
```

---

### Task 8: Hoist constant SPARQL bindings out of the per-target loop

**Files:**
- Modify: `src/validation/constraints/sparql.rs:104-125`

**Interfaces:**
- Consumes: `utils::inject_values_bindings(query: &str, bindings: &[(String, String)])` — injects one `VALUES $var { value }` line per binding; binding *order* does not affect semantics (each variable is independent), so reordering `value` to the end is safe.
- Produces: unchanged `Validate` impl for `SparqlConstraint`

Inside `for maybe_value in run_once_targets`, the bindings for `this`, `shapesGraph`, `currentShape`, `PATH`, and all parameter bindings are rebuilt (with `format!` allocations) per target, though they are constant for the whole call. Only `value` varies. Build the constant part once.

- [x] **Step 1: Hoist the constant bindings**

In `src/validation/constraints/sparql.rs`, the loop currently begins:

```rust
        for maybe_value in run_once_targets {
            let mut bindings: Vec<(String, String)> = Vec::new();
            bindings.push(("this".to_string(), format!("{}", focus_node)));
            bindings.push((
                "shapesGraph".to_string(),
                format!("<{}>", dataset::SHAPES_GRAPH_IRI),
            ));
            bindings.push(("currentShape".to_string(), format!("{}", shape.node)));

            if let Some(value) = maybe_value {
                bindings.push(("value".to_string(), format!("{}", value)));
            }

            if let Some(path) = path {
                if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                    bindings.push(("PATH".to_string(), format!("{}", predicate)));
                }
            }

            for (name, value) in &self.parameter_bindings {
                bindings.push((name.to_string(), format!("{}", value)));
            }
```

Replace that with (the constant part moves *above* the loop):

```rust
        let mut base_bindings: Vec<(String, String)> = Vec::new();
        base_bindings.push(("this".to_string(), format!("{}", focus_node)));
        base_bindings.push((
            "shapesGraph".to_string(),
            format!("<{}>", dataset::SHAPES_GRAPH_IRI),
        ));
        base_bindings.push(("currentShape".to_string(), format!("{}", shape.node)));

        if let Some(path) = path {
            if let Some(predicate) = utils::extract_direct_predicates(path).into_iter().next() {
                base_bindings.push(("PATH".to_string(), format!("{}", predicate)));
            }
        }

        for (name, value) in &self.parameter_bindings {
            base_bindings.push((name.to_string(), format!("{}", value)));
        }

        for maybe_value in run_once_targets {
            let mut bindings = base_bindings.clone();

            if let Some(value) = maybe_value {
                bindings.push(("value".to_string(), format!("{}", value)));
            }
```

Everything after this point in the loop body (`let bound_query = utils::inject_values_bindings(...)` onward) stays unchanged.

- [x] **Step 2: Verify**

Run: `cargo test`
Expected: all tests pass — the conformance suite covers SPARQL constraints with parameters and paths.

- [x] **Step 3: Commit**

```bash
git add src/validation/constraints/sparql.rs
git commit -m "perf: build constant SPARQL bindings once per constraint call, not per target"
```

---

### Task 9: Cut allocation churn in property-path evaluation

**Files:**
- Modify: `src/core/path.rs:108-211` (replaces `resolve_path_for_given_node` and `resolve_element`)

**Interfaces:**
- Consumes: nothing new
- Produces: `resolve_path_for_given_node` keeps its exact signature. Private helper `resolve_element` is replaced by private associated fns `expand` and `expand_transitive`. (Verified: `resolve_element` has no callers outside this file.)

Today, `resolve_element` allocates a `subjects` Vec, a `results` Vec, and a dedup `HashSet` on **every** call — and the `ZeroOrMore`/`OneOrMore` arms call it once per BFS step with a single-element slice, so a transitive path allocates three collections per visited node. The rewrite below expands neighbors through a callback, deduplicates once per sequence step, and keeps a single visited set per transitive closure. Semantics are unchanged:
- `ZeroOrMore` emits the start node plus all transitively reachable nodes, each once.
- `OneOrMore` emits all transitively reachable nodes but never the start node itself — exactly as before: the start is pre-inserted into `visited`, so even a cycle back to it does not emit it.
- `ZeroOrOne` emits the node itself plus direct neighbors.
- `Alternative` emits the union of all alternatives.
- Results per sequence step are deduplicated (the old code deduped at the end of every `resolve_element` call; the new code dedupes in the sequence loop — same visible result).
- Literals are skipped as subjects, as before.

The doctest at the top of `src/core/path.rs` (lines 17-73) asserts on result sets via `len` + `contains` and stays valid; do not modify it.

- [x] **Step 1: Replace the two resolve methods**

In `src/core/path.rs`, replace everything from the line `/// Resolves the path for a given node in the graph, returning all reachable nodes.` (line 108) through the end of `resolve_element` (the `}` at line 211, just before the closing `}` of `impl<'a> Path<'a>`) with:

```rust
    /// Resolves the path for a given node in the graph, returning all reachable nodes.
    pub fn resolve_path_for_given_node(
        &self,
        graph: &'a oxigraph::model::Graph,
        node: &oxigraph::model::NamedOrBlankNodeRef<'a>,
    ) -> Vec<oxigraph::model::TermRef<'a>> {
        debug!("Resolving path for node {:?} with path: {}", node, self);
        let mut current_nodes: Vec<TermRef<'a>> = vec![(*node).into()];

        // Apply each path element in sequence, deduplicating the frontier per step.
        for element in &self.path {
            let mut next_nodes: Vec<TermRef<'a>> = Vec::new();
            let mut seen: HashSet<TermRef<'a>> = HashSet::new();
            for current in &current_nodes {
                let subject = match current {
                    TermRef::NamedNode(n) => NamedOrBlankNodeRef::from(*n),
                    TermRef::BlankNode(b) => NamedOrBlankNodeRef::from(*b),
                    TermRef::Literal(_) => continue,
                };
                Self::expand(graph, element, subject, &mut |term| {
                    if seen.insert(term) {
                        next_nodes.push(term);
                    }
                });
            }
            current_nodes = next_nodes;
        }
        debug!("Resolved nodes: {:?}", current_nodes);
        current_nodes
    }

    /// Emits every node reachable from `subject` via `element`. May emit
    /// duplicates; callers deduplicate.
    fn expand(
        graph: &'a oxigraph::model::Graph,
        element: &PathElement<'a>,
        subject: NamedOrBlankNodeRef<'a>,
        emit: &mut dyn FnMut(TermRef<'a>),
    ) {
        match element {
            PathElement::Iri(predicate) => {
                for object in graph.objects_for_subject_predicate(subject, *predicate) {
                    emit(object);
                }
            }
            PathElement::Inverse(predicate) => {
                for s in graph.subjects_for_predicate_object(*predicate, TermRef::from(subject)) {
                    emit(TermRef::from(s));
                }
            }
            PathElement::ZeroOrMore(inner) => {
                // Kleene star: include the starting node itself.
                emit(subject.into());
                Self::expand_transitive(graph, inner, subject, emit);
            }
            PathElement::OneOrMore(inner) => {
                Self::expand_transitive(graph, inner, subject, emit);
            }
            PathElement::ZeroOrOne(inner) => {
                emit(subject.into());
                Self::expand(graph, inner, subject, emit);
            }
            PathElement::Alternative(alternatives) => {
                for alt in alternatives {
                    Self::expand(graph, alt, subject, emit);
                }
            }
        }
    }

    /// Transitive closure of `element` starting at `start`, excluding `start`
    /// itself. Each reachable node is emitted exactly once.
    fn expand_transitive(
        graph: &'a oxigraph::model::Graph,
        element: &PathElement<'a>,
        start: NamedOrBlankNodeRef<'a>,
        emit: &mut dyn FnMut(TermRef<'a>),
    ) {
        let mut visited: HashSet<TermRef<'a>> = HashSet::new();
        visited.insert(start.into());
        let mut to_visit: Vec<NamedOrBlankNodeRef<'a>> = vec![start];

        while let Some(current) = to_visit.pop() {
            let mut found: Vec<TermRef<'a>> = Vec::new();
            Self::expand(graph, element, current, &mut |term| found.push(term));
            for next in found {
                if visited.insert(next) {
                    emit(next);
                    match next {
                        TermRef::NamedNode(n) => to_visit.push(NamedOrBlankNodeRef::from(n)),
                        TermRef::BlankNode(b) => to_visit.push(NamedOrBlankNodeRef::from(b)),
                        TermRef::Literal(_) => {}
                    }
                }
            }
        }
    }
```

- [x] **Step 2: Verify the path tests specifically, then the whole suite**

Run: `cargo test --test path`
Expected: PASS.

Run: `cargo test`
Expected: all tests pass, including the doctest in `src/core/path.rs` (doctests run as part of `cargo test`).

- [x] **Step 3: Benchmark**

Run: `cargo bench --bench validation`
Expected: no regression on `validate_only_*` (the benchmark uses simple single-IRI paths; the big wins here are on transitive paths, which the conformance suite covers for correctness).

- [x] **Step 4: Commit**

```bash
git add src/core/path.rs
git commit -m "perf: expand property paths via callback to avoid per-step Vec/HashSet allocations"
```

---

## Final verification (after all tasks)

- [x] Run `cargo test` — everything green.
- [x] Run `cargo check -p shacl-wasm --target wasm32-unknown-unknown` — compiles.
- [x] Run `cargo bench --bench validation` and append the final numbers to `docs/superpowers/plans/2026-07-19-bench-baseline.txt` under a `=== after ===` line.
- [x] Run `cargo clippy --all-targets` — no *new* warnings versus `main` (pre-existing warnings are out of scope).
- [x] Summarize in your final report: per-benchmark before/after medians, any task skipped and why, and the `sh:class` subclass-semantics note from Task 4.

## Out of scope (deliberately not tasks — needs design decisions or riskier surgery)

These were identified in the same performance review but are **not** part of this plan. Do not attempt them:

1. **Prepared SPARQL queries with variable pre-binding** — replacing textual `VALUES` injection (`utils::inject_values_bindings`) with oxigraph's `query_opt_with_substituted_variables` would parse each constraint query once instead of once per target, but changes the query-execution path and error surface.
2. **Per-shape caching of the sibling-qualified-shapes map** (`src/validation/mod.rs:227-254`) and the `sh:closed` allowed-property set (`src/validation/mod.rs:371`) — both are recomputed per focus node but depend only on the shape; caching needs a place to hang shape-level state.
3. **Memoizing value-node resolution per (focus node, path)** — the pair constraints (`equals`, `disjoint`, `lessThan`, `lessThanOrEquals`) re-resolve paths already resolved by `validate_focus_node`.
4. **Flattening the nested rayon parallelism** in `src/validation/mod.rs:64/113` into a single level with a `fold`/`reduce` instead of per-node `ValidationReport` merges.
5. **`rdfs:subClassOf`-aware `sh:class`** — correctness change, not perf.
6. **Parsing constant literal bounds once** — `compare_values` (`src/utils.rs:397`) re-parses both literals as `f64` per comparison; for `sh:minInclusive`/`sh:maxInclusive`/`sh:minExclusive`/`sh:maxExclusive` the bound is fixed at parse time. Needs struct changes across four constraint types.
7. **Lazy violation-string construction** — several constraints build `format!` details/messages on the conforming path (`sparql.rs`, `language_in.rs`, `sh_and.rs`), and `build_validation_result` (`src/validation/mod.rs:655`) clones each message twice.
