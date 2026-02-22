use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use oxigraph::model::{NamedNodeRef, NamedOrBlankNodeRef};

use super::{constraints::Constraint, path::Path, target::Target};

/// Reference to another shape, inline or by node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeReference<'a> {
    /// Inline shape definition.
    Inline(Box<Shape<'a>>),
    /// Shape node reference.
    Reference(NamedOrBlankNodeRef<'a>),
}

/// Configuration for `sh:closed` validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedConstraint<'a> {
    /// Extra allowed predicates.
    pub ignored_properties: Vec<NamedNodeRef<'a>>,
}

/// SHACL shape model used for both node and property shapes.
///
/// A shape is a node shape when `path` is `None`, and a property shape when
/// `path` is `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape<'a> {
    /// Shape node identifier.
    pub node: NamedOrBlankNodeRef<'a>,

    /// Optional `sh:name`.
    pub name: Option<String>,

    /// Optional `sh:description`.
    pub description: Option<String>,

    /// Property path (`None` for node shapes).
    pub path: Option<Path<'a>>,

    /// Shape targets.
    pub targets: HashSet<Target<'a>>,

    /// Whether validation is disabled for this shape.
    pub deactivated: bool,

    /// Optional messages.
    pub message: HashSet<String>,

    /// Result severity.
    pub severity: NamedNodeRef<'a>,

    /// Attached constraints.
    pub constraints: Vec<Constraint<'a>>,

    /// Optional `sh:closed` configuration.
    pub closed: Option<ClosedConstraint<'a>>,

    /// Nested property shapes.
    pub property_shapes: Vec<Shape<'a>>,

    pub parent: Option<NamedOrBlankNodeRef<'a>>,
}

pub struct ShapesInfo<'a> {
    shapes: &'a [Shape<'a>],
    graph_len: usize,
    detailed: bool,
}

impl<'a> ShapesInfo<'a> {
    pub fn new(shapes: &'a [Shape<'a>], graph_len: usize, detailed: bool) -> Self {
        ShapesInfo {
            shapes,
            graph_len,
            detailed,
        }
    }
}

impl<'a> Shape<'a> {
    pub fn node_shape(node: NamedOrBlankNodeRef<'a>, severity: NamedNodeRef<'a>) -> Self {
        Shape {
            node,
            name: None,
            description: None,
            path: None,
            targets: HashSet::new(),
            deactivated: false,
            message: HashSet::new(),
            severity,
            constraints: Vec::new(),
            closed: None,
            property_shapes: Vec::new(),
            parent: None,
        }
    }

    pub fn property_shape(
        node: NamedOrBlankNodeRef<'a>,
        path: Path<'a>,
        severity: NamedNodeRef<'a>,
    ) -> Self {
        Shape {
            node,
            name: None,
            description: None,
            path: Some(path),
            targets: HashSet::new(),
            deactivated: false,
            message: HashSet::new(),
            severity,
            constraints: Vec::new(),
            closed: None,
            property_shapes: Vec::new(),
            parent: None,
        }
    }

    pub fn is_node_shape(&self) -> bool {
        self.path.is_none()
    }

    pub fn is_property_shape(&self) -> bool {
        self.path.is_some()
    }

    pub fn with_node(mut self, node: NamedOrBlankNodeRef<'a>) -> Self {
        self.node = node;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_parent(mut self, parent: NamedOrBlankNodeRef<'a>) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn get_name(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else {
            format!("{}", self.node)
        }
    }

    pub fn add_target(mut self, target: Target<'a>) -> Self {
        self.targets.insert(target);
        self
    }

    pub fn with_deactivated(mut self, deactivated: bool) -> Self {
        self.deactivated = deactivated;
        self
    }

    pub fn add_message(mut self, message: String) -> Self {
        self.message.insert(message);
        self
    }

    pub fn with_severity(mut self, severity: NamedNodeRef<'a>) -> Self {
        self.severity = severity;
        self
    }

    pub fn add_constraint(mut self, constraint: Constraint<'a>) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn with_closed(mut self, closed: ClosedConstraint<'a>) -> Self {
        self.closed = Some(closed);
        self
    }

    pub fn add_property_shape(mut self, shape: Shape<'a>) -> Self {
        self.property_shapes.push(shape);
        self
    }

    pub fn has_constraints(&self) -> bool {
        !self.constraints.is_empty() || self.closed.is_some() || !self.property_shapes.is_empty()
    }

    pub fn all_nested_shapes(&self) -> Vec<&Shape<'a>> {
        let mut result = Vec::new();

        for prop_shape in &self.property_shapes {
            result.push(prop_shape);
            result.extend(prop_shape.all_nested_shapes());
        }

