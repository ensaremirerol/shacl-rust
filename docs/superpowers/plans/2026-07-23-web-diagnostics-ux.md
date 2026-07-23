# Web Diagnostics UX Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the browser demo (`web/`) from a plain-text/JSON/RDF output box into a
diagnostics-first, click-to-drill-down UI that showcases rich violation diagnostics, shape
linting, `explain`, and `why` (including "why did this conforming node pass").

**Architecture:** Four new additive `wasm-bindgen` exports in `crates/shacl-wasm/src/lib.rs`
return JSON (built from the already-shipped `shacl_rust::diagnostics::*` API) instead of
pre-rendered text. A new pure-function rendering module (`web/diagnostics.js`) turns that JSON
into HTML strings via `escapeHtml()`-guarded template literals. `web/main.js` wires wasm calls to
DOM events (validate, code-badge clicks, focus-node-chip clicks) using event delegation. No
framework, no build step — vanilla DOM/template-string rendering, consistent with the existing
`main.js`.

**Tech Stack:** Rust (`shacl-rust` core + `shacl-wasm` crate, `wasm-bindgen`, `serde_json`),
vanilla JS ES modules, CodeMirror 6 (unchanged), `wasm-pack`.

## Global Constraints

- No new framework or build step for `web/`.
- No removal or signature change to any existing wasm export (`validate_graphs`,
  `validate_graphs_diagnostics`, `validate_graphs_conforms`, `lint_data_graph`,
  `lint_shapes_graph`) — new exports are additive only. `validate_graphs_diagnostics` becomes
  uncalled from the redesigned `web/main.js` (superseded by `validate_diagnostics_json`) but
  stays in `crates/shacl-wasm/src/lib.rs` unchanged — it is still part of the published wasm/npm
  API surface.
- MCP integration (`crates/shacl-mcp`) is explicitly deferred by the spec — do not implement it
  in this plan.
- New JSON payloads must reuse the existing `diagnostic_to_json` shape exactly: `code, severity
  ("error"|"warning"|"info"), title, constraint_component, snippets[{origin: "data"|"shapes",
  turtle, highlight, annotation}], expected, actual, notes[], help, focus_node, source_shape,
  path, verdict ("conforms"|"violates"|"not-targeted"|"vacuous"|null)`.
- All interpolated text that can contain user-pasted graph content (turtle, messages, notes,
  IRIs) must go through one shared `escapeHtml()` helper before insertion into HTML — never raw
  string concatenation.
- Rust verification gates (run at the end): `cargo test` (full suite, conformance 120/120),
  `cargo clippy --all-targets --all-features` (0 warnings), `cargo fmt --all --check`, `cargo
  check -p shacl-wasm --target wasm32-unknown-unknown`.
- No automated browser test suite exists or is being added; web verification is manual
  (`node --check` as a syntax gate, then a real-browser pass, Playwright-assisted).

---

## File Structure

- Modify: `src/diagnostics/explain_pass.rs` — add `pub fn shape_target_nodes`, appended at the
  end of the file (reuses the module's existing `resolve_target` import path and `Shape`/
  `ValidationDataset` imports already at the top of the file).
- Modify: `src/diagnostics/mod.rs` — re-export `shape_target_nodes`.
- Create: `tests/diagnostics_targets.rs` — integration tests for `shape_target_nodes`, mirroring
  the existing `tests/diagnostics_why.rs` pattern.
- Modify: `crates/shacl-wasm/src/lib.rs` — add four new `#[wasm_bindgen]` exports:
  `validate_diagnostics_json`, `shape_target_nodes_json`, `explain_code_json`, `why_json`.
- Create: `web/diagnostics.js` — pure rendering functions (no wasm calls, no global state):
  `escapeHtml`, card/snippet/banner/panel renderers.
- Modify: `web/index.html` — toolbar (drop the primary Output dropdown, add an "Explain a code"
  input), new results area (summary banner, diagnostics list, Shapes & Focus Nodes panel, raw
  report `<details>`), a shared slide-over `<aside>` for Why/Explain.
- Modify: `web/main.js` — wire the four new wasm bindings, replace `validateNow()` with
  `runValidate()` (diagnostics-first) + `generateRawReport()` (the demoted raw-report path),
  add `openExplainPanel`/`openWhyPanel`/`closeSidePanel` and click-delegation handlers.

## Task 1: Core helper `shape_target_nodes`

**Files:**
- Modify: `src/diagnostics/explain_pass.rs` (append after the end of the file, i.e. after the
  closing brace of `constraint_component_iri`)
- Modify: `src/diagnostics/mod.rs:13`
- Create: `tests/diagnostics_targets.rs`

**Interfaces:**
- Consumes: `crate::validation::resolve_target` (already `pub(crate)`, already imported/used in
  this file at `explain_pass.rs:60`), `Shape<'a>.targets: HashSet<Target<'a>>`,
  `Shape<'a>.node: NamedOrBlankNodeRef<'a>`, `ValidationDataset`.
- Produces: `pub fn shape_target_nodes<'a>(dataset: &'a ValidationDataset, shapes: &'a
  [Shape<'a>]) -> Vec<(String, Vec<String>)>` — one `(shape node display string, [distinct
  resolved focus node display strings, sorted])` per shape that has at least one target. Shapes
  with no targets are omitted. Re-exported from `shacl_rust::diagnostics::shape_target_nodes`.
  Consumed by Task 3 (wasm `shape_target_nodes_json`).

- [ ] **Step 1: Write the failing integration test**

Create `tests/diagnostics_targets.rs`:

```rust
use shacl_rust::diagnostics::shape_target_nodes;
use shacl_rust::parse_shapes;
use shacl_rust::rdf::read_graph_from_string;
use shacl_rust::validation::dataset::ValidationDataset;

const SHAPES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person .
    ex:OrphanShape a sh:NodeShape .
"#;

const DATA: &str = "@prefix ex: <http://example.org/> .
    ex:alice a ex:Person .
    ex:bob a ex:Person .";

fn run(shapes_ttl: &str, data_ttl: &str) -> Vec<(String, Vec<String>)> {
    let dg = read_graph_from_string(data_ttl, "turtle").unwrap();
    let sg = read_graph_from_string(shapes_ttl, "turtle").unwrap();
    let shapes = parse_shapes(&sg).unwrap();
    let dataset = ValidationDataset::from_graphs(dg, sg).unwrap();
    shape_target_nodes(&dataset, &shapes)
}

#[test]
fn lists_all_resolved_targets_conforming_and_violating_alike() {
    let result = run(SHAPES, DATA);

    assert_eq!(
        result.len(),
        1,
        "OrphanShape has no targets and must be omitted: {result:?}"
    );
    let (shape_node, nodes) = &result[0];
    assert_eq!(shape_node, "<http://example.org/PersonShape>");
    assert_eq!(
        nodes,
        &vec![
            "<http://example.org/alice>".to_string(),
            "<http://example.org/bob>".to_string(),
        ]
    );
}

#[test]
fn shape_with_no_targets_is_omitted_even_when_it_is_the_only_shape() {
    let shapes = "@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:OrphanShape a sh:NodeShape .";
    let result = run(shapes, "");
    assert!(result.is_empty(), "{result:?}");
}
```

