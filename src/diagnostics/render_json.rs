pub fn render_ndjson(_d: &[super::Diagnostic]) -> String {
    String::new()
}

pub fn diagnostic_to_json(_d: &super::Diagnostic) -> serde_json::Value {
    serde_json::Value::Null
}
