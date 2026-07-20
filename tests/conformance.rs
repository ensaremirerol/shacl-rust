use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{vocab::rdf, Graph, NamedNodeRef, NamedOrBlankNodeRef, TermRef, Triple};
use shacl_rust::{parser, validation};
use std::collections::HashSet;
use std::error::Error;
use std::io::BufReader;
use std::path::{Path, PathBuf};

// Vocabulary for test manifests
mod mf {
    use oxigraph::model::NamedNodeRef;
    pub const MANIFEST: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#Manifest",
    );
    pub const ENTRIES: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#entries",
    );
    pub const INCLUDE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#include",
    );
    pub const ACTION: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#action",
    );
    pub const RESULT: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#result",
    );
    pub const STATUS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked(
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#status",
    );
}

mod sht {
    use oxigraph::model::NamedNodeRef;
    pub const VALIDATE: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl-test#Validate");
    pub const DATA_GRAPH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl-test#dataGraph");
    pub const SHAPES_GRAPH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl-test#shapesGraph");
    pub const APPROVED: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl-test#approved");
    pub const FAILURE: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl-test#Failure");
}

mod sh {
    use oxigraph::model::NamedNodeRef;
    pub const VALIDATION_REPORT: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ValidationReport");
    pub const CONFORMS: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#conforms");
    pub const RESULT: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#result");
    pub const FOCUS_NODE: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#focusNode");
    pub const RESULT_SEVERITY: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultSeverity");
    pub const SOURCE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#sourceConstraintComponent");
    pub const RESULT_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultPath");
    pub const INVERSE_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#inversePath");
    pub const ALTERNATIVE_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#alternativePath");
    pub const ZERO_OR_MORE_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#zeroOrMorePath");
    pub const ONE_OR_MORE_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#oneOrMorePath");
    pub const ZERO_OR_ONE_PATH: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#zeroOrOnePath");
    pub const VIOLATION: NamedNodeRef<'_> =
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Violation");
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ExpectedOutcome {
    Conforms(bool),
    Failure,
}

/// One expected sh:result, keyed the way the report-fidelity comparison works:
/// (focusNode, sourceConstraintComponent, resultPath, severity), messages and
/// values ignored. Blank-node focus nodes are wildcarded as "_:" since labels
/// are not stable across parses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ResultKey {
    focus: String,
    component: String,
    path: String,
    severity: String,
}

fn term_key(term: TermRef) -> String {
    match term {
        TermRef::BlankNode(_) => "_:".to_string(),
        other => other.to_string(),
    }
}

/// Renders a manifest sh:resultPath structure in the same syntax
/// `shacl_rust::Path`'s Display produces, so both sides compare as strings.
fn render_manifest_path(graph: &Graph, term: TermRef) -> Option<String> {
    match term {
        TermRef::NamedNode(nn) => Some(format!("{}", nn)),
        TermRef::BlankNode(bn) => {
            let node = NamedOrBlankNodeRef::BlankNode(bn);
            if let Some(inner) = graph.object_for_subject_predicate(node, sh::INVERSE_PATH) {
                return Some(format!("^{}", render_manifest_path(graph, inner)?));
            }
            if let Some(inner) = graph.object_for_subject_predicate(node, sh::ALTERNATIVE_PATH) {
                let alts = parse_rdf_list_nodes(graph, inner)?
                    .into_iter()
                    .map(|t| render_manifest_path(graph, t))
                    .collect::<Option<Vec<_>>>()?;
                return Some(format!("({})", alts.join(" | ")));
            }
            if let Some(inner) = graph.object_for_subject_predicate(node, sh::ZERO_OR_MORE_PATH) {
                return Some(format!("({}*)", render_manifest_path(graph, inner)?));
            }
            if let Some(inner) = graph.object_for_subject_predicate(node, sh::ONE_OR_MORE_PATH) {
                return Some(format!("({}+)", render_manifest_path(graph, inner)?));
            }
            if let Some(inner) = graph.object_for_subject_predicate(node, sh::ZERO_OR_ONE_PATH) {
                return Some(format!("({}?)", render_manifest_path(graph, inner)?));
            }
            // RDF list -> sequence path
            let elements = parse_rdf_list_nodes(graph, term)?
                .into_iter()
                .map(|t| render_manifest_path(graph, t))
                .collect::<Option<Vec<_>>>()?;
            Some(elements.join(" / "))
        }
        TermRef::Literal(_) => None,
    }
}

