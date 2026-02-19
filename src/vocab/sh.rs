//! SHACL vocabulary constants
//!
//! This module provides RDF node references for the W3C Shapes Constraint Language (SHACL) vocabulary.
//! Based on the SHACL 1.0 specification: https://www.w3.org/TR/shacl/

use oxigraph::model::NamedNodeRef;

// Shapes vocabulary -------------------------------------------------------

/// A shape is a collection of constraints that may be targeted for certain nodes.
pub const SHAPE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Shape");

/// A node shape is a shape that specifies constraints that need to be met with respect to focus nodes.
pub const NODE_SHAPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeShape");

/// A property shape is a shape that specifies constraints on the values of a focus node for a given property or path.
pub const PROPERTY_SHAPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PropertyShape");

/// If set to true then all nodes conform to this.
pub const DEACTIVATED: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#deactivated");

/// Links a shape to a class, indicating that all instances of the class must conform to the shape.
pub const TARGET_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#targetClass");

/// Links a shape to individual nodes, indicating that these nodes must conform to the shape.
pub const TARGET_NODE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#targetNode");

/// Links a shape to a property, indicating that all objects of triples that have the given property as their predicate must conform to the shape.
pub const TARGET_OBJECTS_OF: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#targetObjectsOf");

/// Links a shape to a property, indicating that all subjects of triples that have the given property as their predicate must conform to the shape.
pub const TARGET_SUBJECTS_OF: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#targetSubjectsOf");

/// A human-readable message explaining the cause of the result.
pub const MESSAGE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#message");

/// Defines the severity that validation results produced by a shape must have. Defaults to sh:Violation.
pub const SEVERITY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#severity");

// Node kind vocabulary -------------------------------------------------------

/// The class of all node kinds.
pub const NODE_KIND: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeKind");

/// The node kind of all blank nodes.
pub const BLANK_NODE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#BlankNode");

/// The node kind of all blank nodes or IRIs.
pub const BLANK_NODE_OR_IRI: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#BlankNodeOrIRI");

/// The node kind of all blank nodes or literals.
pub const BLANK_NODE_OR_LITERAL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#BlankNodeOrLiteral");

/// The node kind of all IRIs.
pub const IRI: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#IRI");

/// The node kind of all IRIs or literals.
pub const IRI_OR_LITERAL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#IRIOrLiteral");

/// The node kind of all literals.
pub const LITERAL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Literal");

// Results vocabulary ----------------------------------------------------------

/// The class of SHACL validation reports.
pub const VALIDATION_REPORT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ValidationReport");

/// True if the validation did not produce any validation results, and false otherwise.
pub const CONFORMS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#conforms");

/// The validation results contained in a validation report.
pub const RESULT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#result");

/// If true then the validation engine was certain that the shapes graph has passed all SHACL syntax requirements during the validation process.
pub const SHAPES_GRAPH_WELL_FORMED: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#shapesGraphWellFormed");

/// The base class of validation results, typically not instantiated directly.
pub const ABSTRACT_RESULT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#AbstractResult");

/// The class of validation results.
pub const VALIDATION_RESULT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ValidationResult");

/// The class of validation result severity levels.
pub const SEVERITY_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Severity");

/// The severity for an informational validation result.
pub const INFO: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Info");

/// The severity for a violation validation result.
pub const VIOLATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Violation");

/// The severity for a warning validation result.
pub const WARNING: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Warning");

/// Links a result with other results that provide more details.
pub const DETAIL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#detail");

/// The focus node that was validated when the result was produced.
pub const FOCUS_NODE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#focusNode");

/// Human-readable messages explaining the cause of the result.
pub const RESULT_MESSAGE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultMessage");

/// The path of a validation result, based on the path of the validated property shape.
pub const RESULT_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultPath");

/// The severity of the result.
pub const RESULT_SEVERITY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultSeverity");

/// The constraint that was validated when the result was produced.
pub const SOURCE_CONSTRAINT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#sourceConstraint");

/// The shape that was validated when the result was produced.
pub const SOURCE_SHAPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#sourceShape");

/// The constraint component that is the source of the result.
pub const SOURCE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#sourceConstraintComponent");

/// An RDF node that has caused the result.
pub const VALUE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#value");

// Graph properties ---------------------------------------------------------

/// Shapes graphs that should be used when validating this data graph.
pub const SHAPES_GRAPH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#shapesGraph");

