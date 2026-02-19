use oxigraph::model::{Graph, NamedOrBlankNodeRef};

use crate::{Constraint, ShaclError};

pub trait ConstraintParserTrait {
    fn parse_constraint<'a>(
        &self,
        shape_node: NamedOrBlankNodeRef<'a>,
        graph: &'a Graph,
    ) -> Result<Vec<Constraint<'a>>, ShaclError>;
}
