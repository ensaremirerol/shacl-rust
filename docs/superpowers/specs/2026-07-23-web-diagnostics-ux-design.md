# Web Diagnostics UX + MCP Integration — Design

**Date:** 2026-07-23
**Status:** approved pending user review
**Builds on:** `docs/superpowers/specs/2026-07-23-diagnostics-design.md` (the diagnostics
model itself — `Diagnostic`, `render_text`/`render_ndjson`, `lint_shapes`, `from_report`,
`explain_conformance`, the code registry — all already implemented and shipped on `main`).

## Goal

Two follow-ups to the diagnostics feature, scoped together because both are "expose the
already-built diagnostics model to a consumer that doesn't have it yet":

1. **Web demo redesign** — the browser demo (`web/`) currently only exposes `validate`
   (as a plain-text/JSON/RDF report). Redesign it into a diagnostics-first, click-to-drill-down
   UI that showcases all four diagnostics capabilities: rich violation diagnostics, shape
   linting, `explain`, and `why` (including "why did this *conforming* node pass").
2. **MCP integration** — the MCP server (`crates/shacl-mcp`) currently exposes `validate_graphs`,
   `validate_graphs_conforms`, `lint_graph` (syntax-only), `parse_shapes_graph`. It has no
   tools for the diagnostics feature at all. Add them, mirroring the CLI/Python/wasm surface.

