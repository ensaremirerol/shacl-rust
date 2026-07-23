# MCP Diagnostics Tools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the four diagnostics-related MCP tools deferred by
`docs/superpowers/specs/2026-07-23-web-diagnostics-ux-design.md`'s "MCP integration" section to
`crates/shacl-mcp`, giving MCP clients the same diagnostics/lint/explain/why surface the CLI,
Python bindings, and (as of the prior plan) the wasm/web bindings already expose.

**Architecture:** Four new `#[tool]`-annotated async methods on `ShaclServer`
(`crates/shacl-mcp/src/main.rs`), each with its own `schemars::JsonSchema`-deriving args struct,
following the file's existing pattern exactly (`validate_graphs`/`validate_graphs_conforms`/
`lint_graph`/`parse_shapes_graph`, all untouched). Each new tool is thin wiring over
`shacl_rust::diagnostics::*` (already shipped, already used identically by the CLI/Python/wasm
bindings) — no new parsing or validation logic.

**Tech Stack:** Rust (`shacl-rust` core, `shacl-mcp` crate, `rmcp` 0.16 MCP server framework,
`schemars`, `serde_json`), Node.js (verification script only, using the stdio JSON-RPC transport
directly — no new project dependency).

## Global Constraints

- No removal or signature change to any existing MCP tool (`validate_graphs`,
  `validate_graphs_conforms`, `lint_graph`, `parse_shapes_graph`) — new tools are additive only.
- New tool names, exactly: `validate_diagnostics`, `lint_shacl_shapes` (not `lint_shapes` — avoids
  confusion with the existing syntax-only `lint_graph`), `explain_diagnostic_code`,
  `why_conformance`.
- Each tool returns `Result<String, String>` where `Ok` is a JSON string (an array of
  `Diagnostic` objects, or a single registry-entry object for `explain_diagnostic_code`) and `Err`
  is a plain message string — matching the file's existing convention exactly (see
  `validate_graphs_conforms`/`lint_graph`'s `Ok(json!(...).to_string())` pattern).
  `explain_diagnostic_code` on an unknown code must return `Err("Unknown diagnostic code:
  {code}")` — the exact message text the CLI/wasm/Python bindings already use for this case.
  `Diagnostic` JSON payload shape (same `diagnostic_to_json` used everywhere else in the crate):
  `code, severity ("error"|"warning"|"info"), title, constraint_component,
  snippets[{origin: "data"|"shapes", turtle, highlight, annotation}], expected, actual, notes[],
  help, focus_node, source_shape, path, verdict ("conforms"|"violates"|"not-targeted"|"vacuous"|null)`.
- No new tests directory or framework — `shacl-mcp` has no existing tests (confirmed: no
  `#[test]`/`tests/` for this crate) and none are being added; verification is manual
  tool-invocation via a JSON-RPC-over-stdio script, per the spec's stated approach.
