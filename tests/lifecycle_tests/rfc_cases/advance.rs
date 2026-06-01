// ============================================================================
// RFC Advance Tests
// ============================================================================

#[test]
fn test_advance_spec_to_impl() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_impl_to_test() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_test_to_stable() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "test"],
            &["rfc", "advance", "RFC-0001", "stable"],
            &["rfc", "list"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_draft_to_impl_fails() -> common::TestResult {
    // Cannot advance draft RFC to impl phase
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "advance", "RFC-0001", "impl"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_skip_phase_fails() -> common::TestResult {
    // Cannot skip phases (e.g., spec -> test)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "test"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_backwards_fails() -> common::TestResult {
    // Cannot go backwards (e.g., impl -> spec)
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "advance", "RFC-0001", "spec"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_nonexistent_rfc() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(temp_dir.path(), &[&["rfc", "advance", "RFC-9999", "impl"]])?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_finalize_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_advance_sets_updated_field() -> common::TestResult {
    let temp_dir = init_project()?;
    let date = today();

    let output = run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Updated RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "advance", "RFC-0001", "impl"],
            &["rfc", "get", "RFC-0001", "updated"],
        ],
    )?;
    assert_lifecycle_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
