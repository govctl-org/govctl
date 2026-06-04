use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_tests_do_not_use_include_macro_for_suite_splitting() -> TestResult {
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut offenders = Vec::new();

    for path in rust_files_under(&tests_dir)? {
        let content = fs::read_to_string(&path)?;
        let rel_path = path.strip_prefix(env!("CARGO_MANIFEST_DIR"))?;
        offenders.extend(macro_injection_locations(rel_path, &content)?);
    }

    assert!(
        offenders.is_empty(),
        "tests must use normal Rust modules instead of macro-based source injection:\n{}",
        offenders.join("\n")
    );
    Ok(())
}

#[test]
fn test_include_macro_detector_handles_multiline_spacing() -> TestResult {
    let sample = ["fn test() {", "include", "!", "(\"case.rs\");", "}"].join("\n");
    let offenders = macro_injection_locations(Path::new("tests/example.rs"), &sample)?;

    assert_eq!(offenders, vec!["tests/example.rs:2"]);
    Ok(())
}

fn macro_injection_locations(path: &Path, content: &str) -> TestResult<Vec<String>> {
    let pattern = Regex::new(&format!(r"{name}\s*!\s*\(", name = "include"))?;
    Ok(pattern
        .find_iter(content)
        .map(|matched| {
            format!(
                "{}:{}",
                path.display(),
                line_number(content, matched.start())
            )
        })
        .collect())
}

fn line_number(content: &str, byte_offset: usize) -> usize {
    content[..byte_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn rust_files_under(root: &Path) -> TestResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> TestResult {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}
