//! Status command implementation.

use crate::config::Config;
use crate::diagnostic::{DiagnosticResult, Diagnostics};
use crate::load::load_project;
use crate::model::{AdrStatus, ClauseStatus, RfcPhase, RfcStatus, WorkItemEntry, WorkItemStatus};
use crate::status_counts::{StatusCounts, count_by, count_for, total_count};
use crate::theme::status_semantic;
use crate::ui::stdout_supports_color;
use owo_colors::OwoColorize;
use std::hash::Hash;

struct StatusPrinter {
    colors: bool,
}

struct StatusRow<K> {
    label: &'static str,
    key: K,
}

impl<K> StatusRow<K> {
    fn new(label: &'static str, key: K) -> Self {
        Self { label, key }
    }
}

struct StatusSection<'a, K> {
    title: &'static str,
    counts: &'a StatusCounts<K>,
    rows: &'a [StatusRow<K>],
    total: usize,
}

impl StatusPrinter {
    fn new() -> Self {
        Self {
            colors: stdout_supports_color(),
        }
    }

    fn title(&self) {
        if self.colors {
            println!("{}", "govctl status".bold());
        } else {
            println!("govctl status");
        }
    }

    fn section_header(&self, title: &str) {
        if self.colors {
            println!("\n{}", title.bold().underline());
        } else {
            println!("\n{}", title);
        }
    }

    fn status_line(&self, label: &str, count: usize, status: &str) {
        if count == 0 {
            return;
        }

        if self.colors {
            let color = status_semantic(status).to_owo();
            let count_str = count.to_string();
            println!(
                "  {:12} {}",
                label.color(color),
                count_str.color(color).bold()
            );
        } else {
            println!("  {:12} {}", label, count);
        }
    }

    fn total_line(&self, count: usize) {
        if self.colors {
            println!("  {:12} {}", "Total".dimmed(), count.to_string().bold());
        } else {
            println!("  {:12} {}", "Total", count);
        }
    }

    fn status_section<K>(&self, section: StatusSection<'_, K>)
    where
        K: Copy + Eq + Hash,
    {
        self.status_section_with_extra(section, |_| {});
    }

    fn status_section_with_extra<K, Extra>(&self, section: StatusSection<'_, K>, extra: Extra)
    where
        K: Copy + Eq + Hash,
        Extra: FnOnce(&Self),
    {
        self.section_header(section.title);
        self.status_breakdown(section.counts, section.rows);
        extra(self);
        self.total_line(section.total);
    }

    fn status_breakdown<K>(&self, counts: &StatusCounts<K>, rows: &[StatusRow<K>])
    where
        K: Copy + Eq + Hash,
    {
        for row in rows {
            self.status_line(row.label, count_for(counts, row.key), row.label);
        }
    }

    fn phase_breakdown(&self, spec: usize, impl_phase: usize, test: usize, stable: usize) {
        if spec == 0 && impl_phase == 0 && test == 0 {
            return;
        }

        println!();
        if self.colors {
            print!("  {} ", "phases".dimmed());
            self.pending_phase_count("spec", spec);
            self.pending_phase_count("impl", impl_phase);
            self.pending_phase_count("test", test);
            print!("stable:");
            println!("{}", stable.to_string().green());
        } else {
            println!(
                "  phases spec:{} impl:{} test:{} stable:{}",
                spec, impl_phase, test, stable
            );
        }
    }

    fn active_work(&self, active_items: &[&WorkItemEntry]) {
        if active_items.is_empty() {
            return;
        }

        self.section_header("Active Work");
        for item in active_items {
            if self.colors {
                println!("  {} {}", item.meta().id.cyan().bold(), item.meta().title);
            } else {
                println!("  {} {}", item.meta().id, item.meta().title);
            }
        }
    }

    fn pending_phase_count(&self, label: &str, count: usize) {
        print!("{label}:");
        if count > 0 {
            print!("{} ", count.to_string().yellow());
        } else {
            print!("{} ", "0".dimmed());
        }
    }
}

/// Show summary status
pub fn show_status(config: &Config) -> DiagnosticResult<Diagnostics> {
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };
    let printer = StatusPrinter::new();

    printer.title();

    let by_status = count_by(&index.rfcs, |rfc| rfc.rfc.status);
    let by_phase = count_by(&index.rfcs, |rfc| rfc.rfc.phase);
    let spec = count_for(&by_phase, RfcPhase::Spec);
    let impl_phase = count_for(&by_phase, RfcPhase::Impl);
    let test = count_for(&by_phase, RfcPhase::Test);
    let stable = count_for(&by_phase, RfcPhase::Stable);

    printer.status_section_with_extra(
        StatusSection {
            title: "RFCs",
            counts: &by_status,
            rows: &[
                StatusRow::new("draft", RfcStatus::Draft),
                StatusRow::new("normative", RfcStatus::Normative),
                StatusRow::new("deprecated", RfcStatus::Deprecated),
            ],
            total: index.rfcs.len(),
        },
        |printer| printer.phase_breakdown(spec, impl_phase, test, stable),
    );

    let clause_by_status = count_by(index.iter_clauses(), |(_, clause)| clause.spec.status);
    printer.status_section(StatusSection {
        title: "Clauses",
        counts: &clause_by_status,
        rows: &[
            StatusRow::new("active", ClauseStatus::Active),
            StatusRow::new("deprecated", ClauseStatus::Deprecated),
            StatusRow::new("superseded", ClauseStatus::Superseded),
        ],
        total: total_count(&clause_by_status),
    });

    let adr_by_status = count_by(&index.adrs, |adr| adr.meta().status);
    printer.status_section(StatusSection {
        title: "ADRs",
        counts: &adr_by_status,
        rows: &[
            StatusRow::new("proposed", AdrStatus::Proposed),
            StatusRow::new("accepted", AdrStatus::Accepted),
            StatusRow::new("superseded", AdrStatus::Superseded),
        ],
        total: index.adrs.len(),
    });

    let work_by_status = count_by(&index.work_items, |item| item.meta().status);
    printer.status_section(StatusSection {
        title: "Work Items",
        counts: &work_by_status,
        rows: &[
            StatusRow::new("queue", WorkItemStatus::Queue),
            StatusRow::new("active", WorkItemStatus::Active),
            StatusRow::new("done", WorkItemStatus::Done),
            StatusRow::new("cancelled", WorkItemStatus::Cancelled),
        ],
        total: index.work_items.len(),
    });

    let active_items: Vec<_> = index
        .work_items
        .iter()
        .filter(|w| w.meta().status == WorkItemStatus::Active)
        .collect();

    printer.active_work(&active_items);

    println!();
    Ok(vec![])
}
