use oxigraph::model::{Graph, NamedOrBlankNodeRef, TermRef};

use crate::{
    core::constraints::QualifiedValueShapeConstraint,
    parser::constraint_parser_trait::ConstraintParserTrait,
    sh,
    utils::{get_boolean_value, get_integer_value},
    Constraint, ShaclError,
};

struct SHQualifiedValueShapeConstraintParser;

impl ConstraintParserTrait for SHQualifiedValueShapeConstraintParser {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError> {
        if let Some(qvs_obj) =
            graph.object_for_subject_predicate(shape_node, sh::QUALIFIED_VALUE_SHAPE)
        {
            let qvs_node = match qvs_obj {
                TermRef::NamedNode(nn) => NamedOrBlankNodeRef::NamedNode(nn),
                TermRef::BlankNode(bn) => NamedOrBlankNodeRef::BlankNode(bn),
                _ => return Ok(vec![]),
            };

            if let Ok(shape) = super::super::parse_shape(graph, qvs_node, Some(shape_node)) {
                let qualified_min_count =
                    get_integer_value(graph, shape_node, sh::QUALIFIED_MIN_COUNT);
                let qualified_max_count =
                    get_integer_value(graph, shape_node, sh::QUALIFIED_MAX_COUNT);
                let qualified_value_shapes_disjoint =
                    get_boolean_value(graph, shape_node, sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
                        .unwrap_or(false);

                return Ok(vec![Constraint::QualifiedValueShape(
                    QualifiedValueShapeConstraint {
                        shape: Box::new(shape),
                        qualified_min_count,
                        qualified_max_count,
                        qualified_value_shapes_disjoint,
                    },
                )]);
            }
        }
        Ok(vec![])
    }
}

pub fn parser() -> &'static dyn ConstraintParserTrait {
    &SHQualifiedValueShapeConstraintParser
}
