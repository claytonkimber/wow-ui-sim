use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

const LOCK_FILE_NAME: &str = "wow-ui-sim-test-workloads.lock";

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Shared,
    Exclusive,
}

#[cfg(target_os = "linux")]
pub(crate) struct Permit {
    _local: LocalPermit,
    _file: File,
}

#[cfg(target_os = "linux")]
enum LocalPermit {
    Shared {
        _guard: RwLockReadGuard<'static, ()>,
    },
    Exclusive {
        _guard: RwLockWriteGuard<'static, ()>,
    },
}

#[cfg(not(target_os = "linux"))]
pub(crate) struct Permit;

pub(crate) fn acquire(mode: Mode) -> io::Result<Permit> {
    acquire_at(&default_lock_path(), mode)
}

#[cfg(target_os = "linux")]
pub(crate) fn acquire_at(path: &Path, mode: Mode) -> io::Result<Permit> {
    let local = acquire_local(mode);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    lock(&file, mode)?;
    Ok(Permit {
        _local: local,
        _file: file,
    })
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn acquire_at(_path: &Path, _mode: Mode) -> io::Result<Permit> {
    Ok(Permit)
}

pub(crate) fn with_lock<T>(mode: Mode, body: impl FnOnce() -> T) -> T {
    with_lock_at(&default_lock_path(), mode, body)
}

pub(crate) fn with_lock_at<T>(path: &Path, mode: Mode, body: impl FnOnce() -> T) -> T {
    let _permit = acquire_at(path, mode)
        .unwrap_or_else(|error| panic!("acquire {} workload gate: {error}", mode.description()));
    body()
}

fn default_lock_path() -> PathBuf {
    std::env::temp_dir().join(LOCK_FILE_NAME)
}

#[cfg(target_os = "linux")]
fn acquire_local(mode: Mode) -> LocalPermit {
    static LOCAL_GATE: OnceLock<RwLock<()>> = OnceLock::new();
    let gate = LOCAL_GATE.get_or_init(|| RwLock::new(()));
    match mode {
        Mode::Shared => LocalPermit::Shared {
            _guard: gate.read().unwrap_or_else(|poisoned| poisoned.into_inner()),
        },
        Mode::Exclusive => LocalPermit::Exclusive {
            _guard: gate
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        },
    }
}

impl Mode {
    #[cfg(target_os = "linux")]
    fn operation(self) -> libc::c_int {
        match self {
            Self::Shared => libc::LOCK_SH,
            Self::Exclusive => libc::LOCK_EX,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Exclusive => "exclusive",
        }
    }
}

#[cfg(target_os = "linux")]
fn lock(file: &File, mode: Mode) -> io::Result<()> {
    loop {
        let result = unsafe { libc::flock(file.as_raw_fd(), mode.operation()) };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}
