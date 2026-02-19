import initWasm, {
  lint_data_graph,
  lint_shapes_graph,
  validate_graphs_all_formats,
  validate_graphs_conforms,
  validate_graphs_json,
  validate_graphs_output,
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

export function validateGraphsJson(dataGraph, shapesGraph, dataFormat, shapesFormat) {
  ensureInit();
  return validate_graphs_json(dataGraph, shapesGraph, dataFormat, shapesFormat);
}

export function validateGraphsOutput(
  dataGraph,
  shapesGraph,
  dataFormat,
  shapesFormat,
  outputFormat
) {
  ensureInit();
  return validate_graphs_output(dataGraph, shapesGraph, dataFormat, shapesFormat, outputFormat);
}

export function validateGraphsAllFormats(
  dataGraph,
  shapesGraph,
  dataFormat,
  shapesFormat,
  graphFormat
) {
  ensureInit();
  return validate_graphs_all_formats(dataGraph, shapesGraph, dataFormat, shapesFormat, graphFormat);
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
