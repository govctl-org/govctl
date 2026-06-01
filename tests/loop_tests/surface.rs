use crate::common;
use crate::common::loop_helpers::{assert_schema_rejects, loop_id};
use crate::common::{init_project, run_dynamic_commands};
use std::fs;

include!("surface_cases/start.rs");
include!("surface_cases/listing.rs");
include!("surface_cases/validation.rs");