fn parse_rdf_list_nodes<'a>(graph: &'a Graph, head: TermRef<'a>) -> Option<Vec<TermRef<'a>>> {
    let mut items = Vec::new();
    let mut current = head;
    loop {
        match current {
            TermRef::NamedNode(nn) if nn == rdf::NIL => return Some(items),
            TermRef::NamedNode(nn) => {
                let node = NamedOrBlankNodeRef::NamedNode(nn);
                items.push(graph.object_for_subject_predicate(node, rdf::FIRST)?);
                current = graph.object_for_subject_predicate(node, rdf::REST)?;
            }
            TermRef::BlankNode(bn) => {
                let node = NamedOrBlankNodeRef::BlankNode(bn);
                items.push(graph.object_for_subject_predicate(node, rdf::FIRST)?);
                current = graph.object_for_subject_predicate(node, rdf::REST)?;
            }
            TermRef::Literal(_) => return None,
        }
    }
}

fn parse_expected_results(graph: &Graph, report_node: NamedOrBlankNodeRef) -> Vec<ResultKey> {
    let mut keys = Vec::new();
    for result_term in graph.objects_for_subject_predicate(report_node, sh::RESULT) {
        let result_node = match result_term {
            TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
            TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
            _ => continue,
        };
        let focus = graph
            .object_for_subject_predicate(result_node, sh::FOCUS_NODE)
            .map(term_key)
            .unwrap_or_default();
        let component = graph
            .object_for_subject_predicate(result_node, sh::SOURCE_CONSTRAINT_COMPONENT)
            .map(|t| t.to_string())
            .unwrap_or_default();
        let severity = graph
            .object_for_subject_predicate(result_node, sh::RESULT_SEVERITY)
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!("{}", sh::VIOLATION));
        let path = graph
            .object_for_subject_predicate(result_node, sh::RESULT_PATH)
            .and_then(|t| render_manifest_path(graph, t))
            .unwrap_or_default();
        keys.push(ResultKey {
            focus,
            component,
            path,
            severity,
        });
    }
    keys
}

