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

const statusEl = document.getElementById("status");
const validateBtn = document.getElementById("validate-btn");
const dataFileEl = document.getElementById("data-file");
const shapesFileEl = document.getElementById("shapes-file");
const dataFormatEl = document.getElementById("data-format");
const shapesFormatEl = document.getElementById("shapes-format");
const outputTypeEl = document.getElementById("output-type");
const rdfOutputLabelEl = document.getElementById("rdf-output-label");
const rdfOutputFormatEl = document.getElementById("rdf-output-format");
const outputEl = document.getElementById("output");

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
let validateGraphsOutput = null;
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

function validateNow() {
  if (!wasmReady || !validateGraphsOutput) {
    setStatus("WASM is not ready yet.", "err");
    return;
  }

  validateBtn.disabled = true;
  setStatus("Validating...", "ok");

  try {
    const result = validateGraphsOutput(
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

    setStatus("Validation completed.", "ok");
  } catch (error) {
    outputEl.value = "";
    setStatus(`Validation failed: ${error}`, "err");
  } finally {
    validateBtn.disabled = false;
  }
}

async function loadWasmModule() {
  const moduleUrl = new URL("./pkg/shacl_wasm.js", import.meta.url).href;
  const wasmModule = await import(moduleUrl);
  wasmInit = wasmModule.default;
  validateGraphsOutput = wasmModule.validate_graphs_output;
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
  validateBtn.addEventListener("click", validateNow);

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
