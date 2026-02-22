import initWasm, {
  lint_data_graph,
  lint_shapes_graph,
  validate_graphs,
  validate_graphs_conforms
} from "./shacl_wasm.js";

let initialized = false;

export async function init(input) {
  if (!initialized) {
    await initWasm(input);
    initialized = true;
  }
}

function ensureInit() {
  if (!initialized) {
    throw new Error("shacl-wasm is not initialized. Call init() first.");
  }
}

export function validateGraphs(
  dataGraph,
  shapesGraph,
  dataFormat,
  shapesFormat,
  outputFormat
) {
  ensureInit();
  return validate_graphs(dataGraph, shapesGraph, dataFormat, shapesFormat, outputFormat);
}

export function validateGraphsConforms(dataGraph, shapesGraph, dataFormat, shapesFormat) {
  ensureInit();
  return validate_graphs_conforms(dataGraph, shapesGraph, dataFormat, shapesFormat);
}

export function lintDataGraph(dataGraph, dataFormat) {
  ensureInit();
  return lint_data_graph(dataGraph, dataFormat);
}

export function lintShapesGraph(shapesGraph, shapesFormat) {
  ensureInit();
  return lint_shapes_graph(shapesGraph, shapesFormat);
}
