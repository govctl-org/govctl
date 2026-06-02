use crate::OutputFormat;
use crate::cmd::output::{print_json_array, table_with_bold_headers};
use comfy_table::Cell;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct TagEntry {
    pub(super) tag: String,
    pub(super) usage: usize,
}

pub(super) fn print_tag_entries(entries: &[TagEntry], output: OutputFormat) {
    match output {
        OutputFormat::Json => {
            print_json_array(entries);
        }
        OutputFormat::Plain => {
            for entry in entries {
                println!("{}\t{}", entry.tag, entry.usage);
            }
        }
        OutputFormat::Table => {
            let mut table = table_with_bold_headers(&["Tag", "Usage"]);
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
