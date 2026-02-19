use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use std::fmt::Display;

use crate::Path;

use super::shape::Shape;

/// Node kind constraint values as defined in SHACL spec
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    BlankNode,
    IRI,
    Literal,
    BlankNodeOrIRI,
    BlankNodeOrLiteral,
    IRIOrLiteral,
}

// ============ Constraint Newtype Wrappers ============
// Each constraint type gets its own wrapper that implements Validate

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassConstraint<'a>(pub NamedNodeRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatatypeConstraint<'a>(pub NamedNodeRef<'a>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeKindConstraint(pub NodeKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinCountConstraint(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxCountConstraint(pub i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinExclusiveConstraint<'a>(pub TermRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinInclusiveConstraint<'a>(pub TermRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxExclusiveConstraint<'a>(pub TermRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxInclusiveConstraint<'a>(pub TermRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinLengthConstraint(pub i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxLengthConstraint(pub i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternConstraint {
    pub pattern: String,
    pub flags: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInConstraint(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueLangConstraint(pub bool);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqualsConstraint<'a>(pub Path<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisjointConstraint<'a>(pub Path<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LessThanConstraint<'a>(pub Path<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LessThanOrEqualsConstraint<'a>(pub Path<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HasValueConstraint<'a>(pub TermRef<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InConstraint<'a>(pub Vec<TermRef<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConstraint<'a>(pub Box<Shape<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedValueShapeConstraint<'a> {
    pub shape: Box<Shape<'a>>,
    pub qualified_min_count: Option<i32>,
    pub qualified_max_count: Option<i32>,
    pub qualified_value_shapes_disjoint: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndConstraint<'a>(pub Vec<Shape<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrConstraint<'a>(pub Vec<Shape<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XoneConstraint<'a>(pub Vec<Shape<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotConstraint<'a>(pub Box<Shape<'a>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SparqlExecutable {
    Select(String),
    Ask(String),
}

impl SparqlExecutable {
    pub fn query(&self) -> &str {
        match self {
            SparqlExecutable::Select(query) => query,
            SparqlExecutable::Ask(query) => query,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlConstraint<'a> {
    pub source_constraint: Option<NamedOrBlankNodeRef<'a>>,
    pub source_constraint_component: Option<NamedOrBlankNodeRef<'a>>,
    pub executable: SparqlExecutable,
    pub messages: Vec<String>,
    pub prefixes: Vec<(String, String)>,
    pub parameter_bindings: Vec<(String, TermRef<'a>)>,
}

/// SHACL Constraint that can be applied to focus nodes or property values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint<'a> {
    // ============ Value Type Constraints ============
    /// Specifies that each value node is a SHACL instance of a given type
    Class(ClassConstraint<'a>),

    /// Specifies that each value node must have the given datatype
    Datatype(DatatypeConstraint<'a>),

    /// Specifies the node kind (IRI, BlankNode, Literal, etc.)
    NodeKind(NodeKindConstraint),

    // ============ Cardinality Constraints (require path) ============
    /// Minimum number of value nodes
    MinCount(MinCountConstraint),

    /// Maximum number of value nodes
    MaxCount(MaxCountConstraint),

    // ============ Value Range Constraints ============
    /// Specifies the minimum exclusive value
    MinExclusive(MinExclusiveConstraint<'a>),

    /// Specifies the minimum inclusive value
    MinInclusive(MinInclusiveConstraint<'a>),

    /// Specifies the maximum exclusive value
    MaxExclusive(MaxExclusiveConstraint<'a>),

    /// Specifies the maximum inclusive value
    MaxInclusive(MaxInclusiveConstraint<'a>),

    // ============ String-based Constraints ============
    /// Minimum string length
    MinLength(MinLengthConstraint),

    /// Maximum string length
    MaxLength(MaxLengthConstraint),

    /// Regular expression pattern
    Pattern(PatternConstraint),

    /// List of allowed language tags
    LanguageIn(LanguageInConstraint),

    /// Each value must have a unique language tag (requires path)
    UniqueLang(UniqueLangConstraint),

    // ============ Property Pair Constraints (require path) ============
    /// Property values must equal values of another property
    Equals(EqualsConstraint<'a>),

    /// Property values must be disjoint from values of another property
    Disjoint(DisjointConstraint<'a>),

    /// Property values must be less than values of another property
    LessThan(LessThanConstraint<'a>),

    /// Property values must be less than or equal to values of another property
    LessThanOrEquals(LessThanOrEqualsConstraint<'a>),

    // ============ Other Value Constraints ============
    /// Specifies a specific required value
    HasValue(HasValueConstraint<'a>),

    /// List of allowed values
    In(InConstraint<'a>),

    // ============ Shape-based Constraints ============
    /// All value nodes must conform to the given shape
    Node(NodeConstraint<'a>),

    // ============ Qualified Value Shapes (require path) ============
    /// Qualified value shape constraint
    QualifiedValueShape(QualifiedValueShapeConstraint<'a>),

    // ============ Logical Constraints (recursive) ============
    /// All of the given shapes must be satisfied
    And(AndConstraint<'a>),

    /// At least one of the given shapes must be satisfied
    Or(OrConstraint<'a>),

    /// Exactly one of the given shapes must be satisfied
    Xone(XoneConstraint<'a>),

    /// The given shape must not be satisfied
    Not(NotConstraint<'a>),

    /// Constraint backed by a SPARQL executable.
    Sparql(SparqlConstraint<'a>),
}

impl<'a> Constraint<'a> {
    /// Returns true if this constraint requires a path to be meaningful
    pub fn requires_path(&self) -> bool {
        matches!(
            self,
            Constraint::MinCount(_)
                | Constraint::MaxCount(_)
                | Constraint::UniqueLang(_)
                | Constraint::Equals(_)
                | Constraint::Disjoint(_)
                | Constraint::LessThan(_)
                | Constraint::LessThanOrEquals(_)
                | Constraint::QualifiedValueShape(_)
        )
    }
}

impl Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeKind::BlankNode => write!(f, "BlankNode"),
            NodeKind::IRI => write!(f, "IRI"),
            NodeKind::Literal => write!(f, "Literal"),
            NodeKind::BlankNodeOrIRI => write!(f, "BlankNodeOrIRI"),
            NodeKind::BlankNodeOrLiteral => write!(f, "BlankNodeOrLiteral"),
            NodeKind::IRIOrLiteral => write!(f, "IRIOrLiteral"),
        }
    }
}

impl Display for SparqlExecutable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SparqlExecutable::Select(_) => write!(f, "SELECT"),
            SparqlExecutable::Ask(_) => write!(f, "ASK"),
        }
    }
}

impl<'a> Display for SparqlConstraint<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = self.source_constraint {
            write!(f, "{}", source)?;
        }

        if let Some(component) = self.source_constraint_component {
            write!(f, " component: {}", component)?;
        }

        write!(f, " [{}]", self.executable)?;

        if !self.messages.is_empty() {
            write!(f, " messages: {}", self.messages.len())?;
        }

        if !self.prefixes.is_empty() {
            write!(f, " prefixes: {}", self.prefixes.len())?;
        }

        if !self.parameter_bindings.is_empty() {
            write!(f, " params:")?;
            for (name, value) in &self.parameter_bindings {
                write!(f, " ${}={}", name, value)?;
            }
        }

        write!(f, " query: \"{}\"", self.executable.query().replace('\n', " "))?;

        Ok(())
    }
}

impl<'a> Display for Constraint<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Value Type Constraints
            Constraint::Class(c) => write!(f, "sh:class {}", c.0),
            Constraint::Datatype(d) => write!(f, "sh:datatype {}", d.0),
            Constraint::NodeKind(nk) => write!(f, "sh:nodeKind {}", nk.0),

            // Cardinality Constraints
            Constraint::MinCount(c) => write!(f, "sh:minCount {}", c.0),
            Constraint::MaxCount(c) => write!(f, "sh:maxCount {}", c.0),

            // Value Range Constraints
            Constraint::MinExclusive(c) => write!(f, "sh:minExclusive {}", c.0),
            Constraint::MinInclusive(c) => write!(f, "sh:minInclusive {}", c.0),
            Constraint::MaxExclusive(c) => write!(f, "sh:maxExclusive {}", c.0),
            Constraint::MaxInclusive(c) => write!(f, "sh:maxInclusive {}", c.0),

            // String-based Constraints
            Constraint::MinLength(c) => write!(f, "sh:minLength {}", c.0),
            Constraint::MaxLength(c) => write!(f, "sh:maxLength {}", c.0),
            Constraint::Pattern(c) => {
                write!(f, "sh:pattern \"{}\"", c.pattern)?;
                if let Some(flags_str) = &c.flags {
                    write!(f, " flags: {}", flags_str)?;
                }
                Ok(())
            }
            Constraint::LanguageIn(c) => {
                write!(f, "sh:languageIn (")?;
                for (i, lang) in c.0.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", lang)?;
                }
                write!(f, ")")
            }
            Constraint::UniqueLang(c) => write!(f, "sh:uniqueLang {}", c.0),

            // Property Pair Constraints
            Constraint::Equals(c) => write!(f, "sh:equals {}", c.0),
            Constraint::Disjoint(c) => write!(f, "sh:disjoint {}", c.0),
            Constraint::LessThan(c) => write!(f, "sh:lessThan {}", c.0),
            Constraint::LessThanOrEquals(c) => write!(f, "sh:lessThanOrEquals {}", c.0),

            // Other Value Constraints
            Constraint::HasValue(c) => write!(f, "sh:hasValue {}", c.0),
            Constraint::In(c) => {
                write!(f, "sh:in (")?;
                for (i, val) in c.0.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }

            // Shape-based Constraints
            Constraint::Node(c) => {
                writeln!(f, "sh:node {{")?;
                for line in format!("{}", c.0).lines() {
                    writeln!(f, "  {}", line)?;
                }
                write!(f, "}}")
            }

            // Qualified Value Shapes
            Constraint::QualifiedValueShape(c) => {
                writeln!(f, "sh:qualifiedValueShape {{")?;
                for line in format!("{}", c.shape).lines() {
                    writeln!(f, "  {}", line)?;
                }
                write!(f, "}}")?;
                if let Some(min) = c.qualified_min_count {
                    write!(f, " min: {}", min)?;
                }
                if let Some(max) = c.qualified_max_count {
                    write!(f, " max: {}", max)?;
                }
                if c.qualified_value_shapes_disjoint {
                    write!(f, " disjoint: true")?;
                }
                Ok(())
            }

            // Logical Constraints
            Constraint::And(c) => {
                writeln!(f, "sh:and [")?;
                for constraint in &c.0 {
                    for line in format!("{}", constraint).lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "]")
            }
            Constraint::Or(c) => {
                writeln!(f, "sh:or [")?;
                for constraint in &c.0 {
                    for line in format!("{}", constraint).lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "]")
            }
            Constraint::Xone(c) => {
                writeln!(f, "sh:xone [")?;
                for constraint in &c.0 {
                    for line in format!("{}", constraint).lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }
                write!(f, "]")
            }
            Constraint::Not(c) => {
                writeln!(f, "sh:not {{")?;
                for line in format!("{}", c.0).lines() {
                    writeln!(f, "  {}", line)?;
                }
                write!(f, "}}")
            }
            Constraint::Sparql(c) => {
                write!(f, "sh:sparql {}", c)
            }
        }
    }
}