- [ ] **Step 2: Run the test to verify it fails to compile**

Run: `cargo test --test diagnostics_targets`
Expected: compile error — `unresolved import 'shacl_rust::diagnostics::shape_target_nodes'` (or
similar `cannot find function` error), since the function doesn't exist yet.

- [ ] **Step 3: Implement `shape_target_nodes`**

Append to the end of `src/diagnostics/explain_pass.rs` (after the closing `}` of
`constraint_component_iri`):

```rust

/// Every shape with at least one target, paired with the Display strings of
/// its distinct resolved focus nodes - conforming and violating alike. This
/// is the data behind the web demo's "Shapes & Focus Nodes" browser, which
/// lets a user click a *conforming* node and ask "why did this pass?"
/// instead of only ever landing on nodes that already appear in a
/// violation. Shapes with no targets are omitted (nothing to browse).
pub fn shape_target_nodes<'a>(
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> Vec<(String, Vec<String>)> {
    shapes
        .iter()
        .filter(|shape| !shape.targets.is_empty())
        .map(|shape| {
            let nodes: std::collections::BTreeSet<String> = shape
                .targets
                .iter()
                .flat_map(|&target| crate::validation::resolve_target(dataset, target))
                .map(|term| term.to_string())
                .collect();
            (shape.node.to_string(), nodes.into_iter().collect())
        })
        .collect()
}
```

- [ ] **Step 4: Re-export it**

In `src/diagnostics/mod.rs`, change line 13 from:

```rust
pub use explain_pass::explain_conformance;
```

to:

```rust
pub use explain_pass::{explain_conformance, shape_target_nodes};
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test diagnostics_targets`
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 6: Commit**

```bash
git add src/diagnostics/explain_pass.rs src/diagnostics/mod.rs tests/diagnostics_targets.rs
git commit -m "feat: shape_target_nodes helper for browsing all resolved focus nodes"
```

## Task 2: New wasm diagnostics exports

**Files:**
- Modify: `crates/shacl-wasm/src/lib.rs` (append after line 131, the end of the file)

**Interfaces:**
- Consumes: `shacl_rust::diagnostics::{lint_shapes, from_report, sort_diagnostics,
  diagnostic_to_json, entry, explain_conformance, shape_target_nodes}` (all already `pub`),
  `shacl_rust::validate`, `shacl_rust::parse_shapes`,
  `shacl_rust::validation::dataset::ValidationDataset::from_graphs`,
  `shacl_rust::rdf::read_graph_from_string`, `to_js_error` (already defined at
  `crates/shacl-wasm/src/lib.rs:9`).
- Produces: four new `#[wasm_bindgen]` functions consumed by Task 6 (`web/main.js`):
  - `validate_diagnostics_json(data_graph: &str, shapes_graph: &str, data_format: &str,
    shapes_format: &str, skip_lint: bool) -> Result<String, JsValue>` — JSON array of
    `Diagnostic` objects.
  - `shape_target_nodes_json(data_graph: &str, shapes_graph: &str, data_format: &str,
    shapes_format: &str) -> Result<String, JsValue>` — JSON array of `{shape: string, targets:
    [{node: string, term_kind: "iri"|"blank"}]}`.
  - `explain_code_json(code: &str) -> Result<String, JsValue>` — JSON object `{code, title,
    component, spec_ref, explanation, failing_example, fixed_example}`, or a JS exception with
    message `"Unknown diagnostic code: {code}"`.
  - `why_json(data_graph: &str, shapes_graph: &str, data_format: &str, shapes_format: &str,
    focus_iri: &str, shape_iri: &str) -> Result<String, JsValue>` — JSON array of trace
    `Diagnostic` objects (with `verdict` populated). `shape_iri: ""` means "no filter".

- [ ] **Step 1: Append the four exports**

Append to the end of `crates/shacl-wasm/src/lib.rs` (after the closing `}` of
`lint_shapes_graph`):

```rust

#[wasm_bindgen]
pub fn validate_diagnostics_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    skip_lint: bool,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let validation_dataset =
        shacl_rust::validation::dataset::ValidationDataset::from_graphs(data, shapes)
            .map_err(|e| to_js_error(format!("Failed to create validation dataset: {}", e)))?;

    let parsed_shapes = parse_shapes(validation_dataset.shapes_graph())
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let mut diagnostics = if skip_lint {
        Vec::new()
    } else {
        shacl_rust::diagnostics::lint_shapes(validation_dataset.shapes_graph(), &parsed_shapes)
    };

    let report = validate(&validation_dataset, &parsed_shapes);
    diagnostics.extend(shacl_rust::diagnostics::from_report(
        &report,
        &validation_dataset,
        &parsed_shapes,
    ));
    shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

    let json_array: Vec<serde_json::Value> = diagnostics
        .iter()
        .map(shacl_rust::diagnostics::diagnostic_to_json)
        .collect();
    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize diagnostics: {}", e)))
}

#[wasm_bindgen]
pub fn shape_target_nodes_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let validation_dataset =
        shacl_rust::validation::dataset::ValidationDataset::from_graphs(data, shapes)
            .map_err(|e| to_js_error(format!("Failed to create validation dataset: {}", e)))?;

    let parsed_shapes = parse_shapes(validation_dataset.shapes_graph())
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let entries =
        shacl_rust::diagnostics::shape_target_nodes(&validation_dataset, &parsed_shapes);

    let json_array: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|(shape, nodes)| {
            let targets: Vec<serde_json::Value> = nodes
                .into_iter()
                .map(|node| {
                    let term_kind = if node.starts_with("_:") { "blank" } else { "iri" };
                    serde_json::json!({ "node": node, "term_kind": term_kind })
                })
                .collect();
            serde_json::json!({ "shape": shape, "targets": targets })
        })
        .collect();

    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize shape target nodes: {}", e)))
}

#[wasm_bindgen]
pub fn explain_code_json(code: &str) -> Result<String, JsValue> {
    let entry = shacl_rust::diagnostics::entry(code)
        .ok_or_else(|| to_js_error(format!("Unknown diagnostic code: {}", code)))?;

    let json = serde_json::json!({
        "code": entry.code,
        "title": entry.title,
        "component": entry.component,
        "spec_ref": entry.spec_ref,
        "explanation": entry.explanation,
        "failing_example": entry.failing_example,
        "fixed_example": entry.fixed_example,
    });

    serde_json::to_string(&json)
        .map_err(|e| to_js_error(format!("Failed to serialize registry entry: {}", e)))
}

/// Trims a single pair of optional surrounding angle brackets (`<...>`) from
/// an IRI argument, mirroring the CLI's `--focus`/`--shape` parsing
/// (`crates/shacl-cli/src/main.rs`'s `trim_angle_brackets`) so the web demo
/// can pass a `Diagnostic.focus_node`/`shape_target_nodes_json` display
/// string (always bracket-wrapped for IRIs) straight through.
fn trim_angle_brackets(s: &str) -> &str {
    s.trim().trim_start_matches('<').trim_end_matches('>')
}

#[wasm_bindgen]
pub fn why_json(
    data_graph: &str,
    shapes_graph: &str,
    data_format: &str,
    shapes_format: &str,
    focus_iri: &str,
    shape_iri: &str,
) -> Result<String, JsValue> {
    let data = read_graph_from_string(data_graph, data_format)
        .map_err(|e| to_js_error(format!("Failed to parse data graph: {}", e)))?;
    let shapes = read_graph_from_string(shapes_graph, shapes_format)
        .map_err(|e| to_js_error(format!("Failed to parse shapes graph: {}", e)))?;

    let validation_dataset =
        shacl_rust::validation::dataset::ValidationDataset::from_graphs(data, shapes)
            .map_err(|e| to_js_error(format!("Failed to create validation dataset: {}", e)))?;

    let parsed_shapes = parse_shapes(validation_dataset.shapes_graph())
        .map_err(|e| to_js_error(format!("Failed to parse SHACL shapes: {}", e)))?;

    let focus_trimmed = trim_angle_brackets(focus_iri);
    let focus_node = oxigraph::model::NamedNode::new(focus_trimmed)
        .map_err(|e| to_js_error(format!("Invalid focus IRI '{}': {}", focus_trimmed, e)))?;
    let focus_term = validation_dataset
        .data()
        .canonical_term(oxigraph::model::TermRef::from(focus_node.as_ref()))
        .unwrap_or_else(|| oxigraph::model::TermRef::from(focus_node.as_ref()));

    let shape_node = if shape_iri.is_empty() {
        None
    } else {
        let shape_trimmed = trim_angle_brackets(shape_iri);
        Some(oxigraph::model::NamedNode::new(shape_trimmed).map_err(|e| {
            to_js_error(format!("Invalid shape IRI '{}': {}", shape_trimmed, e))
        })?)
    };
    let shape_filter = shape_node
        .as_ref()
        .map(|n| oxigraph::model::NamedOrBlankNodeRef::from(n.as_ref()));

    let diags = shacl_rust::diagnostics::explain_conformance(
        &validation_dataset,
        &parsed_shapes,
        focus_term,
        shape_filter,
    );

    let json_array: Vec<serde_json::Value> = diags
        .iter()
        .map(shacl_rust::diagnostics::diagnostic_to_json)
        .collect();
    serde_json::to_string(&json_array)
        .map_err(|e| to_js_error(format!("Failed to serialize why trace: {}", e)))
}
```

