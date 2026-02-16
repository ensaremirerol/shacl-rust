use oxigraph::model::NamedNodeRef;

pub enum Constraint<'a> {
    Datatype(NamedNodeRef<'a>),
}
