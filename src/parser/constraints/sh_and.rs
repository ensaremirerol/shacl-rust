use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::AndConstraint, parser::constraint_parser_trait::ConstraintParserTrait, sh,
    utils::parse_rdf_list, Constraint, ShaclError,
};

struct SHAndConstraintParser;

impl ConstraintParserTrait for SHAndConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        let mut constraints = Vec::new();

        for and_obj in graph.objects_for_subject_predicate(shape_node, sh::AND) {
            let and_node = match and_obj {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => continue,
            };

            let shape_refs = parse_rdf_list(graph, and_node);
            let mut and_shapes = Vec::new();
            for shape_ref in shape_refs {
                let sn = match shape_ref {
                    TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                    TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                    _ => continue,
                };
                if let Ok(sub_shape) = super::super::parse_shape(graph, sn, Some(shape_node)) {
                    and_shapes.push(sub_shape);
                }
            }

            if !and_shapes.is_empty() {
                constraints.push(Constraint::And(AndConstraint(and_shapes)));
            }
        }

        Ok(constraints)
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHAndConstraintParser
}