- [ ] **Step 2: Verify it compiles for the wasm target**

Run: `cargo check -p shacl-wasm --target wasm32-unknown-unknown`
Expected: `Finished` with no errors (warnings, if any, must be fixed — target zero).

- [ ] **Step 3: Verify the native check and clippy also pass**

Run: `cargo check -p shacl-wasm && cargo clippy -p shacl-wasm --all-targets --all-features`
Expected: both `Finished` with no errors/warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/shacl-wasm/src/lib.rs
git commit -m "feat(wasm): add JSON diagnostics/why/explain/target-nodes exports"
```

## Task 3: `web/diagnostics.js` rendering module

**Files:**
- Create: `web/diagnostics.js`

**Interfaces:**
- Consumes: nothing (pure functions over plain JS objects/arrays matching the JSON shapes
  produced by Task 2's wasm exports).
- Produces (all named exports, consumed by Task 6's `web/main.js`):
  - `escapeHtml(value): string`
  - `renderSummaryBanner(diags: Diagnostic[]): string`
  - `renderDiagnosticsList(diags: Diagnostic[]): string`
  - `renderShapesPanel(shapeTargets: {shape, targets: {node, term_kind}[]}[], diags:
    Diagnostic[]): string`
  - `renderExplainPanel(entry: {code, title, component, spec_ref, explanation,
    failing_example, fixed_example}): string`
  - `renderWhyPanel(traceDiags: Diagnostic[], focusNode: string): string`
  - Rendered diagnostic cards carry `data-code`, `.code-badge[data-code]`, and
    `.focus-chip[data-focus][data-shape]` attributes/classes for `main.js`'s click delegation.
    Rendered shape-panel chips carry `.node-chip[data-node][data-shape][data-kind]`
    (`"iri"|"blank"`) for the same purpose.

- [ ] **Step 1: Write the file**

Create `web/diagnostics.js`:

```js
export function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

const SEVERITY_ICON = { error: "⛔", warning: "⚠️", info: "ℹ️" };
const VERDICT_ICON = {
  conforms: "✅",
  violates: "❌",
  "not-targeted": "⊘",
  vacuous: "~",
};
const VERDICT_LABEL = {
  conforms: "Conforms",
  violates: "Violates",
  "not-targeted": "Not targeted",
  vacuous: "Vacuously conforms",
};

function renderSnippet(snippet) {
  const origin = snippet.origin === "data" ? "Data graph" : "Shapes graph";
  const highlightIndex = snippet.highlight ? snippet.turtle.indexOf(snippet.highlight) : -1;

  let turtleHtml;
  if (highlightIndex >= 0) {
    const before = snippet.turtle.slice(0, highlightIndex);
    const match = snippet.turtle.slice(highlightIndex, highlightIndex + snippet.highlight.length);
    const after = snippet.turtle.slice(highlightIndex + snippet.highlight.length);
    turtleHtml = `${escapeHtml(before)}<mark>${escapeHtml(match)}</mark>${escapeHtml(after)}`;
  } else {
    turtleHtml = escapeHtml(snippet.turtle);
  }

  return `
    <div class="snippet">
      <div class="snippet-origin">${escapeHtml(origin)}</div>
      <pre class="snippet-turtle">${turtleHtml}</pre>
      <div class="snippet-annotation">${escapeHtml(snippet.annotation)}</div>
    </div>
  `;
}

function renderDiagnosticBody(diag) {
  const parts = [];
  for (const snippet of diag.snippets ?? []) {
    parts.push(renderSnippet(snippet));
  }
  if (diag.expected != null) {
    parts.push(`<div class="diag-field"><strong>Expected:</strong> ${escapeHtml(diag.expected)}</div>`);
  }
  if (diag.actual != null) {
    parts.push(`<div class="diag-field"><strong>Actual:</strong> ${escapeHtml(diag.actual)}</div>`);
  }
  for (const note of diag.notes ?? []) {
    parts.push(`<div class="diag-note">${escapeHtml(note)}</div>`);
  }
  if (diag.help) {
    parts.push(`<div class="diag-help"><strong>Help:</strong> ${escapeHtml(diag.help)}</div>`);
  }
  return parts.join("");
}

function renderDiagnosticCard(diag, options = {}) {
  const icon = options.leadingIcon ?? SEVERITY_ICON[diag.severity] ?? "";
  const expanded = options.defaultExpanded ?? diag.severity === "error";
  const footer =
    diag.focus_node != null
      ? `<div class="diag-footer">
          <button type="button" class="focus-chip" data-focus="${escapeHtml(diag.focus_node)}" data-shape="${escapeHtml(diag.source_shape ?? "")}">
            Explain why &rarr; <code>${escapeHtml(diag.focus_node)}</code>
          </button>
        </div>`
      : "";

  return `
    <article class="diag-card" data-code="${escapeHtml(diag.code)}">
      <header class="diag-header">
        <span class="diag-icon">${icon}</span>
        <button type="button" class="code-badge" data-code="${escapeHtml(diag.code)}">${escapeHtml(diag.code)}</button>
        <span class="diag-title">${escapeHtml(diag.title)}</span>
      </header>
      <div class="diag-body${expanded ? "" : " hidden"}">
        ${renderDiagnosticBody(diag)}
      </div>
      ${footer}
    </article>
  `;
}

