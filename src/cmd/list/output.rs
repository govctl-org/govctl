use crate::OutputFormat;
use crate::cmd::output::print_json_array;
use crate::theme::{SemanticColor, status_semantic};
use crate::ui::stdout_supports_color;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;

fn cell(text: &str) -> Cell {
    Cell::new(text)
}

fn id_cell(text: &str, use_colors: bool) -> Cell {
    if use_colors {
        Cell::new(text)
            .fg(SemanticColor::Info.to_comfy())
            .add_attribute(Attribute::Bold)
    } else {
        Cell::new(text)
    }
}

fn status_cell(status: &str, use_colors: bool) -> Cell {
    if use_colors {
        Cell::new(status).fg(status_semantic(status).to_comfy())
    } else {
        Cell::new(status)
    }
}

fn header_cell(text: &str, use_colors: bool) -> Cell {
    if use_colors {
        Cell::new(text).add_attribute(Attribute::Bold)
    } else {
        Cell::new(text)
    }
}

pub(super) fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
    }
}

pub(super) fn output_list<T: Serialize>(
    items: &[T],
    headers: &[&str],
    format: OutputFormat,
    to_row: impl Fn(&T) -> Vec<String>,
) {
    match format {
        OutputFormat::Json => {
            print_json_array(items);
        }
        OutputFormat::Plain => {
            for item in items {
                let row = to_row(item);
                println!("{}", row.join("\t"));
            }
        }
        OutputFormat::Table => {
            let use_colors = stdout_supports_color();
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(
                    headers
                        .iter()
                        .map(|h| header_cell(h, use_colors))
                        .collect::<Vec<_>>(),
                );

            for item in items {
                let row = to_row(item);
                table.add_row(
                    row.iter()
                        .enumerate()
                        .map(|(i, v)| {
                            if i == 0 {
                                id_cell(v, use_colors)
                            } else if headers
                                .get(i)
                                .is_some_and(|h| *h == "Status" || *h == "Phase")
                            {
                                status_cell(v, use_colors)
                            } else {
                                cell(v)
                            }
                        })
                        .collect::<Vec<_>>(),
                );
            }

            println!("{table}");
        }
    }
}