- Rust verification gates: `cargo build -p shacl-mcp`, `cargo test` (unaffected, but confirm no
  regression), `cargo clippy --all-targets --all-features` (0 warnings), `cargo fmt --all --check`
  (run this explicitly — a prior plan's wasm task skipped it and shipped an unformatted file that
  only surfaced at final verification; don't repeat that gap here).

---

## File Structure

- Modify: `crates/shacl-mcp/src/main.rs` — add 4 args structs (after the existing
  `ParseShapesGraphArgs`, before `impl Default for ShaclServer`), 4 tool methods (inside the
  `#[tool_router] impl ShaclServer { ... }` block, after `parse_shapes_graph`), and one private
  `trim_angle_brackets` helper (mirroring the CLI's and wasm binding's own identically-named
  copies), placed after the `impl ShaclServer` block closes and before the
  `#[tool_handler] impl ServerHandler` block.

## Task 1: Four new MCP diagnostics tools

**Files:**
- Modify: `crates/shacl-mcp/src/main.rs`

**Interfaces:**
- Consumes: `shacl_rust::diagnostics::{lint_shapes, from_report, sort_diagnostics,
  diagnostic_to_json, entry, explain_conformance}` (all already `pub`, already used identically
  by `crates/shacl-wasm/src/lib.rs` and `crates/shacl-py/src/lib.rs`), `shacl_rust::validate`,
  `shacl_rust::parse_shapes`, `shacl_rust::rdf::read_graph_from_string`,
  `shacl_rust::validation::dataset::ValidationDataset::from_graphs` (all already imported in this
  file), plus `oxigraph::model::{NamedNode, TermRef, NamedOrBlankNodeRef}` (not yet imported —
  add via fully-qualified paths, matching this file's existing style of qualifying
  `oxigraph::io::RdfFormat` inline rather than adding top-level `use` statements for
  occasionally-used types).
- Produces: four new `#[tool]` methods on `ShaclServer`, consumed only by MCP clients (no other
  code in this repo calls them) — `validate_diagnostics`, `lint_shacl_shapes`,
  `explain_diagnostic_code`, `why_conformance`. Verified in Task 2.

- [ ] **Step 1: Add the four args structs**

In `crates/shacl-mcp/src/main.rs`, insert after the existing `ParseShapesGraphArgs` struct
(currently ends at line 69) and before `impl Default for ShaclServer` (currently line 71):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for validating RDF data against SHACL shapes and returning rich diagnostics"
)]
struct ValidateDiagnosticsArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
    #[schemars(
        description = "Skip the 12 semantic shape-lint rules and only return validation diagnostics"
    )]
    skip_lint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for linting a SHACL shapes graph with the semantic shape-lint rules"
)]
struct LintShaclShapesArgs {
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Arguments for explaining a diagnostic code")]
struct ExplainDiagnosticCodeArgs {
    #[schemars(description = "A diagnostic code, e.g. 'V0007' or 'L0003'")]
    code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Arguments for tracing why a focus node does or does not conform to SHACL shapes"
)]
struct WhyConformanceArgs {
    #[schemars(description = "RDF data graph as a string")]
    data_graph: String,
    #[schemars(description = "SHACL shapes graph as a string")]
    shapes_graph: String,
    #[schemars(description = "Format of the data graph (e.g., 'ttl', 'nt', 'jsonld')")]
    data_format: String,
    #[schemars(description = "Format of the shapes graph (e.g., 'ttl', 'nt', 'jsonld')")]
    shapes_format: String,
    #[schemars(description = "IRI of the focus node to trace, e.g. 'http://example.org/alice'")]
    focus_node: String,
    #[schemars(description = "Optional IRI of a single shape to restrict the trace to")]
    shape: Option<String>,
}
```

- [ ] **Step 2: Add the four tool methods**

Inside `#[tool_router] impl ShaclServer { ... }`, insert after the existing `parse_shapes_graph`
method (currently ends at line 190) and before the block's closing `}` (currently line 191):