function renderWhyTraceCard(diag) {
  const verdict = diag.verdict ?? "not-targeted";
  const icon = `${VERDICT_ICON[verdict] ?? ""} ${VERDICT_LABEL[verdict] ?? verdict}`;
  return renderDiagnosticCard(diag, { leadingIcon: icon, defaultExpanded: true });
}

export function renderSummaryBanner(diags) {
  const conforms = !diags.some((d) => d.code.startsWith("V") && d.severity === "error");
  const errorCount = diags.filter((d) => d.severity === "error").length;
  const warningCount = diags.filter((d) => d.severity === "warning").length;
  const cls = conforms ? "banner ok" : "banner err";
  const headline = conforms ? "✓ Conforms" : "✗ Data does not conform";

  return `<div class="${cls}">
    <strong>${escapeHtml(headline)}</strong>
    <span class="banner-counts">${errorCount} error${errorCount === 1 ? "" : "s"}, ${warningCount} warning${warningCount === 1 ? "" : "s"}</span>
  </div>`;
}

export function renderDiagnosticsList(diags) {
  if (diags.length === 0) {
    return '<p class="empty">No diagnostics.</p>';
  }
  return diags.map((d) => renderDiagnosticCard(d)).join("");
}

function violationKey(node, shape) {
  return `${node} ${shape ?? ""}`;
}

export function renderShapesPanel(shapeTargets, diags) {
  if (shapeTargets.length === 0) {
    return '<p class="empty">No shapes with targets.</p>';
  }

  const violatingPairs = new Set(
    diags.filter((d) => d.focus_node != null).map((d) => violationKey(d.focus_node, d.source_shape))
  );

  return shapeTargets
    .map((entry) => {
      const chips = entry.targets
        .map((t) => {
          const flagged = violatingPairs.has(violationKey(t.node, entry.shape)) ? " flagged" : "";
          return `<button type="button" class="node-chip${flagged}" data-node="${escapeHtml(t.node)}" data-shape="${escapeHtml(entry.shape)}" data-kind="${escapeHtml(t.term_kind)}">${escapeHtml(t.node)}</button>`;
        })
        .join("");
      return `<div class="shape-row">
        <div class="shape-name">${escapeHtml(entry.shape)}</div>
        <div class="node-chips">${chips}</div>
      </div>`;
    })
    .join("");
}

export function renderExplainPanel(entry) {
  const component = entry.component
    ? `<p class="explain-component"><strong>Component:</strong> ${escapeHtml(entry.component)}</p>`
    : "";

  return `
    <h3>${escapeHtml(entry.code)}: ${escapeHtml(entry.title)}</h3>
    ${component}
    <p><a href="${escapeHtml(entry.spec_ref)}" target="_blank" rel="noopener">SHACL spec reference &rarr;</a></p>
    <p class="explain-explanation">${escapeHtml(entry.explanation)}</p>
    <div class="diag-field"><strong>Failing example</strong><pre class="snippet-turtle">${escapeHtml(entry.failing_example)}</pre></div>
    <div class="diag-field"><strong>Fixed example</strong><pre class="snippet-turtle">${escapeHtml(entry.fixed_example)}</pre></div>
  `;
}

export function renderWhyPanel(traceDiags, focusNode) {
  if (traceDiags.length === 0) {
    return `<p class="empty">No trace results for <code>${escapeHtml(focusNode)}</code>.</p>`;
  }
  return traceDiags.map((d) => renderWhyTraceCard(d)).join("");
}
```

- [ ] **Step 2: Syntax-check the file**

Run: `node --check web/diagnostics.js`
Expected: no output, exit code 0.

- [ ] **Step 3: Commit**

```bash
git add web/diagnostics.js
git commit -m "feat(web): pure rendering module for the diagnostics-first UI"
```

## Task 4: Restructure `web/index.html`

**Files:**
- Modify: `web/index.html` (full-file rewrite: CSS additions + toolbar/results markup)

**Interfaces:**
- Consumes: nothing new (still loads `./main.js` as a module).
- Produces: DOM element ids consumed by Task 6's `web/main.js`: `explain-code-input`,
  `explain-code-btn`, `summary-banner`, `diagnostics-list`, `shapes-panel-details`,
  `shapes-panel-body`, `raw-report-btn`, `side-panel`, `side-panel-title`, `side-panel-body`,
  `side-panel-close`. Existing ids kept as-is: `status`, `validate-btn`, `data-file`,
  `shapes-file`, `data-format`, `shapes-format`, `output-type`, `rdf-output-label`,
  `rdf-output-format`, `skip-lint-check`, `output`, `data-graph-editor`, `shapes-graph-editor`.

- [ ] **Step 1: Write the file**

Rewrite `web/index.html`:

```html
<!doctype html>
<html lang="en">

