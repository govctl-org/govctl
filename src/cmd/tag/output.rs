use crate::OutputFormat;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct TagEntry {
    pub(super) tag: String,
    pub(super) usage: usize,
}

pub(super) fn print_tag_entries(entries: &[TagEntry], output: OutputFormat) {
    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
            );
        }
        OutputFormat::Plain => {
            for entry in entries {
                println!("{}\t{}", entry.tag, entry.usage);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("Tag").add_attribute(Attribute::Bold),
                    Cell::new("Usage").add_attribute(Attribute::Bold),
                ]);
            for entry in entries {
                table.add_row(vec![
                    Cell::new(&entry.tag),
                    Cell::new(entry.usage.to_string()),
                ]);
            }
            println!("{table}");
        }
    }
}