**Execution order:** web redesign first; MCP integration is written into this spec now (so
scope isn't lost across a context reset) but is explicitly deferred — do not implement it
until asked.

## Decisions (settled during brainstorming, web section)

1. **Scope:** all four features — Validate+lint+diagnostics, Explain, Why — no feature cut.
   `why` is treated as core, not an afterthought.
2. **Visual companion:** declined; design specified in text/prose, not mockups.
3. **Page organization: unified flow.** One "Validate" action produces one rich result view
   containing conformance + lint findings + violation diagnostics together. There is no
   separate "Lint" tab — lint diagnostics are just part of the same sorted list `from_report`/
   `lint_shapes` already produce combined (L-codes sort before V-codes lexically). `Explain`
   is reached by clicking any diagnostic's code badge (or via a standalone code-search box).
   `Why` is reached by clicking a focus-node chip — never a blind typed-IRI box.
4. **Raw report kept, demoted.** The existing text/JSON/RDF report output stays, but moves
   into a collapsed "Raw report" disclosure below the diagnostics view — real, already-shipped
   functionality (especially the RDF serialization, which diagnostics can't replace), just no
   longer the primary view.
5. **`why` node scope: all resolved targets, not just violating ones.** Every shape's
   resolved focus nodes are listed (via a new small library helper), so a user can click a
   *conforming* node and ask "why did this pass?" — not only click into nodes that already
   appear in a violation.

## Architecture — new wasm surface (additive only)

The existing four wasm exports (`validate_graphs`, `validate_graphs_conforms`,
`lint_data_graph`, `lint_shapes_graph`) are untouched — same signatures, same behavior, same
callers (they back the raw-report toggle and the existing CodeMirror syntax-error gutters).

New exports in `crates/shacl-wasm/src/lib.rs`, all following the existing `to_js_error` /
graph-loading pattern:

- `validate_diagnostics_json(data_graph, shapes_graph, data_format, shapes_format, skip_lint) -> Result<String, JsValue>`
  — JSON **array** (not NDJSON) of full `Diagnostic` objects: `lint_shapes()` (skippable) +
  `from_report()`, `sort_diagnostics()`'d, each via the existing `diagnostic_to_json`.
- `shape_target_nodes_json(data_graph, shapes_graph, data_format, shapes_format) -> Result<String, JsValue>`
  — JSON array of `{ shape: string, targets: [{ node: string, term_kind: "iri"|"blank" }] }`,
  one entry per shape that has at least one target, each `targets` entry a distinct resolved
  focus node (conforming and violating alike). Needs one new small `pub` helper in the core
  crate (see below) — everything else about target resolution is already `pub(crate)`-only
  from the diagnostics work and unreachable from the wasm crate as-is.
- `explain_code_json(code: &str) -> Result<String, JsValue>` — registry lookup as a JSON
  object `{code, title, component, spec_ref, explanation, failing_example, fixed_example}`;
  `Err` (surfaced as a JS exception) when the code doesn't exist, message `"Unknown
  diagnostic code: {code}"`.
- `why_json(data_graph, shapes_graph, data_format, shapes_format, focus_iri, shape_iri) -> Result<String, JsValue>`
  — `shape_iri` is `Option<String>` on the Rust side but wasm-bindgen needs a concrete JS
  type: accept `shape_iri: &str`, treat `""` as "no filter" (matches the CLI's `--shape`
  being optional; empty string is an unambiguous "unset" sentinel here since IRIs are never
  empty). Wraps `explain_conformance`, returns a JSON array of trace `Diagnostic`s (with
  `verdict` populated).

### New core-library helper (small, additive)

`src/diagnostics/mod.rs` (or a new small function in `explain_pass.rs`, re-exported): a `pub`
function wrapping the same target-resolution the `why` tracer already uses, but returning
owned data so it's usable across the FFI boundary:

```rust
pub fn shape_target_nodes<'a>(
    dataset: &'a ValidationDataset,
    shapes: &'a [Shape<'a>],
) -> Vec<(String, Vec<String>)>  // (shape node display string, [focus node display strings])
```

Implementation: for each shape with a non-empty `targets` set, union the resolved focus nodes
across all its targets (reusing `resolve_target`, already `pub(crate)` from the `why` task —
this new function lives in the same crate so it can call it directly) and collect their
Display strings. Shapes with no targets are omitted from the result (nothing to browse).

## Web page layout

Top (unchanged): the two-pane CodeMirror editors (data graph, shapes graph), each with a
format selector, file upload, and the existing syntax-error gutter linting
(`lintDataGraph`/`lintShapesGraph`, untouched).

Toolbar: single **Validate** button + the existing **Skip shape lint** checkbox (already
shipped). The old Output-format dropdown (`text`/`json`/`rdf`/`diagnostics`) is removed from
the primary toolbar — diagnostics is now the only primary output; text/json/rdf move into
the raw-report disclosure (item 6 below), with their own small format selector there.

Results area (replaces the single output textarea):

1. **Summary banner.** A colored bar: green "✓ Conforms" when there are zero results, or red
   "✗ Data does not conform" with a red/yellow error/warning count breakdown otherwise (lint
   findings, having no `focus_node`, don't affect the conforms/violates wording — the banner
   describes the *data*'s conformance, driven by whether any `V`-coded, `Violation`-severity
   diagnostic exists, same signal the CLI's exit code uses).
2. **Diagnostics list.** One card per diagnostic, in the given (already-sorted) order:
   - Header: severity icon (⛔ error / ⚠️ warning / ℹ️ info) + a `code` badge (click → opens
     the Explain panel for that code) + `title`.
   - Body (expanded by default for Error, collapsed for Info — click header to toggle):
     each `snippet` rendered as a monospace block, `turtle` text with the `highlight`
     substring wrapped in a `<mark>` (styled, not ASCII carets — this is real HTML now) and
     `annotation` shown beneath it; then `expected`/`actual` rows when present; then `notes`;
     then `help`.
   - Footer: when `focus_node` is present, a clickable chip "Explain why → `{focus_node}`"
     that opens the Why panel for `(focus_node, source_shape)`.
   - All interpolated text (turtle content, messages, notes — anything that can contain
     characters from the user's own pasted graph) goes through one shared `escapeHtml()`
     helper before insertion; never build HTML via naive string concatenation of raw field
     values.
3. **Shapes & Focus Nodes panel** (collapsible, below the diagnostics list). One row per
   shape from `shape_target_nodes_json`: the shape's display name/IRI, then a wrapped list of
   node chips (one per resolved target). A chip is marked with a small red dot when that
   node+shape pair appears as a `focus_node`/`source_shape` pair in the diagnostics list
   (cross-referenced client-side, no extra wasm call). Clicking any chip — dotted or not —
   opens the Why panel for `(node, shape)`.
4. **Why panel** (slide-over from the right, dismissible; mutually exclusive with the Explain
   panel — opening one closes the other). Calls `why_json` for the clicked `(focus, shape)`.
   Renders a list of trace entries, each with a verdict badge (✅ Conforms / ❌ Violates /
   ⊘ Not targeted / ~ Vacuous) instead of a severity icon, otherwise the same card body
   layout as the diagnostics list (snippets/notes/help) for consistency.
5. **Explain panel** (same slide-over mechanism as Why). Shows `title`, the `component` IRI
   rendered as a link to `spec_ref` (opens the actual W3C spec anchor in a new tab — the one
   place this page links out), `explanation` prose, `failing_example` and `fixed_example` as
   monospace blocks. A small always-visible "Explain a code" text input + button sits above
   the diagnostics list (independent of any diagnostic — mirrors the CLI's standalone
   `explain V0007` working without any graph loaded) and opens this same panel.
6. **Raw report** (collapsed `<details>` by default). Exactly today's Output dropdown
   (text/json/rdf + RDF-format sub-selector) and textarea, calling the untouched
   `validate_graphs`/`validate_graphs_conforms` exports.

No framework, no build step — vanilla DOM/template-string rendering via `document.createElement`
or `innerHTML` with `escapeHtml()`, consistent with the existing `main.js`. The slide-over
panel is a single shared `<aside>` element toggled via the existing `.hidden` class convention
already used for the RDF-format sub-selector.

## MCP integration (deferred — do not implement until asked)

`crates/shacl-mcp/src/main.rs` follows a consistent pattern: an args struct per tool
(`#[derive(..., schemars::JsonSchema)]`, one `#[schemars(description = ...)]` per field), an
`async fn` on `ShaclServer` under `#[tool(description = "...")]`, returning
`Result<String, String>` where success is a JSON string (`json!(...).to_string()`) or a
formatted report string. New tools, same pattern, additive only (existing four tools
untouched):

- **`validate_diagnostics`** — args: `data_graph, shapes_graph, data_format, shapes_format,
  skip_lint: bool`. Returns the JSON array of `Diagnostic`s (`lint_shapes` + `from_report`,
  sorted) as a string — same payload shape as the wasm `validate_diagnostics_json`.
- **`lint_shacl_shapes`** — args: `shapes_graph, shapes_format`. Returns the JSON array of
  lint-only `Diagnostic`s. (Named `lint_shacl_shapes`, not `lint_shapes`, to avoid confusion
  with the existing syntax-only `lint_graph` tool — this one runs the 12 semantic shape-lint
  rules, not a parse check.)
- **`explain_diagnostic_code`** — args: `code: String`. Returns the registry entry as a JSON
  object, or an `Err(String)` ("Unknown diagnostic code: {code}") when not found.
- **`why_conformance`** — args: `data_graph, shapes_graph, data_format, shapes_format,
  focus_node: String, shape: Option<String>`. Returns the JSON array of trace `Diagnostic`s
  (with `verdict`).

All four reuse the same `read_graph_from_string`/`ValidationDataset::from_graphs`/
`parse_shapes` sequence already used by `validate_graphs`, plus the same
`shacl_rust::diagnostics::*` functions the wasm and Python bindings use — no new parsing or
validation logic, purely wiring.

## Testing / verification

- **Web:** no automated test framework exists for `web/` (plain static HTML/JS, no test
  runner). Verification is manual: serve `web/` locally (`python3 -m http.server`) after a
  fresh `wasm-pack build`, and drive it with a real browser (this session has used headless
  Playwright for this purpose already — same approach). At minimum, verify per interaction:
  Validate produces the summary banner + diagnostic cards + shapes/focus-nodes panel; a code
  badge opens Explain with correct content; a focus-node chip (both from a diagnostic card
  and from the Shapes & Focus Nodes panel, including a *conforming* node) opens Why with the
  correct trace; the raw-report disclosure still round-trips text/json/rdf exactly as before.
  `node --check web/main.js` as a cheap syntax gate before the browser pass.
- **MCP:** `crates/shacl-mcp` has no existing tests either. Verification there is manual
  tool-invocation (e.g. via an MCP inspector/test client) confirming each new tool's JSON
  payload matches the same shape the wasm/CLI paths already produce for the same inputs —
  since the underlying `shacl_rust::diagnostics::*` calls are identical, this is a wiring
  check, not a new-logic check.
- **Rust-side gates (both web and MCP tasks):** `cargo test` (full suite, conformance
  120/120), `cargo clippy --all-targets --all-features` (0 warnings), `cargo fmt --all --check`,
  `cargo check -p shacl-wasm --target wasm32-unknown-unknown` (web task) /
  `cargo build -p shacl-mcp` (MCP task).

## Non-goals

- No new framework or build step for `web/`.
- No removal or signature change to any existing wasm or MCP export.
- No automated browser test suite (out of scope; manual/Playwright-assisted verification only,
  consistent with how the existing web demo has always been verified).
- MCP tools are thin wiring over the existing diagnostics API — no new diagnostics logic.
