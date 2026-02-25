use oxigraph::model::{NamedNodeRef, TermRef};

use crate::ValidationResult;

/// Builder for `ValidationResult`.
#[derive(Debug, Clone)]
pub struct ViolationBuilder<'a> {
    pub focus_node: TermRef<'a>,
    pub value: Option<TermRef<'a>>,
    pub constraint_messages: Vec<String>,
    pub constraint_component: Option<NamedNodeRef<'a>>,
    pub constraint_detail: Option<String>,
    pub trace: Vec<String>,
    pub details: Vec<ValidationResult<'a>>,
}

impl<'a> ViolationBuilder<'a> {
    pub fn new(focus_node: TermRef<'a>) -> Self {
        Self {
            focus_node,
            value: None,
            constraint_messages: Vec::new(),
            constraint_component: None,
            constraint_detail: None,
            trace: Vec::new(),
            details: Vec::new(),
        }
    }

    pub fn value(mut self, value: TermRef<'a>) -> Self {
        self.value = Some(value);
        self
    }

    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.constraint_messages.push(msg.into());
        self
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = String>) -> Self {
        self.constraint_messages.extend(messages);
        self
    }

    pub fn component(mut self, component: NamedNodeRef<'a>) -> Self {
        self.constraint_component = Some(component);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.constraint_detail = Some(detail.into());
        self
    }

    pub fn trace(mut self, trace: Vec<String>) -> Self {
        self.trace = trace;
        self
    }

    pub fn trace_entry(mut self, entry: impl Into<String>) -> Self {
        self.trace.push(entry.into());
        self
    }

    pub fn details(mut self, details: Vec<ValidationResult<'a>>) -> Self {
        self.details = details;
        self
    }
}
