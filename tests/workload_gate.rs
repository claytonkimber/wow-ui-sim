#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const FIXTURE_MODE_ENV: &str = "WOW_SIM_WORKLOAD_GATE_FIXTURE";
const LOCK_PATH_ENV: &str = "WOW_SIM_WORKLOAD_GATE_LOCK";
const READY_PATH_ENV: &str = "WOW_SIM_WORKLOAD_GATE_READY";
const RELEASE_PATH_ENV: &str = "WOW_SIM_WORKLOAD_GATE_RELEASE";
const TIMEOUT_REEXEC_TEST_ENV: &str = "WOW_SIM_TIMEOUT_REEXEC_TEST";
const MARKER_WAIT: Duration = Duration::from_secs(5);
const EXCLUSION_PROBE: Duration = Duration::from_millis(150);

#[test]
fn shared_readers_overlap_and_exclusive_excludes() {
    let temp = TempDir::new().expect("create workload gate tempdir");
    let mut first_reader = spawn_holder(temp.path(), "first-reader", "shared");
    wait_until_ready(temp.path(), "first-reader");

    let mut second_reader = spawn_holder(temp.path(), "second-reader", "shared");
    wait_until_ready(temp.path(), "second-reader");

    let mut writer = spawn_holder(temp.path(), "writer", "exclusive");
    assert_still_waiting(temp.path(), "writer");

    release_holder(temp.path(), "first-reader");
    release_holder(temp.path(), "second-reader");
    wait_for_success(&mut first_reader, "first shared reader");
    wait_for_success(&mut second_reader, "second shared reader");
    wait_until_ready(temp.path(), "writer");

    let mut blocked_reader = spawn_holder(temp.path(), "blocked-reader", "shared");
    assert_still_waiting(temp.path(), "blocked-reader");

    release_holder(temp.path(), "writer");
    wait_for_success(&mut writer, "exclusive writer");
    wait_until_ready(temp.path(), "blocked-reader");
    release_holder(temp.path(), "blocked-reader");
    wait_for_success(&mut blocked_reader, "reader blocked by writer");
}

#[test]
fn same_process_exclusive_waits_for_existing_exclusive() {
    let temp = TempDir::new().expect("create same-process workload tempdir");
    let lock_path = temp.path().join("gate.lock");
    let first = crate::common::workload_gate::acquire_at(
        &lock_path,
        crate::common::workload_gate::Mode::Exclusive,
    )
    .expect("acquire first exclusive workload permit");
    let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
    let second_lock_path = lock_path.clone();
    let second = thread::spawn(move || {
        let _permit = crate::common::workload_gate::acquire_at(
            &second_lock_path,
            crate::common::workload_gate::Mode::Exclusive,
        )
        .expect("acquire second exclusive workload permit");
        acquired_tx
            .send(())
            .expect("report second exclusive acquisition");
    });

    assert!(
        acquired_rx.recv_timeout(EXCLUSION_PROBE).is_err(),
        "second same-process writer acquired while the first writer was active"
    );
    drop(first);
    acquired_rx
        .recv_timeout(MARKER_WAIT)
        .expect("second same-process writer should acquire after release");
    second.join().expect("join second same-process writer");
}

#[test]
fn gate_releases_after_error_panic_and_process_exit() {
    let temp = TempDir::new().expect("create release-check workload tempdir");
    let lock_path = temp.path().join("gate.lock");
    let result: Result<(), &str> = crate::common::workload_gate::with_lock_at(
        &lock_path,
        crate::common::workload_gate::Mode::Shared,
        || Err("deliberate workload failure"),
    );
    assert_eq!(result, Err("deliberate workload failure"));
    assert_exclusive_can_acquire(temp.path(), "after-error");

    let panic = std::panic::catch_unwind(|| {
        crate::common::workload_gate::with_lock_at(
            &lock_path,
            crate::common::workload_gate::Mode::Exclusive,
            || panic!("deliberate workload panic"),
        );
    });
    assert!(panic.is_err(), "workload fixture must panic");
    assert_exclusive_can_acquire(temp.path(), "after-panic");

    let temp = TempDir::new().expect("create process-exit workload tempdir");
    let mut exiting_writer = spawn_holder(temp.path(), "exiting-writer", "exclusive-exit");
    wait_until_ready(temp.path(), "exiting-writer");
    wait_for_success(&mut exiting_writer, "exclusive process-exit fixture");

    let mut reader = spawn_holder(temp.path(), "reader-after-exit", "shared");
    wait_until_ready(temp.path(), "reader-after-exit");
    release_holder(temp.path(), "reader-after-exit");
    wait_for_success(&mut reader, "reader after process exit");
}

