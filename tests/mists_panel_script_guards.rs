use std::path::PathBuf;

#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, process::Command};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path)).expect("failed to read repository file")
}

fn cargo_profile<'a>(manifest: &'a str, profile: &str) -> &'a str {
    let header = format!("[profile.{profile}]");
    let section_start = manifest
        .find(&header)
        .unwrap_or_else(|| panic!("missing {header}"))
        + header.len();
    let section = &manifest[section_start..];

    section
        .split_once("\n[")
        .map_or(section, |(profile, _)| profile)
}

#[cfg(unix)]
fn write_executable(path: &std::path::Path, contents: &str) {
    std::fs::write(path, contents).expect("failed to write fake executable");
    let mut permissions = std::fs::metadata(path)
        .expect("failed to read fake executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("failed to make fake executable runnable");
}

#[cfg(unix)]
fn path_with_prepend(directory: &std::path::Path) -> std::ffi::OsString {
    let current_path = std::env::var_os("PATH").expect("PATH should be set");
    let paths = std::iter::once(directory.to_path_buf()).chain(std::env::split_paths(&current_path));

    std::env::join_paths(paths).expect("failed to prepend fake executable directory to PATH")
}

#[test]
fn mists_panel_scripts_inherit_incremental_dev_profile() {
    let manifest = read_repo_file("Cargo.toml");
    let dev_profile = cargo_profile(&manifest, "dev");
    let release_profile = cargo_profile(&manifest, "release");

    assert!(
        dev_profile
            .lines()
            .any(|line| line.trim() == "incremental = true"),
        "development profile should enable incremental compilation"
    );
    assert!(
        !release_profile
            .lines()
            .any(|line| line.trim_start().starts_with("incremental =")),
        "release profile should preserve Cargo's incremental default"
    );

    for script_path in [
        "scripts/mists-panel-parity.sh",
        "scripts/test-mists-addon-panels.sh",
    ] {
        let script = read_repo_file(script_path);

        assert!(
            script.contains("MISTS_CARGO_TARGET_DIR"),
            "{script_path} should allow a Mists-specific Cargo target override"
        );
        assert!(
            script.contains("wow-ui-sim/cargo-targets/mists-panel-parity"),
            "{script_path} should default scripted Mists builds outside repo target/"
        );
        assert!(
            !script.contains("CARGO_INCREMENTAL"),
            "{script_path} should preserve Cargo profile and caller environment precedence"
        );
    }
}

#[cfg(unix)]
#[test]
fn mists_panel_runner_preserves_explicit_cargo_incremental_override() {
    const EXPLICIT_OVERRIDE: &str = "0";

    let temp_dir = tempfile::tempdir().expect("failed to create Mists runner test directory");
    let fake_bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir(&fake_bin_dir).expect("failed to create fake executable directory");

    let incremental_capture = temp_dir.path().join("cargo-incremental");
    write_executable(
        &fake_bin_dir.join("cargo"),
        "#!/bin/sh\nprintf '%s' \"$CARGO_INCREMENTAL\" > \"$CARGO_INCREMENTAL_CAPTURE\"\n",
    );

    let fake_visual_metrics = temp_dir.path().join("panel-visual-metrics");
    write_executable(&fake_visual_metrics, "#!/bin/sh\nexit 0\n");

    let output = Command::new(repo_root().join("scripts/mists-panel-parity.sh"))
        .current_dir(repo_root())
        .args(["--panel", "__incremental_override_guard_no_match__"])
        .arg("--out-dir")
        .arg(temp_dir.path().join("out"))
        .env("PATH", path_with_prepend(&fake_bin_dir))
        .env("CARGO_INCREMENTAL", EXPLICIT_OVERRIDE)
        .env("CARGO_INCREMENTAL_CAPTURE", &incremental_capture)
        .env("CARGO_TARGET_DIR", temp_dir.path().join("target"))
        .env("MISTS_PANEL_SIGNAL_ONLY", "1")
        .env("PANEL_VISUAL_METRICS_BIN", &fake_visual_metrics)
        .output()
        .expect("failed to run Mists panel runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Mists runner should finish after the unmatched panel filter\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("no panel matched filter"),
        "Mists runner should reach panel selection after invoking Cargo\nstderr:\n{stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(incremental_capture)
            .expect("fake Cargo should capture CARGO_INCREMENTAL"),
        EXPLICIT_OVERRIDE,
        "Mists runner should pass the caller's CARGO_INCREMENTAL value to Cargo unchanged"
    );
}

#[test]
fn installed_addon_panel_runner_separates_exit_and_interrupt_cleanup() {
    let script = read_repo_file("scripts/test-mists-addon-panels.sh");

    assert!(
        script.contains("cleanup_active_addon_on_exit"),
        "addon panel runner should have a quiet EXIT cleanup path"
    );
    assert!(
        script.contains("cleanup_active_addon_on_interrupt"),
        "addon panel runner should have an interrupt-specific cleanup path"
    );
    assert!(
        !script.contains("trap cleanup_active_addon EXIT"),
        "normal EXIT cleanup should not use the interrupt/error-reporting cleanup path"
    );
    assert!(
        !script.contains("trap 'cleanup_active_addon; exit 130' INT TERM"),
        "INT/TERM cleanup should use the interrupt-specific path"
    );
}

#[test]
fn mists_panel_runner_fails_on_texture_manager_load_errors() {
    let script = read_repo_file("scripts/mists-panel-parity.sh");

    assert!(
        script.contains("fail_if_runtime_log_error"),
        "panel runner should centralize runtime stderr/stdout failure checks"
    );
    assert!(
        script.contains("TexMgr") && script.contains("Load error"),
        "panel runner should fail when a panel emits texture-manager load errors"
    );
}
