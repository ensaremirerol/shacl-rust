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
    const shapeIri =
      focusChip.dataset.shape && !focusChip.dataset.shape.startsWith("_:")
        ? focusChip.dataset.shape
        : null;
    openWhyPanel(focusChip.dataset.focus, shapeIri);
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