<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>SHACL Validator (WASM)</title>
    <style>
        :root {
            color-scheme: dark;
        }

        body {
            margin: 0;
            font-family: system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, sans-serif;
            background: #111827;
            color: #e5e7eb;
        }

        main {
            max-width: 96vw;
            margin: 1rem auto;
            padding: 0 0.75rem;
        }

        h1 {
            margin: 0;
        }

        .header {
            margin-bottom: 0.75rem;
        }

        .card {
            background: #1f2937;
            border-radius: 12px;
            padding: 0.8rem;
            border: 1px solid #374151;
        }

        .grid {
            display: grid;
            gap: 0.75rem;
        }

        .grid-2 {
            grid-template-columns: 1fr 1fr;
        }

        @media (max-width: 900px) {
            .grid-2 {
                grid-template-columns: 1fr;
            }
        }

        .label-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 0.75rem;
            margin-bottom: 0.5rem;
        }

        label {
            font-size: 0.92rem;
            color: #d1d5db;
        }

        select,
        button,
        textarea,
        input[type="text"] {
            font: inherit;
        }

        select,
        button,
        input[type="text"] {
            background: #111827;
            color: #e5e7eb;
            border: 1px solid #4b5563;
            border-radius: 8px;
            padding: 0.45rem 0.6rem;
        }

        button {
            cursor: pointer;
        }

        button:disabled {
            opacity: 0.6;
            cursor: not-allowed;
        }

        textarea {
            width: 100%;
            min-height: 180px;
            resize: vertical;
            border: 1px solid #4b5563;
            border-radius: 8px;
            padding: 0.65rem;
            background: #111827;
            color: #f3f4f6;
        }

        .editor {
            min-height: 64vh;
            border: 1px solid #4b5563;
            border-radius: 8px;
            overflow: hidden;
        }

        .editor .cm-editor {
            height: 64vh;
        }

        .editor .cm-scroller {
            overflow: auto;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        }

        .toolbar {
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            gap: 0.6rem;
            margin: 0.75rem 0;
        }

        .toolbar .spacer {
            flex: 1;
        }

        .hidden {
            display: none;
        }

        .status {
            margin: 0.35rem 0 0.75rem;
        }

        .ok {
            color: #34d399;
        }

        .err {
            color: #f87171;
        }

        code {
            background: #111827;
            padding: 0.1rem 0.35rem;
            border-radius: 6px;
        }

        .file-input {
            max-width: 220px;
            font-size: 0.85rem;
        }

        .explain-search-row {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            margin: 0.5rem 0 1rem;
        }

        .explain-search-row input {
            width: 140px;
        }

        .banner {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            padding: 0.75rem 1rem;
            border-radius: 10px;
            margin-bottom: 0.75rem;
        }

        .banner.ok {
            background: #064e3b;
            color: #6ee7b7;
            border: 1px solid #10b981;
        }

        .banner.err {
            background: #450a0a;
            color: #fca5a5;
            border: 1px solid #ef4444;
        }

        .banner-counts {
            opacity: 0.85;
            font-size: 0.9rem;
        }

        .diag-list {
            display: flex;
            flex-direction: column;
            gap: 0.6rem;
            margin-bottom: 0.75rem;
        }

        .diag-card {
            background: #1f2937;
            border: 1px solid #374151;
            border-radius: 10px;
            overflow: hidden;
        }

        .diag-header {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.6rem 0.8rem;
            cursor: pointer;
        }

        .code-badge {
            background: #111827;
            color: #93c5fd;
            border: 1px solid #4b5563;
            border-radius: 6px;
            padding: 0.1rem 0.45rem;
            font-size: 0.85rem;
        }

        .diag-title {
            flex: 1;
        }

        .diag-body {
            padding: 0 0.8rem 0.7rem;
            border-top: 1px solid #374151;
        }

        .snippet {
            margin: 0.6rem 0;
        }

        .snippet-origin {
            font-size: 0.8rem;
            color: #9ca3af;
            margin-bottom: 0.2rem;
        }

        .snippet-turtle {
            background: #111827;
            border: 1px solid #374151;
            border-radius: 8px;
            padding: 0.6rem;
            overflow-x: auto;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
            font-size: 0.85rem;
            white-space: pre-wrap;
            word-break: break-word;
        }

        .snippet-turtle mark {
            background: #b45309;
            color: #fffbeb;
            border-radius: 3px;
            padding: 0 0.1rem;
        }

        .snippet-annotation {
            font-size: 0.85rem;
            color: #d1d5db;
            margin-top: 0.2rem;
        }

        .diag-field,
        .diag-note,
        .diag-help {
            font-size: 0.88rem;
            margin: 0.35rem 0;
        }

        .diag-help {
            color: #93c5fd;
        }

        .diag-footer {
            padding: 0 0.8rem 0.7rem;
        }

        .focus-chip,
        .node-chip {
            background: #111827;
            color: #e5e7eb;
            border: 1px solid #4b5563;
            border-radius: 999px;
            padding: 0.25rem 0.65rem;
            font-size: 0.82rem;
            position: relative;
        }

        .focus-chip code {
            background: transparent;
            padding: 0;
        }

        .shapes-panel summary,
        .raw-report-details summary {
            cursor: pointer;
            font-weight: 600;
            padding: 0.2rem 0;
        }

        .shape-row {
            margin: 0.6rem 0;
        }

        .shape-name {
            font-size: 0.85rem;
            color: #9ca3af;
            margin-bottom: 0.3rem;
            word-break: break-all;
        }

        .node-chips {
            display: flex;
            flex-wrap: wrap;
            gap: 0.4rem;
        }

        .node-chip.flagged::after {
            content: "";
            position: absolute;
            top: -3px;
            right: -3px;
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: #ef4444;
        }

        .side-panel {
            position: fixed;
            top: 0;
            right: 0;
            width: min(480px, 92vw);
            height: 100vh;
            background: #1f2937;
            border-left: 1px solid #374151;
            box-shadow: -8px 0 24px rgba(0, 0, 0, 0.4);
            overflow-y: auto;
            z-index: 20;
            padding: 1rem;
        }

        .side-panel-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 0.75rem;
        }

        .side-panel-header button {
            background: transparent;
            border: none;
            font-size: 1.3rem;
            line-height: 1;
            padding: 0.2rem 0.5rem;
        }

        .empty {
            color: #9ca3af;
            font-size: 0.9rem;
        }

        @media (max-width: 900px) {
            .editor {
                min-height: 42vh;
            }

            .editor .cm-editor {
                height: 42vh;
            }
        }
    </style>
</head>

<body>
    <main>
        <div class="header">
            <h1>SHACL Validator (WASM)</h1>
            <p>Validate a data graph against SHACL shapes directly in your browser.</p>
        </div>

        <div class="grid grid-2">
            <section class="card">
                <div class="label-row">
                    <label for="data-format">Data graph format</label>
                    <select id="data-format">
                        <option value="ttl">Turtle (.ttl)</option>
                        <option value="nt">N-Triples (.nt)</option>
                        <option value="nq">N-Quads (.nq)</option>
                        <option value="rdf">RDF/XML (.rdf)</option>
                        <option value="jsonld">JSON-LD (.jsonld)</option>
                        <option value="trig">TriG (.trig)</option>
                    </select>
                </div>
                <div class="label-row">
                    <label for="data-file">Data graph file</label>
                    <input id="data-file" class="file-input" type="file"
                        accept=".ttl,.nt,.nq,.rdf,.xml,.jsonld,.json,.trig,text/turtle,application/n-triples,application/n-quads,application/rdf+xml,application/ld+json,application/trig" />
                </div>
                <div id="data-graph-editor" class="editor"></div>
            </section>

            <section class="card">
                <div class="label-row">
                    <label for="shapes-format">Shapes graph format</label>
                    <select id="shapes-format">
                        <option value="ttl">Turtle (.ttl)</option>
                        <option value="nt">N-Triples (.nt)</option>
                        <option value="nq">N-Quads (.nq)</option>
                        <option value="rdf">RDF/XML (.rdf)</option>
                        <option value="jsonld">JSON-LD (.jsonld)</option>
                        <option value="trig">TriG (.trig)</option>
                    </select>
                </div>
                <div class="label-row">
                    <label for="shapes-file">Shapes graph file</label>
                    <input id="shapes-file" class="file-input" type="file"
                        accept=".ttl,.nt,.nq,.rdf,.xml,.jsonld,.json,.trig,text/turtle,application/n-triples,application/n-quads,application/rdf+xml,application/ld+json,application/trig" />
                </div>
                <div id="shapes-graph-editor" class="editor"></div>
            </section>
        </div>

        <div class="toolbar">
            <label><input type="checkbox" id="skip-lint-check" /> Skip shape lint</label>
            <span class="spacer"></span>
            <button id="validate-btn">Validate</button>
        </div>

        <p id="status" class="status">Loading WebAssembly package...</p>

        <div class="explain-search-row">
            <label for="explain-code-input">Explain a code</label>
            <input id="explain-code-input" type="text" placeholder="e.g. V0007" />
            <button id="explain-code-btn">Explain</button>
        </div>

        <section id="summary-banner"></section>

        <section id="diagnostics-list" class="diag-list"></section>

        <details id="shapes-panel-details" class="card shapes-panel hidden">
            <summary>Shapes &amp; Focus Nodes</summary>
            <div id="shapes-panel-body"></div>
        </details>

        <details class="card raw-report-details">
            <summary>Raw report</summary>
            <div class="toolbar">
                <label for="output-type">Output</label>
                <select id="output-type">
                    <option value="text">Text</option>
                    <option value="json">JSON</option>
                    <option value="rdf">RDF</option>
                </select>

                <label id="rdf-output-label" class="hidden" for="rdf-output-format">RDF format</label>
                <select id="rdf-output-format" class="hidden">
                    <option value="ttl">Turtle (.ttl)</option>
                    <option value="nt">N-Triples (.nt)</option>
                    <option value="nq">N-Quads (.nq)</option>
                    <option value="rdf">RDF/XML (.rdf)</option>
                    <option value="jsonld">JSON-LD (.jsonld)</option>
                    <option value="trig">TriG (.trig)</option>
                </select>

                <span class="spacer"></span>
                <button id="raw-report-btn">Generate raw report</button>
            </div>
            <textarea id="output" spellcheck="false" readonly></textarea>
        </details>

        <aside id="side-panel" class="side-panel hidden">
            <div class="side-panel-header">
                <strong id="side-panel-title"></strong>
                <button id="side-panel-close" type="button" aria-label="Close">&times;</button>
            </div>
            <div id="side-panel-body"></div>
        </aside>
    </main>
    <script type="module" src="./main.js"></script>