fn actual_result_keys(report: &shacl_rust::ValidationReport) -> Vec<ResultKey> {
    let json = report.as_json();
    json["results"]
        .as_array()
        .map(|results| {
            results
                .iter()
                .map(|r| {
                    let focus_raw = r["focusNode"].as_str().unwrap_or_default();
                    ResultKey {
                        focus: if focus_raw.starts_with("_:") {
                            "_:".to_string()
                        } else {
                            focus_raw.to_string()
                        },
                        component: r["sourceConstraintComponent"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        path: r["resultPath"].as_str().unwrap_or_default().to_string(),
                        severity: r["severity"].as_str().unwrap_or_default().to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Multiset difference: (missing from actual, unexpected in actual).
fn diff_result_sets(expected: &[ResultKey], actual: &[ResultKey]) -> (Vec<ResultKey>, Vec<ResultKey>) {
    let mut missing = expected.to_vec();
    let mut extra = Vec::new();
    for a in actual {
        if let Some(pos) = missing.iter().position(|e| e == a) {
            missing.remove(pos);
        } else {
            extra.push(a.clone());
        }
    }
    (missing, extra)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestCase {
    uri: String,
    label: Option<String>,
    data_graph_file: PathBuf,
    shapes_graph_file: PathBuf,
    expected_outcome: ExpectedOutcome,
    expected_results: Vec<ResultKey>,
}

fn parse_rdf_list<'a>(graph: &'a Graph, list_node: NamedOrBlankNodeRef<'a>) -> Vec<TermRef<'a>> {
    let mut items = Vec::new();
    let mut current = list_node;
    let mut visited = HashSet::new();

    let nil = NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

    loop {
        // Check for cycles
        if !visited.insert(current) {
            break;
        }

        // Check if current is rdf:nil
        if let NamedOrBlankNodeRef::NamedNode(nn) = current {
            if nn == nil {
                break;
            }
        }

        // Get rdf:first
        if let Some(first) = graph.object_for_subject_predicate(current, rdf::FIRST) {
            items.push(first);
        }

        // Get rdf:rest
        if let Some(rest) = graph.object_for_subject_predicate(current, rdf::REST) {
            match rest {
                TermRef::NamedNode(nn) => {
                    if nn == nil {
                        break;
                    }
                    current = NamedOrBlankNodeRef::NamedNode(nn);
                }
                TermRef::BlankNode(bn) => {
                    current = NamedOrBlankNodeRef::BlankNode(bn);
                }
                _ => break,
            }
        } else {
            break;
        }

        // Safety limit: stop after processing 10000 items
        if items.len() > 10000 {
            break;
        }
    }

    items
}

fn resolve_graph_file(base_file: &Path, graph_ref: TermRef) -> Option<PathBuf> {
    match graph_ref {
        TermRef::NamedNode(nn) => {
            let uri = nn.as_str();

            // Handle file:// URIs
            if let Some(path_str) = uri.strip_prefix("file://") {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Some(path);
                }
                // If the file:// path doesn't exist as-is, try normalizing it
                if let Ok(canonical_base) = base_file.canonicalize() {
                    if path == canonical_base {
                        return Some(base_file.to_path_buf());
                    }
                }
            }

            // Check for self-reference (empty or matches base file)
            if uri.is_empty() {
                return Some(base_file.to_path_buf());
            }

            // Try as relative path from base directory
            if let Some(base_dir) = base_file.parent() {
                let relative = base_dir.join(uri);
                if relative.exists() {
                    return Some(relative);
                }

                // Try just the filename
                if let Some(filename) = uri.split('/').next_back() {
                    let candidate = base_dir.join(filename);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }

            None
        }
        _ => None,
    }
}

fn load_test_cases_from_manifest(manifest_file: &Path) -> Vec<TestCase> {
    let mut test_cases = Vec::new();
    let mut visited_files = HashSet::new();

    collect_test_cases_recursive(manifest_file, &mut test_cases, &mut visited_files);

    test_cases
}

fn read_graph_file(path: &Path) -> Result<Graph, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let format_ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            format!(
                "Failed to infer RDF format from file extension: {}",
                path.display()
            )
        })?;

    let rdf_format = RdfFormat::from_extension(format_ext).ok_or_else(|| {
        format!(
            "Unsupported RDF format extension '{}' for file {}",
            format_ext,
            path.display()
        )
    })?;

    let canonical = path.canonicalize()?;
    let base_iri = format!("file://{}", canonical.to_string_lossy());

    let parser = RdfParser::from_format(rdf_format).with_base_iri(&base_iri)?;
    let quads = parser
        .for_reader(BufReader::new(content.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut graph = Graph::new();
    graph.extend(quads.into_iter().map(Triple::from));
    Ok(graph)
}

fn collect_test_cases_recursive(
    manifest_file: &Path,
    test_cases: &mut Vec<TestCase>,
    visited_files: &mut HashSet<PathBuf>,
) {
    if visited_files.contains(manifest_file) {
        return;
    }
    visited_files.insert(manifest_file.to_path_buf());

    let graph = match read_graph_file(manifest_file) {
        Ok(g) => g,
        _ => {
            eprintln!("Failed to read manifest file: {}", manifest_file.display());
            return;
        }
    };

    // Find all manifest nodes
    let manifests: Vec<_> = graph
        .subjects_for_predicate_object(rdf::TYPE, mf::MANIFEST)
        .collect();

    for manifest_node in manifests {
        // Process includes
        for include_ref in graph.objects_for_subject_predicate(manifest_node, mf::INCLUDE) {
            if let Some(include_file) = resolve_graph_file(manifest_file, include_ref) {
                if include_file.exists() {
                    collect_test_cases_recursive(&include_file, test_cases, visited_files);
                }
            }
        }

        // Process entries
        for entries_ref in graph.objects_for_subject_predicate(manifest_node, mf::ENTRIES) {
            if let TermRef::BlankNode(bn) = entries_ref {
                let entries = parse_rdf_list(&graph, NamedOrBlankNodeRef::BlankNode(bn));
                for entry in entries {
                    if let Some(test_case) = parse_test_case(&graph, entry, manifest_file) {
                        test_cases.push(test_case);
                    }
                }
            }
        }
    }
}

fn parse_test_case(graph: &Graph, test_node: TermRef, base_file: &Path) -> Option<TestCase> {
    let test_subject = match test_node {
        TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
        TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
        _ => return None,
    };

    // Check if this is a Validate test
    let is_validate = graph
        .objects_for_subject_predicate(test_subject, rdf::TYPE)
        .any(|t| t == sht::VALIDATE.into());

    if !is_validate {
        return None;
    }

    // Check status - only run approved tests
    let is_approved = graph
        .objects_for_subject_predicate(test_subject, mf::STATUS)
        .any(|t| t == sht::APPROVED.into());

    if !is_approved {
        return None;
    }

    // Get label
    let label = graph
        .object_for_subject_predicate(
            test_subject,
            NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#label"),
        )
        .and_then(|t| match t {
            TermRef::Literal(lit) => Some(lit.value().to_string()),
            _ => None,
        });

    // Get action (contains data and shapes graphs)
    let action = graph.object_for_subject_predicate(test_subject, mf::ACTION)?;
    let action_node = match action {
        TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
        _ => return None,
    };

    let data_graph_ref = graph.object_for_subject_predicate(action_node, sht::DATA_GRAPH)?;
    let shapes_graph_ref = graph.object_for_subject_predicate(action_node, sht::SHAPES_GRAPH)?;

    let data_graph_file = resolve_graph_file(base_file, data_graph_ref)?;
    let shapes_graph_file = resolve_graph_file(base_file, shapes_graph_ref)?;

    // Get expected result
    let result = graph.object_for_subject_predicate(test_subject, mf::RESULT)?;
    let expected_outcome = match result {
        TermRef::NamedNode(nn) if nn == sht::FAILURE => ExpectedOutcome::Failure,
        TermRef::BlankNode(bn) => {
            let result_node = NamedOrBlankNodeRef::BlankNode(bn);

            // Check if result is a ValidationReport
            let is_report = graph
                .objects_for_subject_predicate(result_node, rdf::TYPE)
                .any(|t| t == sh::VALIDATION_REPORT.into());

            if !is_report {
                return None;
            }

            // Get conforms value
            let conforms_value = graph.object_for_subject_predicate(result_node, sh::CONFORMS)?;
            let expected_conforms = match conforms_value {
                TermRef::Literal(lit) => lit.value() == "true",
                _ => return None,
            };

            ExpectedOutcome::Conforms(expected_conforms)
        }
        _ => return None,
    };

    let expected_results = match result {
        TermRef::BlankNode(bn) => {
            parse_expected_results(graph, NamedOrBlankNodeRef::BlankNode(bn))
        }
        _ => Vec::new(),
    };

    Some(TestCase {
        uri: test_subject.to_string(),
        label,
        data_graph_file,
        shapes_graph_file,
        expected_outcome,
        expected_results,
    })
}

fn find_manifest_files(base_dir: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();

    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("manifest.ttl") {
                manifests.push(path);
            } else if path.is_dir() {
                manifests.extend(find_manifest_files(&path));
            }
        }
    }

    manifests
}

#[test]
fn test_shacl_conformance() {
    println!("Starting SHACL conformance test...");
    let resources_dir = Path::new("tests/resources");

    if !resources_dir.exists() {
        panic!("Resources directory not found: {}", resources_dir.display());
    }

    println!("Finding manifest files...");
    let mut manifest_files = find_manifest_files(resources_dir);

    if manifest_files.is_empty() {
        panic!("No manifest files found in {}", resources_dir.display());
    }

    // Sort manifest files for consistent ordering
    manifest_files.sort();

    println!("\nFound {} manifest file(s)", manifest_files.len());

    println!("Loading test cases from manifests...");
    let mut all_test_cases = Vec::new();
    for (i, manifest_file) in manifest_files.iter().enumerate() {
        println!(
            "Loading manifest {}/{}: {}",
            i + 1,
            manifest_files.len(),
            manifest_file.display()
        );
        let test_cases = load_test_cases_from_manifest(manifest_file);
        println!(
            "Loaded {} test cases from {}",
            test_cases.len(),
            manifest_file.display()
        );
        all_test_cases.extend(test_cases);
    }

    // Deduplicate test cases by URI
    let mut unique_cases = HashSet::new();
    all_test_cases.retain(|tc| unique_cases.insert(tc.uri.clone()));

    // Sort test cases by label (or URI if no label) for consistent ordering
    all_test_cases.sort_by(|a, b| {
        let key_a = a.label.as_deref().unwrap_or(&a.uri);
        let key_b = b.label.as_deref().unwrap_or(&b.uri);
        key_a.cmp(key_b)
    });

    println!("\nTotal: {} test cases\n", all_test_cases.len());

    if all_test_cases.is_empty() {
        panic!("No test cases found!");
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for test_case in &all_test_cases {
        let test_name = test_case.label.as_deref().unwrap_or(&test_case.uri);

        println!("Processing: {}", test_name);

        // Skip if files don't exist
        if !test_case.data_graph_file.exists() {
            println!(
                "🚫 SKIP: {} (data file not found: {})",
                test_name,
                test_case.data_graph_file.display()
            );
            skipped += 1;
            continue;
        }
        if !test_case.shapes_graph_file.exists() {
            println!(
                "🚫 SKIP: {} (shapes file not found: {})",
                test_name,
                test_case.shapes_graph_file.display()
            );
            skipped += 1;
            continue;
        }

        // Run actual validation
        match (
            read_graph_file(&test_case.data_graph_file),
            read_graph_file(&test_case.shapes_graph_file),
        ) {
            (Ok(data_graph), Ok(shapes_graph)) => {
                match parser::parse_shapes(&shapes_graph) {
                    Ok(shapes) => {
                        // Create validation dataset
                        let validation_dataset =
                            match validation::dataset::ValidationDataset::from_graphs(
                                data_graph.clone(),
                                shapes_graph.clone(),
                            ) {
                                Ok(dataset) => dataset,
                                Err(e) => {
                                    println!(
                                        "❌ FAIL: {} (failed to create validation dataset: {})",
                                        test_name, e
                                    );
                                    failed += 1;
                                    continue;
                                }
                            };

                        // Run validation
                        let report = validation::validate(&validation_dataset, &shapes);

                        // Also validate through the experimental indexed
                        // backend; both backends must agree on conformance.
                        let indexed_dataset =
                            match validation::dataset::ValidationDataset::from_graphs_with_experimental_index(
                                data_graph.clone(),
                                shapes_graph.clone(),
                            ) {
                                Ok(dataset) => dataset,
                                Err(e) => {
                                    println!(
                                        "\u{274c} FAIL: {} (failed to create indexed validation dataset: {})",
                                        test_name, e
                                    );
                                    failed += 1;
                                    continue;
                                }
                            };
                        let indexed_report = validation::validate(&indexed_dataset, &shapes);
                        if indexed_report.get_conforms() != report.get_conforms() {
                            println!(
                                "\u{274c} FAIL: {} (indexed backend disagrees: plain conforms={}, indexed conforms={})",
                                test_name,
                                report.get_conforms(),
                                indexed_report.get_conforms()
                            );
                            failed += 1;
                            continue;
                        }

                        match test_case.expected_outcome {
                            ExpectedOutcome::Conforms(expected_conforms) => {
                                if *report.get_conforms() == expected_conforms {
                                    // Report fidelity: the emitted result set must
                                    // match the manifest's expected multiset keyed
                                    // on (focus, component, path, severity).
                                    let actual = actual_result_keys(&report);
                                    let (missing, extra) =
                                        diff_result_sets(&test_case.expected_results, &actual);
                                    if !missing.is_empty() || !extra.is_empty() {
                                        println!(
                                            "❌ FAIL: {} (conforms ok, but result set differs: expected {}, got {})",
                                            test_name,
                                            test_case.expected_results.len(),
                                            actual.len()
                                        );
                                        for m in missing.iter().take(4) {
                                            println!("    missing: {:?}", m);
                                        }
                                        for e in extra.iter().take(4) {
                                            println!("    extra:   {:?}", e);
                                        }
                                        failed += 1;
                                        continue;
                                    }
                                    println!(
                                        "✅ PASS: {} (conforms: {}, {} shapes, {} results)",
                                        test_name,
                                        *report.get_conforms(),
                                        shapes.len(),
                                        report.get_results().len()
                                    );
                                    passed += 1;
                                } else {
                                    println!(
                                        "❌ FAIL: {} (expected conforms: {}, got: {}, {} results)",
                                        test_name,
                                        expected_conforms,
                                        *report.get_conforms(),
                                        report.get_results().len()
                                    );
                                    for (i, result) in
                                        report.get_results().iter().take(3).enumerate()
                                    {
                                        println!("  Result {}: {:?}", i + 1, result.get_repr());
                                    }
                                    failed += 1;
                                }
                            }
                            ExpectedOutcome::Failure => {
                                if !*report.get_conforms() {
                                    println!(
                                        "✅ PASS: {} (expected failure observed, {} shapes, {} results)",
                                        test_name,
                                        shapes.len(),
                                        report.get_results().len()
                                    );
                                    passed += 1;
                                } else {
                                    println!(
                                        "❌ FAIL: {} (expected failure, got conforms: true)",
                                        test_name
                                    );
                                    failed += 1;
                                }
                            }
                        }
                    }
                    Err(e) => match test_case.expected_outcome {
                        ExpectedOutcome::Failure => {
                            println!(
                                "✅ PASS: {} (expected failure via parse error: {})",
                                test_name, e
                            );
                            passed += 1;
                        }
                        ExpectedOutcome::Conforms(_) => {
                            println!("❌ FAIL: {} (parse error: {})", test_name, e);
                            failed += 1;
                        }
                    },
                }
            }
            (Err(e), _) => match test_case.expected_outcome {
                ExpectedOutcome::Failure => {
                    println!(
                        "✅ PASS: {} (expected failure via data read error: {})",
                        test_name, e
                    );
                    passed += 1;
                }
                ExpectedOutcome::Conforms(_) => {
                    println!("❌ FAIL: {} (data read error: {})", test_name, e);
                    failed += 1;
                }
            },
            (_, Err(e)) => match test_case.expected_outcome {
                ExpectedOutcome::Failure => {
                    println!(
                        "✅ PASS: {} (expected failure via shapes read error: {})",
                        test_name, e
                    );
                    passed += 1;
                }
                ExpectedOutcome::Conforms(_) => {
                    println!("❌ FAIL: {} (shapes read error: {})", test_name, e);
                    failed += 1;
                }
            },
        }
    }

    println!("\n{}", "=".repeat(80));
    println!(
        "Results: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );
    println!("{}\n", "=".repeat(80));

    assert_eq!(failed, 0, "Some SHACL tests failed");
}