```rust

    #[tool(
        description = "Validate RDF data against SHACL shapes and return rich rustc-style diagnostics (lint findings plus violation diagnostics), sorted"
    )]
    async fn validate_diagnostics(
        &self,
        Parameters(ValidateDiagnosticsArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
            skip_lint,
        }): Parameters<ValidateDiagnosticsArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let mut diagnostics = if skip_lint {
            Vec::new()
        } else {
            shacl_rust::diagnostics::lint_shapes(validation_dataset.shapes_graph(), &shapes)
        };

        let report = validate(&validation_dataset, &shapes);
        diagnostics.extend(shacl_rust::diagnostics::from_report(
            &report,
            &validation_dataset,
            &shapes,
        ));
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }

    #[tool(
        description = "Run the 12 semantic shape-lint rules against a SHACL shapes graph and return lint diagnostics"
    )]
    async fn lint_shacl_shapes(
        &self,
        Parameters(LintShaclShapesArgs {
            shapes_graph,
            shapes_format,
        }): Parameters<LintShaclShapesArgs>,
    ) -> Result<String, String> {
        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Shapes graph syntax error: {}", e))?;

        let shapes =
            parse_shapes(&shapes_graph).map_err(|e| format!("SHACL shapes error: {}", e))?;

        let mut diagnostics = shacl_rust::diagnostics::lint_shapes(&shapes_graph, &shapes);
        shacl_rust::diagnostics::sort_diagnostics(&mut diagnostics);

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }

    #[tool(
        description = "Look up a diagnostic code (e.g. 'V0007') in the registry and return its title, spec reference, explanation, and a failing/fixed Turtle example pair"
    )]
    async fn explain_diagnostic_code(
        &self,
        Parameters(ExplainDiagnosticCodeArgs { code }): Parameters<ExplainDiagnosticCodeArgs>,
    ) -> Result<String, String> {
        let entry = shacl_rust::diagnostics::entry(&code)
            .ok_or_else(|| format!("Unknown diagnostic code: {}", code))?;

        Ok(json!({
            "code": entry.code,
            "title": entry.title,
            "component": entry.component,
            "spec_ref": entry.spec_ref,
            "explanation": entry.explanation,
            "failing_example": entry.failing_example,
            "fixed_example": entry.fixed_example,
        })
        .to_string())
    }

    #[tool(
        description = "Trace why a focus node does or does not conform to SHACL shapes, constraint by constraint"
    )]
    async fn why_conformance(
        &self,
        Parameters(WhyConformanceArgs {
            data_graph,
            shapes_graph,
            data_format,
            shapes_format,
            focus_node,
            shape,
        }): Parameters<WhyConformanceArgs>,
    ) -> Result<String, String> {
        let data_graph = read_graph_from_string(&data_graph, &data_format)
            .map_err(|e| format!("Failed to parse data graph: {}", e))?;

        let shapes_graph = read_graph_from_string(&shapes_graph, &shapes_format)
            .map_err(|e| format!("Failed to parse shapes graph: {}", e))?;

        let validation_dataset = ValidationDataset::from_graphs(data_graph, shapes_graph)
            .map_err(|e| format!("Failed to create validation dataset: {}", e))?;

        let shapes = parse_shapes(validation_dataset.shapes_graph())
            .map_err(|e| format!("Failed to parse shapes: {}", e))?;

        let focus_trimmed = trim_angle_brackets(&focus_node);
        let focus_named = oxigraph::model::NamedNode::new(focus_trimmed)
            .map_err(|e| format!("Invalid focus IRI '{}': {}", focus_trimmed, e))?;
        let focus_term = validation_dataset
            .data()
            .canonical_term(oxigraph::model::TermRef::from(focus_named.as_ref()))
            .unwrap_or_else(|| oxigraph::model::TermRef::from(focus_named.as_ref()));

        let shape_named = match &shape {
            Some(s) => {
                let shape_trimmed = trim_angle_brackets(s);
                Some(
                    oxigraph::model::NamedNode::new(shape_trimmed)
                        .map_err(|e| format!("Invalid shape IRI '{}': {}", shape_trimmed, e))?,
                )
            }
            None => None,
        };
        let shape_filter = shape_named
            .as_ref()
            .map(|n| oxigraph::model::NamedOrBlankNodeRef::from(n.as_ref()));

        let diagnostics = shacl_rust::diagnostics::explain_conformance(
            &validation_dataset,
            &shapes,
            focus_term,
            shape_filter,
        );

        let json_array: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(shacl_rust::diagnostics::diagnostic_to_json)
            .collect();
        Ok(serde_json::Value::Array(json_array).to_string())
    }
```

- [ ] **Step 3: Add the `trim_angle_brackets` helper**

After the `impl ShaclServer { ... }` block's closing `}` (currently line 191) and before the
`// Implement the server handler` comment (currently line 193), insert:

```rust

/// Trims a single pair of optional surrounding angle brackets (`<...>`) from
/// an IRI argument, mirroring the CLI's and wasm binding's own identically
/// named helper so a caller can pass either `http://example.org/a` or
/// `<http://example.org/a>` (e.g. a `focus_node`/`source_shape` display
/// string copied straight from another tool's diagnostic output, which is
/// always bracket-wrapped for IRIs).
fn trim_angle_brackets(s: &str) -> &str {
    s.trim().trim_start_matches('<').trim_end_matches('>')
}
```

- [ ] **Step 4: Verify it compiles cleanly**

Run: `cargo build -p shacl-mcp`
Expected: `Finished` with no errors.

Run: `cargo clippy -p shacl-mcp --all-targets --all-features`
Expected: 0 warnings.

Run: `cargo fmt --all --check`
Expected: no output (already formatted) — run this explicitly and fix with `cargo fmt --all` if
it reports anything; do not skip it.

- [ ] **Step 5: Commit**

```bash
git add crates/shacl-mcp/src/main.rs
git commit -m "feat(mcp): add validate_diagnostics/lint_shacl_shapes/explain_diagnostic_code/why_conformance tools"
```

## Task 2: Manual verification and final gates

**Files:** none (build + verification only)

**Interfaces:** none.

- [ ] **Step 1: Build the binary**

Run: `cargo build -p shacl-mcp`
Expected: `Finished`, binary at `target/debug/shacl-mcp`.

- [ ] **Step 2: Write and run a JSON-RPC-over-stdio verification script**

There is no MCP test framework in this repo; `shacl-mcp` speaks newline-delimited JSON-RPC 2.0
over stdio (the `rmcp` `stdio()` transport). Create `/tmp/pw-check/verify-mcp.js`:

```js
const { spawn } = require("node:child_process");