/// Suggested shapes graphs for this ontology.
pub const SUGGESTED_SHAPES_GRAPH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#suggestedShapesGraph");

/// An entailment regime that indicates what kind of inferencing is required by a shapes graph.
pub const ENTAILMENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#entailment");

// Path vocabulary -----------------------------------------------------------

/// Specifies the property path of a property shape.
pub const PATH: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#path");

/// The (single) value of this property represents an inverse path (object to subject).
pub const INVERSE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#inversePath");

/// The (single) value of this property must be a list of path elements, representing the elements of alternative paths.
pub const ALTERNATIVE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#alternativePath");

/// The (single) value of this property represents a path that is matched zero or more times.
pub const ZERO_OR_MORE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#zeroOrMorePath");

/// The (single) value of this property represents a path that is matched one or more times.
pub const ONE_OR_MORE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#oneOrMorePath");

/// The (single) value of this property represents a path that is matched zero or one times.
pub const ZERO_OR_ONE_PATH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#zeroOrOnePath");

// Parameters metamodel -------------------------------------------------------

/// Superclass of components that can take parameters.
pub const PARAMETERIZABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Parameterizable");

/// The parameters of a function or constraint component.
pub const PARAMETER: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#parameter");

/// Outlines how human-readable labels of instances of the associated Parameterizable shall be produced.
pub const LABEL_TEMPLATE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#labelTemplate");

/// The class of parameter declarations.
pub const PARAMETER_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Parameter");

/// Indicates whether a parameter is optional.
pub const OPTIONAL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#optional");

// Constraint components metamodel -----------------------------------------------

/// The class of constraint components.
pub const CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ConstraintComponent");

/// The validator(s) used to evaluate constraints of either node or property shapes.
pub const VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#validator");

/// The validator(s) used to evaluate a constraint in the context of a node shape.
pub const NODE_VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#nodeValidator");

/// The validator(s) used to evaluate a constraint in the context of a property shape.
pub const PROPERTY_VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#propertyValidator");

/// The class of validators.
pub const VALIDATOR_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Validator");

/// The class of validators based on SPARQL ASK queries.
pub const SPARQL_ASK_VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLAskValidator");

/// The class of validators based on SPARQL SELECT queries.
pub const SPARQL_SELECT_VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLSelectValidator");

// Core Constraint Components -----------------------------------------------

// And constraint component
pub const AND_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#AndConstraintComponent");

pub const AND: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#and");

// Class constraint component
pub const CLASS_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ClassConstraintComponent");

pub const CLASS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#class");

// Closed constraint component
pub const CLOSED_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ClosedConstraintComponent");

pub const CLOSED: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#closed");

pub const IGNORED_PROPERTIES: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ignoredProperties");

// Datatype constraint component
pub const DATATYPE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DatatypeConstraintComponent");

pub const DATATYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#datatype");

// Disjoint constraint component
pub const DISJOINT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DisjointConstraintComponent");

pub const DISJOINT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#disjoint");

// Equals constraint component
pub const EQUALS_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#EqualsConstraintComponent");

pub const EQUALS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#equals");

// HasValue constraint component
pub const HAS_VALUE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#HasValueConstraintComponent");

pub const HAS_VALUE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#hasValue");

// In constraint component
pub const IN_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#InConstraintComponent");

pub const IN: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#in");

// LanguageIn constraint component
pub const LANGUAGE_IN_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#LanguageInConstraintComponent");

pub const LANGUAGE_IN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#languageIn");

// LessThan constraint component
pub const LESS_THAN_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#LessThanConstraintComponent");

pub const LESS_THAN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#lessThan");

// LessThanOrEquals constraint component
pub const LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent");

pub const LESS_THAN_OR_EQUALS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#lessThanOrEquals");

// MaxCount constraint component
pub const MAX_COUNT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxCountConstraintComponent");

pub const MAX_COUNT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#maxCount");

// MaxExclusive constraint component
pub const MAX_EXCLUSIVE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent");

pub const MAX_EXCLUSIVE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#maxExclusive");

// MaxInclusive constraint component
pub const MAX_INCLUSIVE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent");

pub const MAX_INCLUSIVE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#maxInclusive");

// MaxLength constraint component
pub const MAX_LENGTH_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxLengthConstraintComponent");

pub const MAX_LENGTH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#maxLength");

// MinCount constraint component
pub const MIN_COUNT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinCountConstraintComponent");

