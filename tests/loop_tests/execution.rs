use crate::common;
use crate::common::loop_helpers::{
    append_required_guard, loop_id, loop_item_round_count, loop_item_status, read_round_record,
    toml_int, toml_string, validate_toml_against_schema, write_guard,
};
use crate::common::{init_project, run_dynamic_commands};
use std::fs;

include!("execution_cases/lifecycle.rs");
include!("execution_cases/guards.rs");
include!("execution_cases/targeting.rs");
