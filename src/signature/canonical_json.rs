//! Deterministic JSON serialization for signature computation.

use serde_json::Value;

/// Canonicalize a JSON value:
/// - Object keys sorted alphabetically, recursively
/// - Arrays preserve order
/// - Compact format with no extra whitespace
pub(super) fn canonicalize_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical_json(value, &mut out);
    out
}

fn write_canonical_json(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::String(s) => {
            if let Ok(escaped) = serde_json::to_string(s) {
                out.push_str(&escaped);
            }
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical_json(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Ok(escaped_key) = serde_json::to_string(*key) {
                    out.push_str(&escaped_key);
                }
                out.push(':');
                write_canonical_json(&map[*key], out);
            }
            out.push('}');
        }
    }
}