pub const MIN_COUNT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#minCount");

// MinExclusive constraint component
pub const MIN_EXCLUSIVE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent");

pub const MIN_EXCLUSIVE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#minExclusive");

// MinInclusive constraint component
pub const MIN_INCLUSIVE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent");

pub const MIN_INCLUSIVE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#minInclusive");

// MinLength constraint component
pub const MIN_LENGTH_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinLengthConstraintComponent");

pub const MIN_LENGTH: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#minLength");

// Node constraint component
pub const NODE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeConstraintComponent");

pub const NODE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#node");

// NodeKind constraint component
pub const NODE_KIND_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeKindConstraintComponent");

pub const NODE_KIND_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#nodeKind");

// Not constraint component
pub const NOT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NotConstraintComponent");

pub const NOT: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#not");

// Or constraint component
pub const OR_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#OrConstraintComponent");

pub const OR: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#or");

// Pattern constraint component
pub const PATTERN_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PatternConstraintComponent");

pub const PATTERN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#pattern");

pub const FLAGS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#flags");

// Property constraint component
pub const PROPERTY_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PropertyConstraintComponent");

pub const PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#property");

// QualifiedMaxCount constraint component
pub const QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent");

pub const QUALIFIED_MAX_COUNT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#qualifiedMaxCount");

pub const QUALIFIED_VALUE_SHAPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#qualifiedValueShape");

pub const QUALIFIED_VALUE_SHAPES_DISJOINT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint");

// QualifiedMinCount constraint component
pub const QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent");

pub const QUALIFIED_MIN_COUNT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#qualifiedMinCount");

// UniqueLang constraint component
pub const UNIQUE_LANG_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#UniqueLangConstraintComponent");

pub const UNIQUE_LANG: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#uniqueLang");

// Xone constraint component
pub const XONE_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#XoneConstraintComponent");

pub const XONE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#xone");

// SPARQL execution support -----------------------------------------------

/// The class of resources that encapsulate a SPARQL query.
pub const SPARQL_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLExecutable");

/// The class of SPARQL executables that are based on an ASK query.
pub const SPARQL_ASK_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLAskExecutable");

/// The SPARQL ASK query to execute.
pub const ASK: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ask");

/// The class of SPARQL executables that are based on a CONSTRUCT query.
pub const SPARQL_CONSTRUCT_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLConstructExecutable");

/// The SPARQL CONSTRUCT query to execute.
pub const CONSTRUCT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#construct");

/// The class of SPARQL executables based on a SELECT query.
pub const SPARQL_SELECT_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLSelectExecutable");

/// The SPARQL SELECT query to execute.
pub const SELECT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#select");

/// The class of SPARQL executables based on a SPARQL UPDATE.
pub const SPARQL_UPDATE_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLUpdateExecutable");

/// The SPARQL UPDATE to execute.
pub const UPDATE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#update");

/// The prefixes that shall be applied before parsing the associated SPARQL query.
pub const PREFIXES: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#prefixes");

/// The class of prefix declarations.
pub const PREFIX_DECLARATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PrefixDeclaration");

/// Links a resource with its namespace prefix declarations.
pub const DECLARE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#declare");

/// The prefix of a prefix declaration.
pub const PREFIX: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#prefix");

/// The namespace associated with a prefix in a prefix declaration.
pub const NAMESPACE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#namespace");

// SPARQL-based Constraints support -----------------------------------------------

/// A constraint component that can be used to define constraints based on SPARQL queries.
pub const SPARQL_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLConstraintComponent");

/// Links a shape with SPARQL constraints.
pub const SPARQL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#sparql");

/// The class of constraints based on SPARQL SELECT queries.
pub const SPARQL_CONSTRAINT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLConstraint");

// Non-validating constraint properties ----------------------------------------

/// A default value for a property.
pub const DEFAULT_VALUE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#defaultValue");

/// Human-readable descriptions for the property in the context of the surrounding shape.
pub const DESCRIPTION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#description");

/// Can be used to link to a property group.
pub const GROUP: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#group");

/// Human-readable labels for the property in the context of the surrounding shape.
pub const NAME: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#name");

/// Specifies the relative order.
pub const ORDER: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#order");

/// Instances of this class represent groups of property shapes that belong together.
pub const PROPERTY_GROUP: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PropertyGroup");

// Advanced Target vocabulary -----------------------------------------------

/// Links a shape to a target specified by an extension language.
pub const TARGET: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#target");

