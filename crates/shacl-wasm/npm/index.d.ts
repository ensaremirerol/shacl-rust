export type InputFormat = "ttl" | "nt" | "rdf" | "jsonld" | "trig";
export type OutputFormat = "text" | "json" | InputFormat;

export declare function init(input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<void>;

export declare function validateGraphsOutput(
  dataGraph: string,
  shapesGraph: string,
  dataFormat: InputFormat,
  shapesFormat: InputFormat,
  outputFormat: OutputFormat
): string;


export declare function validateGraphsConforms(
  dataGraph: string,
  shapesGraph: string,
  dataFormat: InputFormat,
  shapesFormat: InputFormat
): boolean;

export declare function lintDataGraph(dataGraph: string, dataFormat: InputFormat): void;
export declare function lintShapesGraph(shapesGraph: string, shapesFormat: InputFormat): void;
