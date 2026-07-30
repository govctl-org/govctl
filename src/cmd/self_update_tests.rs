use super::*;

fn render_bin_path_in_archive(version: &str, target: &str, bin: &str) -> String {
    SELF_UPDATE_BIN_PATH_IN_ARCHIVE
        .replace("{{ version }}", version)
        .replace("{{ target }}", target)
        .replace("{{ bin }}", bin)
}

#[test]
fn test_same_version_is_up_to_date() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(compare_versions("0.8.3", "0.8.3")?, VersionCheck::UpToDate);
    Ok(())
}

#[test]
fn test_newer_available() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        compare_versions("0.8.2", "0.8.3")?,
        VersionCheck::UpdateAvailable {
            current: "0.8.2".into(),
            latest: "0.8.3".into(),
        }
    );
    Ok(())
}

#[test]
fn test_current_newer_than_latest_is_up_to_date() -> Result<(), Box<dyn std::error::Error>> {
    // Dev build ahead of latest release
    assert_eq!(compare_versions("0.9.0", "0.8.3")?, VersionCheck::UpToDate);
    Ok(())
}

#[test]
fn test_strips_v_prefix() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(compare_versions("0.8.3", "v0.8.3")?, VersionCheck::UpToDate);
    assert_eq!(
        compare_versions("0.8.2", "v0.9.0")?,
        VersionCheck::UpdateAvailable {
            current: "0.8.2".into(),
            latest: "0.9.0".into(),
        }
    );
    Ok(())
}

#[test]
fn test_major_version_update() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        compare_versions("0.8.3", "1.0.0")?,
        VersionCheck::UpdateAvailable {
            current: "0.8.3".into(),
            latest: "1.0.0".into(),
        }
    );
    Ok(())
}

#[test]
fn test_prerelease_not_newer_than_release() -> Result<(), Box<dyn std::error::Error>> {
    // 1.0.0-alpha < 1.0.0 per semver, so if current is 1.0.0 and latest is 1.0.0-alpha
    assert_eq!(
        compare_versions("1.0.0", "1.0.0-alpha")?,
        VersionCheck::UpToDate
    );
    Ok(())
}

#[test]
fn test_invalid_current_version_errors() {
    assert!(compare_versions("not-a-version", "0.8.3").is_err());
}

#[test]
fn test_invalid_latest_version_errors() {
    assert!(compare_versions("0.8.3", "not-a-version").is_err());
}

#[test]
fn test_usable_github_token_rejects_empty_values() {
    assert_eq!(usable_github_token(None), None);
    assert_eq!(usable_github_token(Some(String::new())), None);
    assert_eq!(usable_github_token(Some(" \t".to_string())), None);
    assert_eq!(
        usable_github_token(Some("token".to_string())),
        Some("token".to_string())
    );
}

#[test]
fn test_unix_archive_bin_path_matches_release_layout() {
    assert_eq!(
        render_bin_path_in_archive("0.8.4", "aarch64-apple-darwin", "govctl"),
        "govctl-v0.8.4-aarch64-apple-darwin/govctl"
    );
}

#[test]
fn test_windows_archive_bin_path_matches_release_layout() {
    assert_eq!(
        render_bin_path_in_archive("0.8.4", "x86_64-pc-windows-msvc", "govctl.exe"),
        "govctl-v0.8.4-x86_64-pc-windows-msvc/govctl.exe"
    );
}

#[test]
fn test_release_metadata_uses_matching_archive_layout() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        SELF_UPDATE_BIN_PATH_IN_ARCHIVE,
        "govctl-v{{ version }}-{{ target }}/{{ bin }}"
    );

    let manifest_path = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Value = toml::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let binstall = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("binstall"))
        .ok_or("missing package.metadata.binstall")?;
    let pkg_url = binstall
        .get("pkg-url")
        .and_then(toml::Value::as_str)
        .ok_or("missing package.metadata.binstall.pkg-url")?;
    assert_eq!(
        pkg_url,
        "{ repo }/releases/download/v{ version }/govctl-v{ version }-{ target }.tar.gz"
    );
    let pkg_fmt = binstall
        .get("pkg-fmt")
        .and_then(toml::Value::as_str)
        .ok_or("missing package.metadata.binstall.pkg-fmt")?;
    assert_eq!(pkg_fmt, "tgz");
    let bin_dir = binstall
        .get("bin-dir")
        .and_then(toml::Value::as_str)
        .ok_or("missing package.metadata.binstall.bin-dir")?;
    assert_eq!(
        bin_dir,
        "govctl-v{ version }-{ target }/{ bin }{ binary-ext }"
    );
    let overrides = binstall
        .get("overrides")
        .and_then(toml::Value::as_table)
        .ok_or("missing package.metadata.binstall.overrides")?;
    for (target, distribution_target) in [
        ("x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu"),
        ("aarch64-pc-windows-msvc", "aarch64-pc-windows-gnullvm"),
        ("x86_64-pc-windows-gnu", "x86_64-pc-windows-gnu"),
        ("aarch64-pc-windows-gnullvm", "aarch64-pc-windows-gnullvm"),
    ] {
        let windows = overrides
            .get(target)
            .ok_or_else(|| format!("missing cargo-binstall override for {target}"))?;
        let pkg_url = windows
            .get("pkg-url")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing cargo-binstall pkg-url for {target}"))?;
        let expected_pkg_url = if target == distribution_target {
            "{ repo }/releases/download/v{ version }/govctl-v{ version }-{ target }.zip".to_string()
        } else {
            format!(
                "{{ repo }}/releases/download/v{{ version }}/govctl-v{{ version }}-{distribution_target}.zip"
            )
        };
        assert_eq!(pkg_url, expected_pkg_url);
        let expected_bin_dir = if target == distribution_target {
            bin_dir.to_string()
        } else {
            format!("govctl-v{{ version }}-{distribution_target}/{{ bin }}{{ binary-ext }}")
        };
        assert_eq!(
            windows
                .get("bin-dir")
                .and_then(toml::Value::as_str)
                .unwrap_or(bin_dir),
            expected_bin_dir
        );
        let pkg_fmt = windows
            .get("pkg-fmt")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing cargo-binstall pkg-fmt for {target}"))?;
        assert_eq!(pkg_fmt, "zip");
    }

    Ok(())
}
