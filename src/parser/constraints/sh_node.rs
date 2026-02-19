use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::NodeConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    Constraint, ShaclError,
};

struct SHNodeConstraintParser;

impl ConstraintParserTrait for SHNodeConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        let mut constraints = Vec::new();

        for node_obj in graph.objects_for_subject_predicate(shape_node, sh::NODE) {
            let node_shape = match node_obj {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => continue,
            };

            if let Ok(shape) = super::super::parse_shape(graph, node_shape, Some(shape_node)) {
                constraints.push(Constraint::Node(NodeConstraint(Box::new(shape))));
            }
        }

        Ok(constraints)
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHNodeConstraintParser
}