</body>

</html>
```

- [ ] **Step 2: Sanity-check the markup**

Run: `python3 -c "import re,sys; s=open('web/index.html').read(); ids=re.findall(r'id=\"([^\"]+)\"', s); dupes=[i for i in set(ids) if ids.count(i)>1]; print('dupes:', dupes); sys.exit(1 if dupes else 0)"`
Expected: `dupes: []`, exit code 0 (no duplicate element ids).

- [ ] **Step 3: Commit**

```bash
git add web/index.html
git commit -m "feat(web): diagnostics-first layout — summary banner, diagnostics list, shapes panel, raw report disclosure"
```

## Task 5: Rewire `web/main.js`

**Files:**
- Modify: `web/main.js` (full-file rewrite)

**Interfaces:**
- Consumes: `web/diagnostics.js`'s `renderSummaryBanner`, `renderDiagnosticsList`,
  `renderShapesPanel`, `renderExplainPanel`, `renderWhyPanel` (Task 3); wasm exports
  `validate_diagnostics_json`, `shape_target_nodes_json`, `explain_code_json`, `why_json` (Task
  2) plus the untouched `validate_graphs`, `lint_data_graph`, `lint_shapes_graph`; DOM ids from
  Task 4.
- Produces: nothing consumed elsewhere (this is the top-level bootstrap module).

- [ ] **Step 1: Write the file**

Rewrite `web/main.js`:

```js
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "https://esm.sh/@codemirror/commands@6.8.1?deps=@codemirror/state@6.5.2,@codemirror/view@6.38.8";
import {
  forceLinting,
  lintGutter,
  linter,
} from "https://esm.sh/@codemirror/lint@6.9.2?deps=@codemirror/state@6.5.2,@codemirror/view@6.38.8";
import { EditorState } from "https://esm.sh/@codemirror/state@6.5.2";
import { oneDark } from "https://esm.sh/@codemirror/theme-one-dark@6.1.3?deps=@codemirror/state@6.5.2,@codemirror/view@6.38.8";
import { EditorView, keymap, lineNumbers } from "https://esm.sh/@codemirror/view@6.38.8?deps=@codemirror/state@6.5.2";
import {
  renderSummaryBanner,
  renderDiagnosticsList,
  renderShapesPanel,
  renderExplainPanel,
  renderWhyPanel,
} from "./diagnostics.js";

const statusEl = document.getElementById("status");
const validateBtn = document.getElementById("validate-btn");
const dataFileEl = document.getElementById("data-file");
const shapesFileEl = document.getElementById("shapes-file");
const dataFormatEl = document.getElementById("data-format");
const shapesFormatEl = document.getElementById("shapes-format");
const outputTypeEl = document.getElementById("output-type");
const rdfOutputLabelEl = document.getElementById("rdf-output-label");
const rdfOutputFormatEl = document.getElementById("rdf-output-format");
const skipLintCheckEl = document.getElementById("skip-lint-check");
const outputEl = document.getElementById("output");
const rawReportBtnEl = document.getElementById("raw-report-btn");
const explainCodeInputEl = document.getElementById("explain-code-input");
const explainCodeBtnEl = document.getElementById("explain-code-btn");
const summaryBannerEl = document.getElementById("summary-banner");
const diagnosticsListEl = document.getElementById("diagnostics-list");
const shapesPanelDetailsEl = document.getElementById("shapes-panel-details");
const shapesPanelBodyEl = document.getElementById("shapes-panel-body");
const sidePanelEl = document.getElementById("side-panel");
const sidePanelTitleEl = document.getElementById("side-panel-title");
const sidePanelBodyEl = document.getElementById("side-panel-body");
const sidePanelCloseEl = document.getElementById("side-panel-close");

const dataEditorEl = document.getElementById("data-graph-editor");
const shapesEditorEl = document.getElementById("shapes-graph-editor");

const EXAMPLE_DATA_TTL = `@prefix ex: <http://example.com/> .

ex:alice a ex:Person ;
  ex:age 17 .
`;

const EXAMPLE_SHAPES_TTL = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.com/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:age ;
    sh:datatype xsd:integer ;
    sh:minInclusive 18 ;
  ] .