const BIN = process.argv[2];
if (!BIN) {
  console.error("usage: node verify-mcp.js <path-to-shacl-mcp-binary>");
  process.exit(2);
}

const DATA_GRAPH = `@prefix ex: <http://example.com/> .
ex:alice a ex:Person ;
  ex:age 17 .
`;
const SHAPES_GRAPH = `@prefix sh: <http://www.w3.org/ns/shacl#> .
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

const child = spawn(BIN, [], { stdio: ["pipe", "pipe", "pipe"] });

let buffer = "";
const pending = new Map();
let nextId = 1;

child.stdout.on("data", (chunk) => {
  buffer += chunk.toString("utf8");
  let idx;
  while ((idx = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, idx);
    buffer = buffer.slice(idx + 1);
    if (!line.trim()) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      console.error("NON_JSON_LINE:", line);
      continue;
    }
    if (msg.id != null && pending.has(msg.id)) {
      const { resolve } = pending.get(msg.id);
      pending.delete(msg.id);
      resolve(msg);
    }
  }
});

child.stderr.on("data", (chunk) => {
  process.stderr.write(`[server stderr] ${chunk}`);
});

function send(method, params) {
  return new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, { resolve });
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
  });
}

function notify(method, params) {
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
}

(async () => {
  const initResp = await send("initialize", {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "verify-mcp", version: "0.0.0" },
  });
  console.log("INIT_OK:", initResp.result != null && initResp.error == null);
  console.log("INIT_RAW:", JSON.stringify(initResp));

  notify("notifications/initialized", {});

  const listResp = await send("tools/list", {});
  const names = (listResp.result?.tools ?? []).map((t) => t.name).sort();
  console.log("TOOLS:", JSON.stringify(names));
  const expected = [
    "explain_diagnostic_code",
    "lint_graph",
    "lint_shacl_shapes",
    "parse_shapes_graph",
    "validate_diagnostics",
    "validate_graphs",
    "validate_graphs_conforms",
    "why_conformance",
  ];
  console.log("TOOLS_MATCH_EXPECTED:", JSON.stringify(names) === JSON.stringify(expected));

  const validateResp = await send("tools/call", {
    name: "validate_diagnostics",
    arguments: {
      data_graph: DATA_GRAPH,
      shapes_graph: SHAPES_GRAPH,
      data_format: "ttl",
      shapes_format: "ttl",
      skip_lint: false,
    },
  });
  console.log("VALIDATE_DIAGNOSTICS_RAW:", JSON.stringify(validateResp));
  const validateText = validateResp.result?.content?.[0]?.text ?? "";
  let validateJson;
  try {
    validateJson = JSON.parse(validateText);
  } catch {
    validateJson = null;
  }
  console.log(
    "VALIDATE_DIAGNOSTICS:",
    "isArray=" + Array.isArray(validateJson),
    "hasVCode=" +
      (Array.isArray(validateJson) && validateJson.some((d) => /^V\d{4}$/.test(d.code))),
    "sample=" + JSON.stringify(validateJson?.[0])
  );

  const lintResp = await send("tools/call", {
    name: "lint_shacl_shapes",
    arguments: { shapes_graph: SHAPES_GRAPH, shapes_format: "ttl" },
  });
  const lintText = lintResp.result?.content?.[0]?.text ?? "";
  let lintJson;
  try {
    lintJson = JSON.parse(lintText);
  } catch {
    lintJson = null;
  }
  console.log(
    "LINT_SHACL_SHAPES:",
    "isArray=" + Array.isArray(lintJson),
    "count=" + lintJson?.length
  );

  const explainResp = await send("tools/call", {
    name: "explain_diagnostic_code",
    arguments: { code: "V0007" },
  });
  const explainText = explainResp.result?.content?.[0]?.text ?? "";
  let explainJson;
  try {
    explainJson = JSON.parse(explainText);
  } catch {
    explainJson = null;
  }
  console.log(
    "EXPLAIN_DIAGNOSTIC_CODE:",
    "code=" + explainJson?.code,
    "hasSpecRef=" + !!explainJson?.spec_ref,
    "hasFailingExample=" + !!explainJson?.failing_example
  );

  const explainUnknownResp = await send("tools/call", {
    name: "explain_diagnostic_code",
    arguments: { code: "V9999" },
  });
  console.log(
    "EXPLAIN_UNKNOWN_IS_ERROR:",
    explainUnknownResp.result?.isError === true,
    "text=" + JSON.stringify(explainUnknownResp.result?.content?.[0]?.text)
  );

  const whyResp = await send("tools/call", {
    name: "why_conformance",
    arguments: {
      data_graph: DATA_GRAPH,
      shapes_graph: SHAPES_GRAPH,
      data_format: "ttl",
      shapes_format: "ttl",
      focus_node: "http://example.com/alice",
      shape: null,
    },
  });
  console.log("WHY_CONFORMANCE_RAW:", JSON.stringify(whyResp));
  const whyText = whyResp.result?.content?.[0]?.text ?? "";
  let whyJson;
  try {
    whyJson = JSON.parse(whyText);
  } catch {
    whyJson = null;
  }
  console.log(
    "WHY_CONFORMANCE:",
    "isArray=" + Array.isArray(whyJson),
    "hasViolates=" + (Array.isArray(whyJson) && whyJson.some((d) => d.verdict === "violates")),
    "hasVerdictField=" + (Array.isArray(whyJson) && whyJson.every((d) => "verdict" in d))
  );

  child.stdin.end();
  child.kill();
})().catch((err) => {
  console.error("SCRIPT_FAILED:", err);
  child.kill();
  process.exit(1);
});
```

