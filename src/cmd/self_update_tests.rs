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
    let windows_pkg_url = binstall
        .get("overrides")
        .and_then(|overrides| overrides.get("x86_64-pc-windows-msvc"))
        .and_then(|windows| windows.get("pkg-url"))
        .and_then(toml::Value::as_str)
        .ok_or("missing package.metadata.binstall.overrides.x86_64-pc-windows-msvc.pkg-url")?;
    assert_eq!(
        windows_pkg_url,
        "{ repo }/releases/download/v{ version }/govctl-v{ version }-{ target }.zip"
    );
    let windows_pkg_fmt = binstall
        .get("overrides")
        .and_then(|overrides| overrides.get("x86_64-pc-windows-msvc"))
        .and_then(|windows| windows.get("pkg-fmt"))
        .and_then(toml::Value::as_str)
        .ok_or("missing package.metadata.binstall.overrides.x86_64-pc-windows-msvc.pkg-fmt")?;
    assert_eq!(windows_pkg_fmt, "zip");

    let release_workflow = std::fs::read_to_string(format!(
        "{}/.github/workflows/release.yml",
        env!("CARGO_MANIFEST_DIR")
    ))?;
    assert!(
        release_workflow.contains(r#"ARCHIVE_NAME="govctl-${VERSION}-${{ matrix.target }}""#),
        "Unix release archive directory must match self-update and cargo-binstall layout"
    );
    assert!(
        release_workflow.contains(r#"$ARCHIVE_NAME = "govctl-${VERSION}-${{ matrix.target }}""#),
        "Windows release archive directory must match self-update and cargo-binstall layout"
    );

    Ok(())
}
