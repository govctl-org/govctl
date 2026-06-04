use super::*;

#[test]
fn resolves_case_insensitive_substring_match() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["first item", "Second Item", "third"];
    let opts = MatchOptions {
        pattern: Some("second"),
        ..Default::default()
    };

    let indices = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::TickSingle,
    )?;

    assert_eq!(indices, vec![1]);
    Ok(())
}

#[test]
fn strips_known_category_prefixes_before_matching() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["some text", "other text"];
    let opts = MatchOptions {
        pattern: Some("fixed: some text"),
        ..Default::default()
    };

    let indices = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::TickSingle,
    )?;

    assert_eq!(indices, vec![0]);
    Ok(())
}

#[test]
fn does_not_strip_non_rendered_category_aliases() {
    let items = ["some text"];
    let opts = MatchOptions {
        pattern: Some("fix: some text"),
        ..Default::default()
    };

    let result = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::Remove,
    );

    assert!(
        result.is_err(),
        "fix: is a parser alias, not a rendered prefix"
    );
}

#[test]
fn allows_remove_all_for_multiple_matches() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["apple", "apricot", "banana"];
    let opts = MatchOptions {
        pattern: Some("ap"),
        all: true,
        ..Default::default()
    };

    let indices = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::Remove,
    )?;

    assert_eq!(indices, vec![0, 1]);
    Ok(())
}

#[test]
fn resolves_negative_indices_from_end() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["first", "second", "third"];
    let opts = MatchOptions {
        at: Some(-1),
        ..Default::default()
    };

    let indices = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::TickSingle,
    )?;

    assert_eq!(indices, vec![2]);
    Ok(())
}

#[test]
fn rejects_ambiguous_tick_matches_without_all() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["test-one", "test-two", "other"];
    let opts = MatchOptions {
        pattern: Some("test"),
        ..Default::default()
    };

    let Err(err) = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::TickSingle,
    ) else {
        return Err("ambiguous tick match should fail".into());
    };

    assert!(err.to_string().contains("2 items match"));
    Ok(())
}

#[test]
fn rejects_invalid_regex_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["a", "b"];
    let opts = MatchOptions {
        pattern: Some("[invalid"),
        regex: true,
        ..Default::default()
    };

    let Err(err) = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::TickSingle,
    ) else {
        return Err("invalid regex should fail".into());
    };

    assert!(err.to_string().contains("Invalid regex"));
    Ok(())
}

#[test]
fn rejects_no_match_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let items = ["apple", "banana", "cherry"];
    let opts = MatchOptions {
        pattern: Some("xyz"),
        ..Default::default()
    };

    let Err(err) = resolve_match_indices(
        "WI-1",
        "acceptance_criteria",
        &items,
        &opts,
        MatchUse::Remove,
    ) else {
        return Err("missing match should fail".into());
    };

    assert!(err.to_string().contains("No items match"));
    Ok(())
}
