const RFC_METADATA_KEYS: &[&str] = &[
    "title",
    "version",
    "status",
    "phase",
    "owners",
    "created",
    "updated",
    "supersedes",
    "refs",
    "signature",
];

const CLAUSE_METADATA_KEYS: &[&str] = &[
    "title",
    "kind",
    "status",
    "anchors",
    "superseded_by",
    "since",
];

fn move_toml_keys(
    source: &mut toml::map::Map<String, toml::Value>,
    target: &mut toml::map::Map<String, toml::Value>,
    keys: &[&str],
) {
    for key in keys {
        if let Some(v) = source.remove(*key) {
            target.insert(key.to_string(), v);
        }
    }
}

fn extract_toml_govctl(
    root: &mut toml::map::Map<String, toml::Value>,
    id_key: &str,
    metadata_keys: &[&str],
) -> Option<toml::Value> {
    if root.contains_key("govctl") {
        return None;
    }
    let id = root.remove(id_key)?;

    let mut govctl = toml::map::Map::new();
    govctl.insert("schema".to_string(), toml::Value::Integer(1));
    govctl.insert("id".to_string(), id);
    move_toml_keys(root, &mut govctl, metadata_keys);
    Some(toml::Value::Table(govctl))
}

/// Normalize a flat RFC TOML value into the `[govctl]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_rfc_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    if let Some(govctl) = extract_toml_govctl(root, "rfc_id", RFC_METADATA_KEYS) {
        root.insert("govctl".to_string(), govctl);
    }
}

/// Normalize a flat clause TOML value into the `[govctl]` + `[content]` wire layout.
/// If the value already has a `govctl` key, it's left untouched.
pub fn normalize_clause_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    let Some(govctl) = extract_toml_govctl(root, "clause_id", CLAUSE_METADATA_KEYS) else {
        return;
    };
    root.insert("govctl".to_string(), govctl);

    let mut content = toml::map::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), toml::Value::Table(content));
}

fn move_json_keys(
    source: &mut serde_json::Map<String, serde_json::Value>,
    target: &mut serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) {
    for key in keys {
        if let Some(v) = source.remove(*key) {
            target.insert(key.to_string(), v);
        }
    }
}

fn extract_json_govctl(
    root: &mut serde_json::Map<String, serde_json::Value>,
    id_key: &str,
    metadata_keys: &[&str],
) -> Option<serde_json::Value> {
    if root.contains_key("govctl") {
        return None;
    }
    let id = root.remove(id_key)?;
    let mut govctl = serde_json::Map::new();
    govctl.insert("schema".to_string(), serde_json::json!(1));
    govctl.insert("id".to_string(), id);
    move_json_keys(root, &mut govctl, metadata_keys);
    Some(serde_json::Value::Object(govctl))
}

/// Normalize a flat RFC JSON value into the `govctl` wire layout.
pub(crate) fn normalize_rfc_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    if let Some(govctl) = extract_json_govctl(root, "rfc_id", RFC_METADATA_KEYS) {
        root.insert("govctl".to_string(), govctl);
    }
}

/// Normalize a flat clause JSON value into the `govctl` + `content` wire layout.
pub(crate) fn normalize_clause_json(raw: &mut serde_json::Value) {
    let Some(root) = raw.as_object_mut() else {
        return;
    };
    let Some(govctl) = extract_json_govctl(root, "clause_id", CLAUSE_METADATA_KEYS) else {
        return;
    };
    root.insert("govctl".to_string(), govctl);

    let mut content = serde_json::Map::new();
    if let Some(text) = root.remove("text") {
        content.insert("text".to_string(), text);
    }
    root.insert("content".to_string(), serde_json::Value::Object(content));
}