        result
    }
}

impl<'a> Display for Shape<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_property_shape() {
            write!(f, "PropertyShape")?;
        } else {
            write!(f, "NodeShape")?;
        }

        write!(f, " <{}>", self.node)?;

        if let Some(name) = &self.name {
            write!(f, " ({})", name)?;
        }

        if self.deactivated {
            write!(f, " [DEACTIVATED]")?;
        }

        writeln!(f)?;

        if let Some(parent) = &self.parent {
            writeln!(f, "  Parent Shape: {}", parent)?;
        }

        if let Some(path) = &self.path {
            writeln!(f, "  Path: {}", path)?;
        }

        if let Some(desc) = &self.description {
            writeln!(f, "  Description: {}", desc)?;
        }

        writeln!(f, "  Severity: {}", self.severity)?;

        if !self.targets.is_empty() {
            writeln!(f, "  Targets:")?;
            for target in &self.targets {
                writeln!(f, "    - {}", target)?;
            }
        }

        if !self.message.is_empty() {
            writeln!(f, "  Messages:")?;
            for msg in &self.message {
                writeln!(f, "    - {}", msg)?;
            }
        }

        if let Some(closed) = &self.closed {
            writeln!(f, "  {}", closed)?;
        }

        if !self.constraints.is_empty() {
            writeln!(f, "  Constraints:")?;
            for constraint in &self.constraints {
                for line in format!("{}", constraint).lines() {
                    writeln!(f, "    {}", line)?;
                }
            }
        }

        if !self.property_shapes.is_empty() {
            writeln!(f, "  Property Shapes:")?;
            for (i, prop_shape) in self.property_shapes.iter().enumerate() {
                writeln!(f, "    [{}]", i)?;
                for line in format!("{}", prop_shape).lines() {
                    writeln!(f, "      {}", line)?;
                }
            }
        }

        Ok(())
    }
}

impl<'a> Display for ShapeReference<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShapeReference::Inline(shape) => write!(f, "Inline({})", shape.get_name()),
            ShapeReference::Reference(node) => write!(f, "Ref({})", node),
        }
    }
}

impl<'a> Display for ClosedConstraint<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Closed Constraint")?;
        if !self.ignored_properties.is_empty() {
            write!(f, " (ignoring:")?;
            for (i, prop) in self.ignored_properties.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, " {}", prop)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl Display for ShapesInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n{}", "=".repeat(80))?;
        writeln!(f, "SHACL Shapes Information")?;
        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f, "Total shapes: {}", self.shapes.len())?;
        writeln!(f, "Total triples in shapes graph: {}", self.graph_len)?;

        let active_shapes = self.shapes.iter().filter(|s| !s.deactivated).count();
        let deactivated_shapes = self.shapes.len() - active_shapes;

        let total_targets: usize = self.shapes.iter().map(|s| s.targets.len()).sum();
        let total_constraints: usize = self.shapes.iter().map(|s| s.constraints.len()).sum();

        writeln!(f, "\nShape Status:")?;
        writeln!(f, "  Active: {}", active_shapes)?;
        writeln!(f, "  Deactivated: {}", deactivated_shapes)?;

        writeln!(f, "\nConstraints:")?;
        writeln!(f, "  Total targets: {}", total_targets)?;
        writeln!(f, "  Total constraints: {}", total_constraints)?;

        if self.detailed {
            writeln!(f, "\n{}", "-".repeat(80))?;
            writeln!(f, "Detailed Shape Information:")?;
            writeln!(f, "{}", "-".repeat(80))?;

            for (idx, shape) in self.shapes.iter().enumerate() {
                writeln!(f, "\nShape #{}: {}", idx + 1, shape.node)?;
                writeln!(
                    f,
                    "  Status: {}",
                    if shape.deactivated {
                        "DEACTIVATED"
                    } else {
                        "ACTIVE"
                    }
                )?;
                writeln!(f, "  Severity: {}", shape.severity)?;
                writeln!(f, "  Targets: {}", shape.targets.len())?;

                for target in &shape.targets {
                    writeln!(f, "    - {}", target)?;
                }

                writeln!(f, "  Constraints: {}", shape.constraints.len())?;

                for constraint in &shape.constraints {
                    writeln!(f, "    - {}", constraint)?;
                }

                if let Some(closed) = &shape.closed {
                    writeln!(f, "  Closed: {}", closed)?;
                }

                if !shape.message.is_empty() {
                    writeln!(f, "  Messages: {}", shape.message.len())?;
                    for msg in &shape.message {
                        writeln!(f, "    - {}", msg)?;
                    }
                }
            }
        }

        writeln!(f, "\n{}", "=".repeat(80))
    }
}
