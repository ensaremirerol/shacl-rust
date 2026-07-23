//! The diagnostic code registry: one `RegistryEntry` per code (V0000-V0029,
//! L0001-L0012) giving a stable title, a link into the SHACL spec, a plain-
//! language explanation, and a minimal failing/fixed Turtle pair. This is
//! the data the `explain <CODE>` subcommand renders; it is also consulted by
//! `derive` (via `code_for_component`) to map a violation's
//! `sh:sourceConstraintComponent` onto a code.

pub struct RegistryEntry {
    pub code: &'static str,
    pub title: &'static str,
    pub component: Option<&'static str>,
    pub spec_ref: &'static str,
    pub explanation: &'static str,
    pub failing_example: &'static str,
    pub fixed_example: &'static str,
}

static ENTRIES: &[RegistryEntry] = &[
    RegistryEntry {
        code: "V0000",
        title: "custom constraint component violated",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#constraint-components",
        explanation: "This is the fallback code for a violation whose \
sh:sourceConstraintComponent is not one of the built-in SHACL-core \
components this registry knows about, typically a user-defined constraint \
component declared with sh:ConstraintComponent and its own validators. The \
diagnostic still carries whatever message, path, and value the component's \
own validator produced; consult that component's own documentation for the \
precise rule being enforced.",
        failing_example: "ex:alice ex:status \"bogus\" .\n# violates a custom sh:ConstraintComponent on ex:StatusShape",
        fixed_example: "ex:alice ex:status \"active\" .",
    },
    RegistryEntry {
        code: "V0001",
        title: "value is not an instance of the required class",
        component: Some("http://www.w3.org/ns/shacl#ClassConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#ClassConstraintComponent",
        explanation: "sh:class requires every value node to be an instance of the given \
class, following rdf:type together with rdfs:subClassOf reasoning when the \
shapes graph provides class hierarchy triples. A value node that is a \
literal, or an IRI/blank node with no rdf:type path to the class, violates \
the constraint. Multiple sh:class values on the same shape are all \
required (conjunction), not alternatives.",
        failing_example: "ex:alice a ex:Company .\n# with: sh:path ex:knows ; sh:class ex:Person",
        fixed_example: "ex:alice a ex:Person .",
    },
    RegistryEntry {
        code: "V0002",
        title: "value does not have the required datatype",
        component: Some("http://www.w3.org/ns/shacl#DatatypeConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#DatatypeConstraintComponent",
        explanation: "sh:datatype requires every value node to be a literal whose datatype \
IRI exactly matches the given one; IRIs, blank nodes, and literals with a \
different (even compatible) datatype all violate. For xsd:string the \
constraint additionally requires the literal to have no language tag, \
since a language-tagged literal has datatype rdf:langString.",
        failing_example: "ex:alice ex:age \"30\" .\n# with: sh:path ex:age ; sh:datatype xsd:integer",
        fixed_example: "ex:alice ex:age \"30\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0003",
        title: "value has the wrong node kind",
        component: Some("http://www.w3.org/ns/shacl#NodeKindConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#NodeKindConstraintComponent",
        explanation: "sh:nodeKind restricts each value node to one of sh:IRI, sh:BlankNode, \
sh:Literal, or one of the three compound kinds (sh:BlankNodeOrIRI, \
sh:BlankNodeOrLiteral, sh:IRIOrLiteral). A value node whose kind is not \
among those permitted by the declared sh:nodeKind violates the constraint.",
        failing_example: "ex:alice ex:homepage \"http://example.org/a\" .\n# with: sh:path ex:homepage ; sh:nodeKind sh:IRI",
        fixed_example: "ex:alice ex:homepage <http://example.org/a> .",
    },
    RegistryEntry {
        code: "V0004",
        title: "too few values for property",
        component: Some("http://www.w3.org/ns/shacl#MinCountConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MinCountConstraintComponent",
        explanation: "sh:minCount requires at least that many value nodes for the shape's \
path at the focus node; it applies only to property shapes, never to node \
shapes. A focus node with fewer matching values than the bound - including \
zero - violates the constraint, producing exactly one validation result \
per focus node.",
        failing_example: "ex:alice a ex:Person .\n# with: sh:path ex:name ; sh:minCount 1 (no ex:name triple)",
        fixed_example: "ex:alice ex:name \"Alice\" .",
    },
    RegistryEntry {
        code: "V0005",
        title: "too many values for property",
        component: Some("http://www.w3.org/ns/shacl#MaxCountConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MaxCountConstraintComponent",
        explanation: "sh:maxCount requires at most that many value nodes for the shape's \
path at the focus node; like sh:minCount it applies only to property \
shapes. A focus node whose value-node count for the path exceeds the bound \
violates the constraint, producing one validation result per focus node.",
        failing_example: "ex:alice ex:ssn \"111\" , \"222\" .\n# with: sh:path ex:ssn ; sh:maxCount 1",
        fixed_example: "ex:alice ex:ssn \"111\" .",
    },
    RegistryEntry {
        code: "V0006",
        title: "value is not greater than the exclusive minimum",
        component: Some("http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MinExclusiveConstraintComponent",
        explanation: "sh:minExclusive requires every value node to be a literal that \
compares strictly greater than the given bound. Comparison follows the \
same type-aware ordering as the other range constraints (numeric, \
xsd:dateTime, or lexical for other literal types); a value equal to the \
bound, or one that cannot be compared to it at all, violates.",
        failing_example: "ex:alice ex:age \"0\"^^xsd:integer .\n# with: sh:path ex:age ; sh:minExclusive 0",
        fixed_example: "ex:alice ex:age \"1\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0007",
        title: "value violates sh:minInclusive",
        component: Some("http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MinInclusiveConstraintComponent",
        explanation: "sh:minInclusive requires every value node to be a literal \
that compares greater than or equal to the given bound. Values are compared by \
type: numerics numerically, xsd:dateTime by timeline, other literals lexically; \
a value that cannot be compared to the bound (wrong type, non-literal) also \
violates.",
        failing_example: "ex:alice ex:age \"-3\"^^xsd:integer .\n# with: sh:path ex:age ; sh:minInclusive 0",
        fixed_example: "ex:alice ex:age \"3\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0008",
        title: "value is not less than the exclusive maximum",
        component: Some("http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MaxExclusiveConstraintComponent",
        explanation: "sh:maxExclusive requires every value node to be a literal that \
compares strictly less than the given bound. Comparison follows the same \
type-aware ordering as the other range constraints; a value equal to the \
bound, or one that cannot be compared to it at all, violates.",
        failing_example: "ex:alice ex:age \"100\"^^xsd:integer .\n# with: sh:path ex:age ; sh:maxExclusive 100",
        fixed_example: "ex:alice ex:age \"99\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0009",
        title: "value violates sh:maxInclusive",
        component: Some("http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MaxInclusiveConstraintComponent",
        explanation: "sh:maxInclusive requires every value node to be a literal that \
compares less than or equal to the given bound. Comparison follows the \
same type-aware ordering as the other range constraints; a value greater \
than the bound, or one that cannot be compared to it at all, violates.",
        failing_example: "ex:alice ex:age \"150\"^^xsd:integer .\n# with: sh:path ex:age ; sh:maxInclusive 120",
        fixed_example: "ex:alice ex:age \"80\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0010",
        title: "value is shorter than sh:minLength",
        component: Some("http://www.w3.org/ns/shacl#MinLengthConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MinLengthConstraintComponent",
        explanation: "sh:minLength requires every value node's string representation \
(the lexical form for literals, the IRI string for IRIs) to be at least \
the given number of characters long. Blank nodes have no string \
representation and always violate this constraint.",
        failing_example: "ex:alice ex:code \"ab\" .\n# with: sh:path ex:code ; sh:minLength 3",
        fixed_example: "ex:alice ex:code \"abc\" .",
    },
    RegistryEntry {
        code: "V0011",
        title: "value is longer than sh:maxLength",
        component: Some("http://www.w3.org/ns/shacl#MaxLengthConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#MaxLengthConstraintComponent",
        explanation: "sh:maxLength requires every value node's string representation \
(the lexical form for literals, the IRI string for IRIs) to be at most \
the given number of characters long. Blank nodes have no string \
representation and always violate this constraint.",
        failing_example: "ex:alice ex:code \"abcdef\" .\n# with: sh:path ex:code ; sh:maxLength 3",
        fixed_example: "ex:alice ex:code \"abc\" .",
    },
    RegistryEntry {
        code: "V0012",
        title: "value does not match sh:pattern",
        component: Some("http://www.w3.org/ns/shacl#PatternConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#PatternConstraintComponent",
        explanation: "sh:pattern requires every value node's string representation to \
match the given regular expression, evaluated with XPath/XQuery regex \
semantics and the flags from an optional sh:flags (e.g. \"i\" for \
case-insensitive matching). A value whose string form does not contain a \
match for the pattern violates; non-literal, non-IRI value nodes have no \
string representation and always violate.",
        failing_example: "ex:alice ex:code \"AB12\" .\n# with: sh:path ex:code ; sh:pattern \"^[a-z]+$\"",
        fixed_example: "ex:alice ex:code \"ab\" .",
    },
    RegistryEntry {
        code: "V0013",
        title: "language tag not permitted by sh:languageIn",
        component: Some("http://www.w3.org/ns/shacl#LanguageInConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#LanguageInConstraintComponent",
        explanation: "sh:languageIn requires every value node to be a literal whose \
language tag matches one of the given tags, or is a more specific \
sub-tag of one of them per BCP 47 (e.g. \"en-US\" matches \"en\"). A \
value with no language tag at all, or with a tag not covered by the \
list, violates.",
        failing_example: "ex:alice ex:label \"bonjour\"@fr .\n# with: sh:path ex:label ; sh:languageIn (\"en\" \"de\")",
        fixed_example: "ex:alice ex:label \"hello\"@en .",
    },
    RegistryEntry {
        code: "V0014",
        title: "language tag used more than once",
        component: Some("http://www.w3.org/ns/shacl#UniqueLangConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#UniqueLangConstraintComponent",
        explanation: "sh:uniqueLang true requires that, among the value nodes for the \
path at a given focus node, no language tag (compared case-insensitively) \
is used by more than one literal. The constraint reports one violation per \
duplicated language tag, not one per offending literal, and is ignored \
when set to false.",
        failing_example: "ex:alice ex:label \"hello\"@en , \"hi\"@en .\n# with: sh:path ex:label ; sh:uniqueLang true",
        fixed_example: "ex:alice ex:label \"hello\"@en , \"bonjour\"@fr .",
    },
    RegistryEntry {
        code: "V0015",
        title: "values differ from sh:equals property",
        component: Some("http://www.w3.org/ns/shacl#EqualsConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#EqualsConstraintComponent",
        explanation: "sh:equals requires the set of value nodes for the shape's path to be \
exactly the same set as the values of the given other property at the \
same focus node. Any value present on one side but not the other produces \
a validation result, so both a missing value and an extra value are \
reported as violations.",
        failing_example: "ex:alice ex:givenName \"Alice\" ; ex:firstName \"Alicia\" .\n# with: sh:path ex:givenName ; sh:equals ex:firstName",
        fixed_example: "ex:alice ex:givenName \"Alice\" ; ex:firstName \"Alice\" .",
    },
    RegistryEntry {
        code: "V0016",
        title: "value shared with sh:disjoint property",
        component: Some("http://www.w3.org/ns/shacl#DisjointConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#DisjointConstraintComponent",
        explanation: "sh:disjoint requires that no value node for the shape's path also \
appears among the values of the given other property at the same focus \
node. Any term appearing in both sets violates the constraint, one \
validation result per shared value.",
        failing_example: "ex:alice ex:homeEmail \"a@x.org\" ; ex:workEmail \"a@x.org\" .\n# with: sh:path ex:homeEmail ; sh:disjoint ex:workEmail",
        fixed_example: "ex:alice ex:homeEmail \"a@x.org\" ; ex:workEmail \"b@x.org\" .",
    },
    RegistryEntry {
        code: "V0017",
        title: "value not less than sh:lessThan property value",
        component: Some("http://www.w3.org/ns/shacl#LessThanConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#LessThanConstraintComponent",
        explanation: "sh:lessThan requires every value node to compare strictly less than \
every value of the given other property at the same focus node. A pair \
that is equal, out of order, or not comparable at all (different or \
non-orderable literal types) violates, one result per offending pair.",
        failing_example: "ex:alice ex:startDate \"2024-06-01\"^^xsd:date ; ex:endDate \"2024-01-01\"^^xsd:date .\n# with: sh:path ex:startDate ; sh:lessThan ex:endDate",
        fixed_example: "ex:alice ex:startDate \"2024-01-01\"^^xsd:date ; ex:endDate \"2024-06-01\"^^xsd:date .",
    },
    RegistryEntry {
        code: "V0018",
        title: "value not <= sh:lessThanOrEquals property value",
        component: Some("http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#LessThanOrEqualsConstraintComponent",
        explanation: "sh:lessThanOrEquals requires every value node to compare less than \
or equal to every value of the given other property at the same focus \
node. A pair that is out of order, or not comparable at all (different or \
non-orderable literal types), violates, one result per offending pair.",
        failing_example: "ex:alice ex:minPrice \"20\"^^xsd:integer ; ex:maxPrice \"10\"^^xsd:integer .\n# with: sh:path ex:minPrice ; sh:lessThanOrEquals ex:maxPrice",
        fixed_example: "ex:alice ex:minPrice \"10\"^^xsd:integer ; ex:maxPrice \"20\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0019",
        title: "required value missing",
        component: Some("http://www.w3.org/ns/shacl#HasValueConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#HasValueConstraintComponent",
        explanation: "sh:hasValue requires that the given fixed term (IRI, literal, or \
blank node) appears at least once among the value nodes for the shape at \
the focus node. If the term is absent, exactly one validation result is \
produced for the focus node regardless of how many other values it has.",
        failing_example: "ex:alice a ex:Employee .\n# with: sh:path rdf:type ; sh:hasValue ex:Manager",
        fixed_example: "ex:alice a ex:Employee , ex:Manager .",
    },
    RegistryEntry {
        code: "V0020",
        title: "value not in the allowed list",
        component: Some("http://www.w3.org/ns/shacl#InConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#InConstraintComponent",
        explanation: "sh:in requires every value node to be term-equal to one of the \
members of the given rdf:List. Term equality is exact - a literal must \
match datatype and language tag as well as lexical form - so a value \
absent from the list, even if semantically similar, violates.",
        failing_example: "ex:alice ex:status \"pending\" .\n# with: sh:path ex:status ; sh:in (\"active\" \"inactive\")",
        fixed_example: "ex:alice ex:status \"active\" .",
    },
    RegistryEntry {
        code: "V0021",
        title: "value does not conform to the referenced node shape",
        component: Some("http://www.w3.org/ns/shacl#NodeConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#NodeConstraintComponent",
        explanation: "sh:node requires every value node to itself conform, as a focus \
node, to the referenced node shape. In this implementation, literal value \
nodes are currently reported as violations of sh:node, since the referenced \
shape is evaluated against the value node as a focus node. Any nested \
violation against the referenced shape is surfaced as one validation \
result at the outer value node.",
        failing_example: "ex:alice ex:address [ ex:city \"\" ] .\n# with: sh:path ex:address ; sh:node ex:AddressShape (requires ex:city sh:minLength 1)",
        fixed_example: "ex:alice ex:address [ ex:city \"Berlin\" ] .",
    },
    RegistryEntry {
        code: "V0022",
        title: "too few values conform to the qualified shape",
        component: Some("http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#QualifiedMinCountConstraintComponent",
        explanation: "sh:qualifiedValueShape together with sh:qualifiedMinCount requires \
at least that many value nodes to conform to the qualified shape. When \
sh:qualifiedValueShapesDisjoint is true, value nodes conforming to a \
sibling qualified shape in the same sh:or/sh:and/sh:xone list are excluded \
from this shape's count so overlapping matches are not double-counted.",
        failing_example: "ex:alice ex:contact ex:homePhone .\n# with: sh:path ex:contact ; sh:qualifiedValueShape ex:PhoneShape ; sh:qualifiedMinCount 2",
        fixed_example: "ex:alice ex:contact ex:homePhone , ex:workPhone .",
    },
    RegistryEntry {
        code: "V0023",
        title: "too many values conform to the qualified shape",
        component: Some("http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#QualifiedMaxCountConstraintComponent",
        explanation: "sh:qualifiedValueShape together with sh:qualifiedMaxCount requires \
at most that many value nodes to conform to the qualified shape. As with \
sh:qualifiedMinCount, an active sh:qualifiedValueShapesDisjoint excludes \
values already counted by a sibling qualified shape in the same \
sh:or/sh:and/sh:xone list from this shape's count.",
        failing_example: "ex:alice ex:contact ex:phone1 , ex:phone2 .\n# with: sh:path ex:contact ; sh:qualifiedValueShape ex:PhoneShape ; sh:qualifiedMaxCount 1",
        fixed_example: "ex:alice ex:contact ex:phone1 .",
    },
    RegistryEntry {
        code: "V0024",
        title: "value fails a conjunct of sh:and",
        component: Some("http://www.w3.org/ns/shacl#AndConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#AndConstraintComponent",
        explanation: "sh:and requires each value node (or the focus node, for a node \
shape) to conform to every shape in the given list, a plain logical \
conjunction. Failing even one of the listed shapes is enough to violate \
the sh:and constraint, independent of how many of the others it passes.",
        failing_example: "ex:alice ex:age \"-1\"^^xsd:integer .\n# with: sh:path ex:age ; sh:and (ShapeMinInclusive0 ShapeIsInteger)",
        fixed_example: "ex:alice ex:age \"1\"^^xsd:integer .",
    },
    RegistryEntry {
        code: "V0025",
        title: "value matches no disjunct of sh:or",
        component: Some("http://www.w3.org/ns/shacl#OrConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#OrConstraintComponent",
        explanation: "sh:or requires each value node (or the focus node, for a node \
shape) to conform to at least one shape in the given list. Only when it \
conforms to none of the disjuncts is a violation reported for the whole \
sh:or constraint; conforming to two or more disjuncts is fine.",
        failing_example: "ex:alice ex:contact \"not-an-iri-or-phone\" .\n# with: sh:path ex:contact ; sh:or (ShapeIsIRI ShapePhonePattern)",
        fixed_example: "ex:alice ex:contact <tel:+15551234> .",
    },
    RegistryEntry {
        code: "V0026",
        title: "value does not match exactly one shape of sh:xone",
        component: Some("http://www.w3.org/ns/shacl#XoneConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#XoneConstraintComponent",
        explanation: "sh:xone requires each value node (or the focus node, for a node \
shape) to conform to exactly one shape in the given list - an exclusive \
or. Conforming to zero of the listed shapes, or to two or more of them \
simultaneously, both violate the constraint.",
        failing_example: "ex:alice ex:payment [ ex:cardNumber \"4111\" ; ex:iban \"DE89\" ] .\n# with: sh:path ex:payment ; sh:xone (CardShape IbanShape) (matches both)",
        fixed_example: "ex:alice ex:payment [ ex:cardNumber \"4111\" ] .",
    },
    RegistryEntry {
        code: "V0027",
        title: "value matches the negated shape",
        component: Some("http://www.w3.org/ns/shacl#NotConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#NotConstraintComponent",
        explanation: "sh:not requires each value node (or the focus node, for a node \
shape) to NOT conform to the given shape. If the value node does conform \
to the negated shape, the sh:not constraint itself is violated - the \
inner shape's own validation results are not reported, only the negation.",
        failing_example: "ex:alice a ex:Robot .\n# with: sh:path rdf:type ; sh:not [ sh:hasValue ex:Robot ]",
        fixed_example: "ex:alice a ex:Human .",
    },
    RegistryEntry {
        code: "V0028",
        title: "property not allowed on closed shape",
        component: Some("http://www.w3.org/ns/shacl#ClosedConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#ClosedConstraintComponent",
        explanation: "sh:closed true restricts a focus node to only use the properties \
declared via sh:property on the shape (identified by their sh:path), plus \
any additionally permitted by sh:ignoredProperties. Any other property \
present on the focus node produces one validation result per \
disallowed triple.",
        failing_example: "ex:alice ex:name \"Alice\" ; ex:extra \"oops\" .\n# with: sh:closed true ; sh:property [ sh:path ex:name ]",
        fixed_example: "ex:alice ex:name \"Alice\" .",
    },
    RegistryEntry {
        code: "V0029",
        title: "SPARQL constraint produced a violation",
        component: Some("http://www.w3.org/ns/shacl#SPARQLConstraintComponent"),
        spec_ref: "https://www.w3.org/TR/shacl/#SPARQLConstraintComponent",
        explanation: "sh:sparql attaches an arbitrary SPARQL SELECT query as a constraint; \
each solution the query returns for a given focus node (with $this bound) \
is a distinct validation result, letting the query's own ?message, ?path, \
and ?value bindings override the defaults. A query producing zero \
solutions passes; any solution row is a violation.",
        failing_example: "ex:alice ex:end \"2024-01-01\"^^xsd:date ; ex:start \"2024-06-01\"^^xsd:date .\n# sh:sparql SELECT WHERE { $this ex:end ?e ; ex:start ?s . FILTER (?e < ?s) }",
        fixed_example: "ex:alice ex:start \"2024-01-01\"^^xsd:date ; ex:end \"2024-06-01\"^^xsd:date .",
    },
    RegistryEntry {
        code: "L0001",
        title: "property shape without sh:path",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#property-shapes",
        explanation: "Every property shape must declare exactly one sh:path; it is what \
tells the engine which value nodes to select at each focus node. A shape \
used as an sh:property value (or otherwise identifiable as a property \
shape) that has no sh:path is malformed and is silently skipped by \
validation - any constraints on it never run, so this lint flags it \
before it causes silent under-validation.",
        failing_example: "ex:PersonShape sh:property [ sh:minCount 1 ] .",
        fixed_example: "ex:PersonShape sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    },
    RegistryEntry {
        code: "L0002",
        title: "invalid sh:pattern regex",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#PatternConstraintComponent",
        explanation: "sh:pattern's value must be a string that parses as a valid regular \
expression under the engine's regex dialect. A malformed pattern (e.g. \
unbalanced brackets) cannot be compiled, so today the constraint silently \
never fires and every value passes - the exact opposite of what the shape \
author intended. This lint flags the shape so the pattern can be fixed \
rather than silently doing nothing.",
        failing_example: "ex:CodeShape sh:property [ sh:path ex:code ; sh:pattern \"^[a-z(+$\" ] .",
        fixed_example: "ex:CodeShape sh:property [ sh:path ex:code ; sh:pattern \"^[a-z]+$\" ] .",
    },
    RegistryEntry {
        code: "L0003",
        title: "SPARQL constraint/target query fails to parse",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#sparql-constraints",
        explanation: "sh:sparql, sh:select (for SPARQL-based targets), and related \
query-bearing properties must hold a syntactically valid SPARQL query \
string, since it is parsed and executed with $this substituted for the \
focus node. A query with a syntax error can never be executed, so the \
constraint or target it defines is inert; this lint surfaces the parse \
failure at shape-load time instead of at validation time.",
        failing_example: "ex:Shape sh:sparql [ sh:select \"SELECT WHERE $this ex:p ?v\" ] .",
        fixed_example: "ex:Shape sh:sparql [ sh:select \"SELECT $this WHERE { $this ex:p ?v }\" ] .",
    },
    RegistryEntry {
        code: "L0004",
        title: "unknown sh:-namespace term",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#shapes",
        explanation: "Terms in the sh: namespace that appear on a shape but are not part \
of SHACL core (not a recognized shape, target, or constraint parameter) \
are almost always typos, since custom vocabulary should live in a \
project's own namespace. This lint flags any such term and, when a known \
sh: term is within Levenshtein distance 2, suggests it as the likely \
intended spelling.",
        failing_example: "ex:PersonShape sh:minCont 1 .\n# did you mean sh:minCount?",
        fixed_example: "ex:PersonShape sh:minCount 1 .",
    },
    RegistryEntry {
        code: "L0005",
        title: "path-requiring constraint on a node shape",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#node-shapes",
        explanation: "Constraint parameters such as sh:minCount, sh:maxCount, sh:equals, \
sh:disjoint, sh:lessThan, and sh:lessThanOrEquals are only meaningful \
together with an sh:path, i.e. on a property shape; placed directly on a \
node shape (no sh:path) they are ignored by validation today. This lint \
flags the mismatch so the constraint can be moved into an sh:property \
block where it will actually be evaluated.",
        failing_example: "ex:PersonShape a sh:NodeShape ; sh:minCount 1 .",
        fixed_example: "ex:PersonShape a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    },
    RegistryEntry {
        code: "L0006",
        title: "unsatisfiable bounds",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#MinCountConstraintComponent",
        explanation: "When a shape declares both ends of a bound pair - sh:minCount with \
sh:maxCount, sh:minLength with sh:maxLength, or sh:minInclusive/ \
sh:minExclusive with sh:maxInclusive/sh:maxExclusive - and the lower bound \
exceeds the upper bound, no value can ever satisfy the shape: every focus \
node with any value, and often every focus node at all, is guaranteed to \
fail. This lint catches the contradiction at shape-authoring time.",
        failing_example: "ex:Shape sh:property [ sh:path ex:age ; sh:minCount 5 ; sh:maxCount 2 ] .",
        fixed_example: "ex:Shape sh:property [ sh:path ex:age ; sh:minCount 1 ; sh:maxCount 2 ] .",
    },
    RegistryEntry {
        code: "L0007",
        title: "sh:class/sh:datatype/sh:nodeKind object has wrong term kind",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#ClassConstraintComponent",
        explanation: "sh:class and sh:datatype both require an IRI as their object (a \
class name, or an XSD/RDF datatype IRI), and sh:nodeKind requires one of \
the fixed sh: node-kind IRIs; a literal or blank node in that position \
can never match anything in the data graph, so the constraint silently \
rejects every value. This lint flags the malformed object so it can be \
corrected to the intended IRI.",
        failing_example: "ex:Shape sh:property [ sh:path ex:type ; sh:class \"Person\" ] .",
        fixed_example: "ex:Shape sh:property [ sh:path ex:type ; sh:class ex:Person ] .",
    },
    RegistryEntry {
        code: "L0008",
        title: "empty sh:in / sh:languageIn list",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#InConstraintComponent",
        explanation: "sh:in and sh:languageIn both take a non-empty rdf:List; an empty \
list means no term (respectively no language tag) can ever satisfy the \
constraint, so every focus node with at least one value is guaranteed to \
fail. This is almost always an authoring mistake - an incomplete list \
literal, or a variable that failed to expand - rather than an intentional \
always-fail rule.",
        failing_example: "ex:Shape sh:property [ sh:path ex:status ; sh:in () ] .",
        fixed_example: "ex:Shape sh:property [ sh:path ex:status ; sh:in (\"active\" \"inactive\") ] .",
    },
    RegistryEntry {
        code: "L0009",
        title: "non-comparable bound on sh:minInclusive/co.",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#MinInclusiveConstraintComponent",
        explanation: "sh:minInclusive, sh:minExclusive, sh:maxInclusive, and \
sh:maxExclusive expect a literal bound with an orderable datatype \
(numeric, xsd:dateTime/xsd:date/xsd:time, or another literal comparable \
lexically); a bound that is an IRI, a blank node, or a literal type with \
no defined ordering can never be compared to any value, so the constraint \
rejects everything it is checked against. This lint flags the \
non-comparable bound.",
        failing_example: "ex:Shape sh:property [ sh:path ex:age ; sh:minInclusive ex:NotALiteral ] .",
        fixed_example: "ex:Shape sh:property [ sh:path ex:age ; sh:minInclusive 0 ] .",
    },
    RegistryEntry {
        code: "L0010",
        title: "sh:ignoredProperties without sh:closed",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#ClosedConstraintComponent",
        explanation: "sh:ignoredProperties only has an effect as a modifier of sh:closed \
true - it lists properties that are exempt from the closed-shape check. \
Declared without sh:closed true on the same shape it does nothing at all, \
so its presence usually signals a shape that was meant to be closed but \
is missing that declaration.",
        failing_example: "ex:PersonShape sh:ignoredProperties (rdf:type) ; sh:property [ sh:path ex:name ] .",
        fixed_example: "ex:PersonShape sh:closed true ; sh:ignoredProperties (rdf:type) ; sh:property [ sh:path ex:name ] .",
    },
    RegistryEntry {
        code: "L0011",
        title: "dead shape: no targets and referenced by no other shape",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#shapes",
        explanation: "A shape only participates in validation if it either has its own \
target (sh:targetClass, sh:targetNode, sh:targetSubjectsOf, \
sh:targetObjectsOf, or implicit class-based targeting) or is reachable \
from a targeted shape via sh:node, sh:property, sh:and/sh:or/sh:xone/ \
sh:not, or sh:qualifiedValueShape. A shape with neither is never \
evaluated against any focus node - dead weight in the shapes graph, often \
left behind after a refactor.",
        failing_example: "ex:OrphanShape a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n# nothing targets or references ex:OrphanShape",
        fixed_example: "ex:OrphanShape a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    },
    RegistryEntry {
        code: "L0012",
        title: "sh:deactivated shape reminder",
        component: None,
        spec_ref: "https://www.w3.org/TR/shacl/#shapes",
        explanation: "sh:deactivated true tells the engine to skip this shape and all its \
nested constraints entirely, as if it were not part of the shapes graph, \
while still leaving it visible in the source for later reactivation. \
Since a deactivated shape produces no violations no matter what the data \
looks like, this lint is an Info-level reminder that validation coverage \
here is intentionally turned off, not a defect.",
        failing_example: "ex:PersonShape sh:deactivated true ; sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        fixed_example: "ex:PersonShape sh:targetClass ex:Person ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
    },
];

pub fn all_entries() -> &'static [RegistryEntry] {
    ENTRIES
}

pub fn entry(code: &str) -> Option<&'static RegistryEntry> {
    ENTRIES.iter().find(|e| e.code == code)
}

pub fn code_for_component(component_iri: &str) -> &'static str {
    ENTRIES
        .iter()
        .find(|e| e.component == Some(component_iri))
        .map(|e| e.code)
        .unwrap_or("V0000")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_component_maps_to_a_specific_code() {
        let iri = "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";
        assert_eq!(code_for_component(iri), "V0007");
        assert_eq!(code_for_component("http://example.org/custom"), "V0000");
    }

    #[test]
    fn every_code_has_a_complete_entry() {
        for e in all_entries() {
            assert!(!e.title.is_empty(), "{} missing title", e.code);
            assert!(!e.explanation.is_empty(), "{} missing explanation", e.code);
            assert!(!e.failing_example.is_empty(), "{} missing example", e.code);
            assert!(!e.fixed_example.is_empty(), "{} missing fix", e.code);
            assert!(!e.spec_ref.is_empty(), "{} missing spec ref", e.code);
        }
    }

    #[test]
    fn v_codes_carry_component_and_l_codes_do_not() {
        for e in all_entries() {
            if e.code.starts_with('V') && e.code != "V0000" {
                assert!(e.component.is_some(), "{}", e.code);
            }
            if e.code.starts_with('L') {
                assert!(e.component.is_none(), "{}", e.code);
            }
        }
    }

    #[test]
    fn lookup_by_code() {
        assert_eq!(entry("V0007").unwrap().code, "V0007");
        assert!(entry("V9999").is_none());
    }
}