`;

const FILE_EXTENSION_TO_FORMAT = {
  ttl: "ttl",
  nt: "nt",
  rdf: "rdf",
  xml: "rdf",
  jsonld: "jsonld",
  json: "jsonld",
  trig: "trig",
};

let wasmReady = false;
let wasmInit = null;
let validateGraphs = null;
let validateDiagnosticsJson = null;
let shapeTargetNodesJson = null;
let explainCodeJson = null;
let whyJson = null;
let lintDataGraph = null;
let lintShapesGraph = null;
let dataEditor = null;
let shapesEditor = null;

function setStatus(message, level = "ok") {
  statusEl.textContent = message;
  statusEl.className = `status ${level}`;
}

function syncRdfOutputVisibility() {
  const showRdfFormat = outputTypeEl.value === "rdf";
  rdfOutputLabelEl.classList.toggle("hidden", !showRdfFormat);
  rdfOutputFormatEl.classList.toggle("hidden", !showRdfFormat);
}

function currentOutputFormat() {
  if (outputTypeEl.value === "rdf") {
    return rdfOutputFormatEl.value;
  }
  return outputTypeEl.value;
}

function parseLineFromError(errorMessage) {
  const lineMatch = /line\s+(\d+)/i.exec(errorMessage);
  if (!lineMatch) {
    return 1;
  }
  const parsed = Number.parseInt(lineMatch[1], 10);
  if (!Number.isFinite(parsed) || parsed < 1) {
    return 1;
  }
  return parsed;
}

function lineToPos(doc, lineNumber) {
  const line = doc.line(Math.max(1, Math.min(lineNumber, doc.lines)));
  return { from: line.from, to: line.to };
}

async function dataGraphLinter(view) {
  if (!wasmReady || !lintDataGraph) {
    return [];
  }

  const text = view.state.doc.toString();
  if (!text.trim()) {
    return [];
  }

  try {
    lintDataGraph(text, dataFormatEl.value);
    return [];
  } catch (error) {
    const message = String(error);
    const lineNumber = parseLineFromError(message);
    const range = lineToPos(view.state.doc, lineNumber);

    return [
      {
        from: range.from,
        to: Math.max(range.from + 1, range.to),
        severity: "error",
        message,
      },
    ];
  }
}

async function shapesGraphLinter(view) {
  if (!wasmReady || !lintShapesGraph) {
    return [];
  }

  const text = view.state.doc.toString();
  if (!text.trim()) {
    return [];
  }

  try {
    lintShapesGraph(text, shapesFormatEl.value);
    return [];
  } catch (error) {
    const message = String(error);
    const lineNumber = parseLineFromError(message);
    const range = lineToPos(view.state.doc, lineNumber);

    return [
      {
        from: range.from,
        to: Math.max(range.from + 1, range.to),
        severity: "error",
        message,
      },
    ];
  }
}

function baseEditorExtensions(customLinter) {
  return [
    lineNumbers(),
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
    oneDark,
    lintGutter(),
    linter(customLinter, { delay: 500 }),
    EditorView.lineWrapping,
  ];
}

function setEditorText(editor, text) {
  editor.dispatch({
    changes: {
      from: 0,
      to: editor.state.doc.length,
      insert: text,
    },
  });
}

function detectFormatFromFilename(fileName) {
  const extension = fileName.toLowerCase().split(".").pop();
  if (!extension) {
    return null;
  }
  return FILE_EXTENSION_TO_FORMAT[extension] ?? null;
}

function updateLinting() {
  if (dataEditor) {
    forceLinting(dataEditor);
  }
  if (shapesEditor) {
    forceLinting(shapesEditor);
  }
}

async function handleUpload(fileInput, editor, formatSelect) {
  const file = fileInput.files?.[0];
  if (!file) {
    return;
  }

  const text = await file.text();
  setEditorText(editor, text);

  const detectedFormat = detectFormatFromFilename(file.name);
  if (detectedFormat) {
    formatSelect.value = detectedFormat;
  }

  updateLinting();
  setStatus(`Loaded file: ${file.name}`, "ok");
}

function getDataGraphText() {
  return dataEditor.state.doc.toString();
}

function getShapesGraphText() {
  return shapesEditor.state.doc.toString();
}

function closeSidePanel() {
  sidePanelEl.classList.add("hidden");
  sidePanelBodyEl.innerHTML = "";
  sidePanelTitleEl.textContent = "";
}

function openSidePanel(title) {
  sidePanelTitleEl.textContent = title;
  sidePanelEl.classList.remove("hidden");
}

function openExplainPanel(code) {
  const trimmed = (code ?? "").trim();
  if (!trimmed) {
    return;
  }
  if (!wasmReady || !explainCodeJson) {
    setStatus("WASM is not ready yet.", "err");
    return;
  }

  openSidePanel(`Explain: ${trimmed}`);
  try {
    const entry = JSON.parse(explainCodeJson(trimmed));
    sidePanelTitleEl.textContent = `Explain: ${entry.code}`;
    sidePanelBodyEl.innerHTML = renderExplainPanel(entry);
  } catch (error) {
    sidePanelBodyEl.innerHTML = `<p class="empty">${String(error)}</p>`;
  }
}

function openWhyPanel(focusNode, shapeIri, options = {}) {
  openSidePanel(`Why: ${focusNode}`);

  if (options.blocked) {
    sidePanelBodyEl.innerHTML =
      '<p class="empty">Why-trace requires an IRI focus node; blank node focus nodes are not supported yet.</p>';
    return;
  }

  if (!wasmReady || !whyJson) {
    sidePanelBodyEl.innerHTML = '<p class="empty">WASM is not ready yet.</p>';
    return;
  }

  try {
    const trace = JSON.parse(
      whyJson(
        getDataGraphText(),
        getShapesGraphText(),
        dataFormatEl.value,
        shapesFormatEl.value,
        focusNode,
        shapeIri ?? ""
      )
    );
    sidePanelBodyEl.innerHTML = renderWhyPanel(trace, focusNode);
  } catch (error) {
    sidePanelBodyEl.innerHTML = `<p class="empty">${String(error)}</p>`;
  }
}

function toggleDiagBody(headerEl) {
  const body = headerEl.parentElement.querySelector(".diag-body");
  body?.classList.toggle("hidden");
}

function handleDiagnosticsListClick(event) {
  const codeBadge = event.target.closest(".code-badge");
  if (codeBadge) {
    openExplainPanel(codeBadge.dataset.code);
    return;
  }

  const focusChip = event.target.closest(".focus-chip");
  if (focusChip) {
    openWhyPanel(focusChip.dataset.focus, focusChip.dataset.shape || null);
    return;
  }

  const header = event.target.closest(".diag-header");
  if (header) {
    toggleDiagBody(header);
  }
}

function handleShapesPanelClick(event) {
  const chip = event.target.closest(".node-chip");
  if (!chip) {
    return;
  }
  openWhyPanel(chip.dataset.node, chip.dataset.shape, {
    blocked: chip.dataset.kind === "blank",
  });
}

function handleSidePanelClick(event) {
  const codeBadge = event.target.closest(".code-badge");
  if (codeBadge) {
    openExplainPanel(codeBadge.dataset.code);
    return;
  }

  const header = event.target.closest(".diag-header");
  if (header) {
    toggleDiagBody(header);
  }
}

function runValidate() {
  if (!wasmReady || !validateDiagnosticsJson || !shapeTargetNodesJson) {
    setStatus("WASM is not ready yet.", "err");
    return;
  }

  validateBtn.disabled = true;
  setStatus("Validating...", "ok");
  closeSidePanel();

  try {
    const dataText = getDataGraphText();
    const shapesText = getShapesGraphText();

    const diagnostics = JSON.parse(
      validateDiagnosticsJson(
        dataText,
        shapesText,
        dataFormatEl.value,
        shapesFormatEl.value,
        skipLintCheckEl.checked
      )
    );
    const shapeTargets = JSON.parse(
      shapeTargetNodesJson(dataText, shapesText, dataFormatEl.value, shapesFormatEl.value)
    );

    summaryBannerEl.innerHTML = renderSummaryBanner(diagnostics);
    diagnosticsListEl.innerHTML = renderDiagnosticsList(diagnostics);
    shapesPanelBodyEl.innerHTML = renderShapesPanel(shapeTargets, diagnostics);
    shapesPanelDetailsEl.classList.toggle("hidden", shapeTargets.length === 0);

    setStatus("Validation completed.", "ok");
  } catch (error) {
    summaryBannerEl.innerHTML = "";
    diagnosticsListEl.innerHTML = "";
    shapesPanelDetailsEl.classList.add("hidden");
    setStatus(`Validation failed: ${error}`, "err");
  } finally {
    validateBtn.disabled = false;
  }
}

function generateRawReport() {
  if (!wasmReady || !validateGraphs) {
    setStatus("WASM is not ready yet.", "err");
    return;
  }

  try {
    const result = validateGraphs(
      getDataGraphText(),
      getShapesGraphText(),
      dataFormatEl.value,
      shapesFormatEl.value,
      currentOutputFormat()
    );

    if (outputTypeEl.value === "json") {
      try {
        outputEl.value = JSON.stringify(JSON.parse(result), null, 2);
      } catch {
        outputEl.value = result;
      }
    } else {
      outputEl.value = result;
    }

    setStatus("Raw report generated.", "ok");
  } catch (error) {
    outputEl.value = "";
    setStatus(`Raw report failed: ${error}`, "err");
  }
}

async function loadWasmModule() {
  const moduleUrl = new URL("./pkg/shacl_wasm.js", import.meta.url).href;
  const wasmModule = await import(moduleUrl);
  wasmInit = wasmModule.default;
  validateGraphs = wasmModule.validate_graphs;
  validateDiagnosticsJson = wasmModule.validate_diagnostics_json;
  shapeTargetNodesJson = wasmModule.shape_target_nodes_json;
  explainCodeJson = wasmModule.explain_code_json;
  whyJson = wasmModule.why_json;
  lintDataGraph = wasmModule.lint_data_graph;
  lintShapesGraph = wasmModule.lint_shapes_graph;
}

function buildEditors() {
  dataEditor = new EditorView({
    state: EditorState.create({
      doc: EXAMPLE_DATA_TTL,
      extensions: baseEditorExtensions(dataGraphLinter),
    }),
    parent: dataEditorEl,
  });

  shapesEditor = new EditorView({
    state: EditorState.create({
      doc: EXAMPLE_SHAPES_TTL,
      extensions: baseEditorExtensions(shapesGraphLinter),
    }),
    parent: shapesEditorEl,
  });
}

async function bootstrap() {
  syncRdfOutputVisibility();
  outputTypeEl.addEventListener("change", syncRdfOutputVisibility);
  validateBtn.addEventListener("click", runValidate);
  rawReportBtnEl.addEventListener("click", generateRawReport);
  explainCodeBtnEl.addEventListener("click", () => openExplainPanel(explainCodeInputEl.value));
  explainCodeInputEl.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      openExplainPanel(explainCodeInputEl.value);
    }
  });
  sidePanelCloseEl.addEventListener("click", closeSidePanel);
  diagnosticsListEl.addEventListener("click", handleDiagnosticsListClick);
  shapesPanelBodyEl.addEventListener("click", handleShapesPanelClick);
  sidePanelBodyEl.addEventListener("click", handleSidePanelClick);

  dataFormatEl.addEventListener("change", updateLinting);
  shapesFormatEl.addEventListener("change", updateLinting);

  dataFileEl.addEventListener("change", () => handleUpload(dataFileEl, dataEditor, dataFormatEl));
  shapesFileEl.addEventListener("change", () =>
    handleUpload(shapesFileEl, shapesEditor, shapesFormatEl)
  );

  buildEditors();

  try {
    await loadWasmModule();
    await wasmInit();
    wasmReady = true;
    setStatus("WASM package loaded successfully.", "ok");
    updateLinting();
  } catch (error) {
    setStatus(
      `Failed to initialize WASM: ${error}.`,
      "err"
    );
  }
}

bootstrap();
```