/// The base class of targets such as those based on SPARQL queries.
pub const TARGET_CLASS_DEF: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Target");

/// The (meta) class for parameterizable targets.
pub const TARGET_TYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#TargetType");

/// The class of targets that are based on SPARQL queries.
pub const SPARQL_TARGET: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLTarget");

/// The (meta) class for parameterizable targets that are based on SPARQL queries.
pub const SPARQL_TARGET_TYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLTargetType");

// Functions Vocabulary -----------------------------------------------

/// The class of SHACL functions.
pub const FUNCTION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Function");

/// The expected type of values returned by the associated function.
pub const RETURN_TYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#returnType");

/// A function backed by a SPARQL query.
pub const SPARQL_FUNCTION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLFunction");

// Result Annotations -----------------------------------------------

/// Links a SPARQL validator with zero or more sh:ResultAnnotation instances.
pub const RESULT_ANNOTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#resultAnnotation");

/// A class of result annotations.
pub const RESULT_ANNOTATION_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ResultAnnotation");

/// The annotation property that shall be set.
pub const ANNOTATION_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#annotationProperty");

/// The (default) values of the annotation property.
pub const ANNOTATION_VALUE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#annotationValue");

/// The name of the SPARQL variable from the SELECT clause.
pub const ANNOTATION_VAR_NAME: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#annotationVarName");

// Node Expressions -----------------------------------------------

/// A node expression that represents the current focus node.
pub const THIS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#this");

/// The shape that all input nodes of the expression need to conform to.
pub const FILTER_SHAPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#filterShape");

/// The node expression producing the input nodes of a filter shape expression.
pub const NODES: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#nodes");

/// A list of node expressions that shall be intersected.
pub const INTERSECTION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#intersection");

/// A list of node expressions that shall be used together.
pub const UNION: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#union");

// Expression Constraints -----------------------------------------------

/// A constraint component that can be used to verify that a given node expression produces true for all value nodes.
pub const EXPRESSION_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ExpressionConstraintComponent");

/// The node expression that must return true for the value nodes.
pub const EXPRESSION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#expression");

// Rules -----------------------------------------------

/// The class of SHACL rules. Never instantiated directly.
pub const RULE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#Rule");

/// The rules linked to a shape.
pub const RULE_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#rule");

/// The shapes that the focus nodes need to conform to before a rule is executed on them.
pub const CONDITION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#condition");

/// A rule based on triple (subject, predicate, object) pattern.
pub const TRIPLE_RULE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#TripleRule");

/// An expression producing the resources that shall be inferred as subjects.
pub const SUBJECT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#subject");

/// An expression producing the properties that shall be inferred as predicates.
pub const PREDICATE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#predicate");

/// An expression producing the nodes that shall be inferred as objects.
pub const OBJECT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#object");

/// The class of SHACL rules based on SPARQL CONSTRUCT queries.
pub const SPARQL_RULE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SPARQLRule");

// SHACL-JS -----------------------------------------------

/// Abstract base class of resources that declare an executable JavaScript.
pub const JS_EXECUTABLE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSExecutable");

/// The class of targets that are based on JavaScript functions.
pub const JS_TARGET: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSTarget");

/// The (meta) class for parameterizable targets that are based on JavaScript functions.
pub const JS_TARGET_TYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSTargetType");

/// The class of constraints backed by a JavaScript function.
pub const JS_CONSTRAINT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSConstraint");

/// A constraint component with the parameter sh:js linking to a sh:JSConstraint containing a sh:script.
pub const JS_CONSTRAINT_COMPONENT: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSConstraintComponent");

/// Constraints expressed in JavaScript.
pub const JS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#js");

/// The name of the JavaScript function to execute.
pub const JS_FUNCTION_NAME: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#jsFunctionName");

/// Declares which JavaScript libraries are needed to execute this.
pub const JS_LIBRARY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#jsLibrary");

/// Declares the URLs of a JavaScript library.
pub const JS_LIBRARY_URL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#jsLibraryURL");

/// The class of SHACL functions that execute a JavaScript function when called.
pub const JS_FUNCTION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSFunction");

/// Represents a JavaScript library.
pub const JS_LIBRARY_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSLibrary");

/// The class of SHACL rules expressed using JavaScript.
pub const JS_RULE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSRule");

/// A SHACL validator based on JavaScript.
pub const JS_VALIDATOR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#JSValidator");