#[test]
fn exclusive_wait_happens_before_timeout_starts() {
    let temp = TempDir::new().expect("create timeout-boundary workload tempdir");
    let lock_path = temp.path().join("gate.lock");
    if std::env::var_os(TIMEOUT_REEXEC_TEST_ENV).is_some() {
        crate::common::with_performance_timeout_at(1, &lock_path, || {});
        return;
    }

    let mut reader = spawn_holder(temp.path(), "timeout-reader", "shared-auto-release");
    wait_until_ready(temp.path(), "timeout-reader");

    let started = Instant::now();
    crate::common::with_performance_timeout_at(1, &lock_path, || {});
    let elapsed = started.elapsed();

    wait_for_success(&mut reader, "timeout-boundary reader");
    assert!(
        elapsed >= Duration::from_millis(1_200),
        "exclusive gate did not wait for the shared workload: {elapsed:?}"
    );
}

#[test]
fn process_fixture() {
    let Some(mode) = std::env::var_os(FIXTURE_MODE_ENV) else {
        return;
    };
    let lock_path = fixture_path(LOCK_PATH_ENV);
    let ready_path = fixture_path(READY_PATH_ENV);
    let release_path = fixture_path(RELEASE_PATH_ENV);

    match mode.to_string_lossy().as_ref() {
        "shared" => with_fixture_lock(
            &lock_path,
            crate::common::workload_gate::Mode::Shared,
            || hold_until_released(&ready_path, &release_path),
        ),
        "exclusive" => with_fixture_lock(
            &lock_path,
            crate::common::workload_gate::Mode::Exclusive,
            || hold_until_released(&ready_path, &release_path),
        ),
        "shared-auto-release" => with_fixture_lock(
            &lock_path,
            crate::common::workload_gate::Mode::Shared,
            || {
                std::fs::write(&ready_path, "ready").expect("write auto-release ready marker");
                thread::sleep(Duration::from_millis(1_300));
            },
        ),
        "exclusive-exit" => with_fixture_lock(
            &lock_path,
            crate::common::workload_gate::Mode::Exclusive,
            || {
                std::fs::write(&ready_path, "ready").expect("write process-exit ready marker");
                unsafe { libc::_exit(0) }
            },
        ),
        unknown => panic!("unknown workload fixture mode: {unknown}"),
    }
}

fn with_fixture_lock<T>(
    path: &Path,
    mode: crate::common::workload_gate::Mode,
    body: impl FnOnce() -> T,
) -> T {
    crate::common::workload_gate::with_lock_at(path, mode, body)
}

fn assert_exclusive_can_acquire(directory: &Path, name: &str) {
    let mut writer = spawn_holder(directory, name, "exclusive");
    wait_until_ready(directory, name);
    release_holder(directory, name);
    wait_for_success(&mut writer, name);
}

fn spawn_holder(directory: &Path, name: &str, mode: &str) -> Child {
    let ready_path = marker_path(directory, name, "ready");
    let release_path = marker_path(directory, name, "release");
    let mut command =
        Command::new(std::env::current_exe().expect("resolve integration test binary"));
    command
        .args([
            "workload_gate::process_fixture",
            "--exact",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(FIXTURE_MODE_ENV, mode)
        .env(LOCK_PATH_ENV, directory.join("gate.lock"))
        .env(READY_PATH_ENV, ready_path)
        .env(RELEASE_PATH_ENV, release_path);
    command.spawn().expect("spawn workload gate fixture")
}

fn hold_until_released(ready_path: &Path, release_path: &Path) {
    std::fs::write(ready_path, "ready").expect("write workload ready marker");
    wait_for_marker(release_path, MARKER_WAIT);
}

fn wait_until_ready(directory: &Path, name: &str) {
    wait_for_marker(&marker_path(directory, name, "ready"), MARKER_WAIT);
}

fn assert_still_waiting(directory: &Path, name: &str) {
    let ready_path = marker_path(directory, name, "ready");
    thread::sleep(EXCLUSION_PROBE);
    assert!(
        !ready_path.exists(),
        "{name} acquired a conflicting workload lock"
    );
}

fn release_holder(directory: &Path, name: &str) {
    std::fs::write(marker_path(directory, name, "release"), "release")
        .expect("write workload release marker");
}

fn wait_for_marker(path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for marker {}", path.display());
}

fn wait_for_success(child: &mut Child, label: &str) {
    let status = child
        .wait()
        .unwrap_or_else(|error| panic!("wait for {label}: {error}"));
    assert!(status.success(), "{label} failed with {status}");
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("missing {name}")))
}

fn marker_path(directory: &Path, name: &str, suffix: &str) -> PathBuf {
    directory.join(format!("{name}-{suffix}"))
}
