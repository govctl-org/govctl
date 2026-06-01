use super::*;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult};
use crate::loop_state::LoopWorkItemStatus;
use crate::model::{
    WorkItemContent, WorkItemEntry, WorkItemMeta, WorkItemSpec, WorkItemStatus,
    WorkItemVerification,
};
use std::path::PathBuf;

fn work_item(id: &str, status: WorkItemStatus, depends_on: &[&str]) -> WorkItemEntry {
    let mut meta = WorkItemMeta::new(id, id, status);
    meta.depends_on = depends_on.iter().map(|id| (*id).to_string()).collect();

    WorkItemEntry {
        spec: WorkItemSpec {
            govctl: meta,
            content: WorkItemContent::default(),
            verification: WorkItemVerification::default(),
        },
        path: PathBuf::from(format!("{id}.toml")),
    }
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn assert_diagnostic_code<T>(
    result: DiagnosticResult<T>,
    code: DiagnosticCode,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Err(diagnostic) = result else {
        return Err(format!("expected diagnostic {}", code.code()).into());
    };
    assert_eq!(diagnostic.code, code);
    assert!(
        diagnostic.message.contains(text),
        "diagnostic should contain '{text}', got: {}",
        diagnostic.message
    );
    Ok(())
}

#[test]
fn test_loop_plan_single_work_item() -> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-010";
    let plan = build_loop_plan(
        "LOOP-2026-05-31-010",
        &ids(&[root]),
        &[work_item(root, WorkItemStatus::Queue, &[])],
    )?;

    assert_eq!(plan.topological_order, ids(&[root]));
    assert_eq!(plan.state.loop_meta.root_work_items, ids(&[root]));
    assert_eq!(plan.state.loop_meta.work_items, ids(&[root]));
    assert_eq!(plan.state.dependencies[root], Vec::<String>::new());
    assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Pending);
    Ok(())
}

#[test]
fn test_loop_plan_resolves_dependency_closure_and_order() -> Result<(), Box<dyn std::error::Error>>
{
    let root = "WI-2026-05-31-014";
    let dependency_a = "WI-2026-05-31-011";
    let dependency_b = "WI-2026-05-31-012";
    let transitive = "WI-2026-05-31-013";
    let plan = build_loop_plan(
        "LOOP-2026-05-31-014",
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[dependency_a, dependency_b]),
            work_item(dependency_a, WorkItemStatus::Queue, &[transitive]),
            work_item(dependency_b, WorkItemStatus::Queue, &[]),
            work_item(transitive, WorkItemStatus::Queue, &[]),
        ],
    )?;

    assert_eq!(
        plan.state.loop_meta.work_items,
        ids(&[dependency_a, dependency_b, transitive, root])
    );
    assert_eq!(
        plan.topological_order,
        ids(&[dependency_b, transitive, dependency_a, root])
    );
    assert_eq!(
        plan.state.dependencies[root],
        ids(&[dependency_a, dependency_b])
    );
    assert_eq!(plan.state.dependencies[dependency_a], ids(&[transitive]));
    Ok(())
}

#[test]
fn test_loop_plan_rejects_missing_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-020";
    let missing = "WI-2026-05-31-999";

    assert_diagnostic_code(
        build_loop_plan(
            "LOOP-2026-05-31-020",
            &ids(&[root]),
            &[work_item(root, WorkItemStatus::Queue, &[missing])],
        ),
        DiagnosticCode::E1205LoopDependencyNotFound,
        missing,
    )
}

#[test]
fn test_loop_plan_rejects_dependency_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let first = "WI-2026-05-31-030";
    let second = "WI-2026-05-31-031";

    assert_diagnostic_code(
        build_loop_plan(
            "LOOP-2026-05-31-030",
            &ids(&[first]),
            &[
                work_item(first, WorkItemStatus::Queue, &[second]),
                work_item(second, WorkItemStatus::Queue, &[first]),
            ],
        ),
        DiagnosticCode::E1206LoopDependencyCycle,
        first,
    )
}

#[test]
fn test_loop_plan_propagates_blocked_outcomes() -> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-043";
    let middle = "WI-2026-05-31-042";
    let failed = "WI-2026-05-31-041";
    let mut plan = build_loop_plan(
        "LOOP-2026-05-31-043",
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[middle]),
            work_item(middle, WorkItemStatus::Queue, &[failed]),
            work_item(failed, WorkItemStatus::Queue, &[]),
        ],
    )?;

    plan.state
        .set_item_status(failed, LoopWorkItemStatus::Failed)?;
    let blocked = propagate_blocked_outcomes(&mut plan.state)?;

    assert_eq!(blocked, ids(&[middle, root]));
    assert_eq!(plan.state.items[middle].status, LoopWorkItemStatus::Blocked);
    assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Blocked);
    Ok(())
}

#[test]
fn test_loop_plan_marks_dependents_blocked_for_pre_existing_cancelled_dependency()
-> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-052";
    let done_middle = "WI-2026-05-31-051";
    let cancelled = "WI-2026-05-31-050";
    let plan = build_loop_plan(
        "LOOP-2026-05-31-052",
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[done_middle]),
            work_item(done_middle, WorkItemStatus::Done, &[cancelled]),
            work_item(cancelled, WorkItemStatus::Cancelled, &[]),
        ],
    )?;

    assert_eq!(
        plan.state.items[cancelled].status,
        LoopWorkItemStatus::Cancelled
    );
    assert_eq!(
        plan.state.items[done_middle].status,
        LoopWorkItemStatus::Blocked
    );
    assert_eq!(plan.state.items[root].status, LoopWorkItemStatus::Blocked);
    assert_eq!(plan.topological_order, ids(&[cancelled, done_middle, root]));
    Ok(())
}

#[test]
fn test_replan_uses_current_cancelled_work_status_over_previous_pending_loop_state()
-> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-062";
    let dependency = "WI-2026-05-31-061";
    let plan = build_loop_plan(
        "LOOP-2026-05-31-062",
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[dependency]),
            work_item(dependency, WorkItemStatus::Queue, &[]),
        ],
    )?;

    let replanned = replan_loop_state(
        &plan.state,
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[dependency]),
            work_item(dependency, WorkItemStatus::Cancelled, &[]),
        ],
    )?;

    assert_eq!(
        replanned.state.items[dependency].status,
        LoopWorkItemStatus::Cancelled
    );
    assert_eq!(
        replanned.state.items[root].status,
        LoopWorkItemStatus::Blocked
    );
    Ok(())
}

#[test]
fn test_replan_preserves_previous_terminal_loop_state_over_current_work_status()
-> Result<(), Box<dyn std::error::Error>> {
    let root = "WI-2026-05-31-072";
    let failed_dependency = "WI-2026-05-31-071";
    let mut plan = build_loop_plan(
        "LOOP-2026-05-31-072",
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Queue, &[failed_dependency]),
            work_item(failed_dependency, WorkItemStatus::Queue, &[]),
        ],
    )?;
    plan.state
        .set_item_status(failed_dependency, LoopWorkItemStatus::Failed)?;
    plan.state.set_item_status(root, LoopWorkItemStatus::Done)?;

    let replanned = replan_loop_state(
        &plan.state,
        &ids(&[root]),
        &[
            work_item(root, WorkItemStatus::Cancelled, &[failed_dependency]),
            work_item(failed_dependency, WorkItemStatus::Done, &[]),
        ],
    )?;

    assert_eq!(
        replanned.state.items[failed_dependency].status,
        LoopWorkItemStatus::Failed
    );
    assert_eq!(replanned.state.items[root].status, LoopWorkItemStatus::Done);
    Ok(())
}