Run: `node /tmp/pw-check/verify-mcp.js /home/ensar/projects/shacl-rust/target/debug/shacl-mcp`

Expected (adapt field-path assumptions if the raw response logs show a different shape than
assumed — `rmcp`'s exact `tools/call` result envelope isn't pinned down by a project convention
elsewhere in this repo, so confirm against the `*_RAW:` log lines first):
- `INIT_OK: true`
- `TOOLS_MATCH_EXPECTED: true` (8 tools total: the 4 pre-existing plus the 4 new ones)
- `VALIDATE_DIAGNOSTICS: isArray=true hasVCode=true` with a `sample` showing a real `Diagnostic`
  object (`code`, `severity`, `title`, etc. fields present)
- `LINT_SHACL_SHAPES: isArray=true count=<some number>=0` (this shapes graph is well-formed, so
  an empty or near-empty array is fine — the check is that it's a valid array, not a specific
  count)
- `EXPLAIN_DIAGNOSTIC_CODE: code=V0007 hasSpecRef=true hasFailingExample=true`
- `EXPLAIN_UNKNOWN_IS_ERROR: true text="Unknown diagnostic code: V9999"`
- `WHY_CONFORMANCE: isArray=true hasViolates=true hasVerdictField=true`

If any tool's actual response doesn't match the JSON payload shape the CLI/wasm/Python bindings
produce for the same inputs, that's a real bug — do not adjust the plan's intended shape to match
a broken implementation; fix the Rust code.

- [ ] **Step 3: Run the final Rust gates**

Run: `cargo test`
Expected: full suite passes, conformance suite 120/120 (unaffected by this change, confirming no
regression).

Run: `cargo clippy --all-targets --all-features`
Expected: 0 warnings.

Run: `cargo fmt --all --check`
Expected: no output.

Run: `cargo build -p shacl-mcp`
Expected: `Finished` with no errors (already built in Step 1, re-confirm clean after any Task 1
fixes).

- [ ] **Step 4: No commit for this task**

This task only builds and verifies; nothing here is checked into git (the `/tmp/pw-check/verify-mcp.js`
script is scratch tooling, not project code). If verification surfaced a real bug requiring a
source fix, that fix should already have been committed as part of Task 1 — amend that task's
work rather than leaving an uncommitted fix here.