- [ ] **Step 2: Syntax-check the file**

Run: `node --check web/main.js`
Expected: no output, exit code 0.

- [ ] **Step 3: Commit**

```bash
git add web/main.js
git commit -m "feat(web): wire diagnostics-first UI to new wasm JSON exports"
```

## Task 6: Build, manual verification, and final gates

**Files:** none (build + verification only)

**Interfaces:** none.

- [ ] **Step 1: Rebuild the wasm package**

Run: `wasm-pack build crates/shacl-wasm --target web --out-dir ../../web/pkg`
Expected: `[INFO]: :-) Your wasm pkg is ready to publish`, no errors.

- [ ] **Step 2: Confirm the four new exports are present in the generated bindings**

Run: `grep -c "export function validate_diagnostics_json\|export function shape_target_nodes_json\|export function explain_code_json\|export function why_json" web/pkg/shacl_wasm.js`
Expected: `4`

- [ ] **Step 3: Serve the demo locally**

Run (background/separate terminal): `cd web && python3 -m http.server 8000`

- [ ] **Step 4: Manual browser walkthrough**

Open `http://localhost:8000` in a browser (headless Playwright is fine) and verify each:
  1. Page loads, status shows "WASM package loaded successfully."
  2. Click **Validate** with the default example data/shapes (age 17, `sh:minInclusive 18`) →
     summary banner shows "✗ Data does not conform", one error-severity diagnostic card is
     expanded by default, showing a snippet with a `<mark>`-highlighted span.
  3. Click the diagnostic's code badge → side panel opens titled "Explain: V00xx" with
     explanation, failing/fixed examples, and a working spec-ref link.
  4. Click the diagnostic's "Explain why →" focus-node chip → side panel switches to "Why:
     ..." with verdict-badged trace cards (e.g. ❌ Violates for the age constraint).
  5. Open the **Shapes & Focus Nodes** panel → `ex:PersonShape` is listed with one node chip
     (`ex:alice`) carrying the red "flagged" dot (it's the violating focus node).
  6. Edit the data graph so `ex:age` is `20` (conforming), click **Validate** again → banner
     turns green "✓ Conforms", zero diagnostic cards. Open Shapes & Focus Nodes, click the
     `ex:alice` chip (now unflagged, no violation) → Why panel opens and shows a ✅ Conforms
     trace for the `minInclusive` constraint — confirms a *conforming* node's trace is
     reachable, not just violating ones.
  7. Type `V0007` into the standalone "Explain a code" box, click **Explain** → panel opens
     with that code's content, independent of any diagnostic card or loaded graph.
  8. Type an unknown code (e.g. `V9999`) into the same box → panel shows the
     `"Unknown diagnostic code: V9999"` message instead of throwing an uncaught error.
  9. Open the **Raw report** disclosure (collapsed by default), select each of Text/JSON/RDF,
     click **Generate raw report** → textarea populates correctly for all three, matching
     pre-redesign behavior exactly (RDF format sub-selector still appears only for RDF).
  10. Confirm the CodeMirror syntax-error gutters (paste malformed Turtle into either editor)
      still show inline errors — unaffected by this redesign.

- [ ] **Step 5: Run the Rust verification gates**

Run: `cargo test`
Expected: full suite passes, conformance suite 120/120.

Run: `cargo clippy --all-targets --all-features`
Expected: 0 warnings.

Run: `cargo fmt --all --check`
Expected: no output (already formatted).

Run: `cargo check -p shacl-wasm --target wasm32-unknown-unknown`
Expected: `Finished` with no errors.

- [ ] **Step 6: No commit for this task**

`web/pkg/` is gitignored (build output only); nothing new to stage. If any issue surfaced during
manual verification required a source fix, that fix should have already been committed as part
of the relevant earlier task — amend that task's work rather than leaving an uncommitted fix
here.
