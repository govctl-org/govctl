use serde::Serialize;

pub(crate) fn print_json_array<T: Serialize>(items: &[T]) {
    println!(
        "{}",
        serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
    );
}
