// SPDX-License-Identifier: GPL-3.0-or-later
//! Ensures only one background program runs per user.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

/// Holds the single-instance lock until dropped.
#[derive(Debug)]
pub struct InstanceGuard {
    _file: File,
}

/// Tries to become the only running instance.
///
/// Returns `Ok(None)` if another instance already holds the lock.
///
/// # Errors
///
/// Fails if the lock file cannot be created.
pub fn acquire(lock_file: &Path) -> io::Result<Option<InstanceGuard>> {
    if let Some(dir) = lock_file.parent() {
        fs::create_dir_all(dir)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_file)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(InstanceGuard { _file: file })),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(e)) => Err(e),
    }
}

/// Returns `true` if a background program currently holds the lock.
#[must_use]
pub fn is_running(lock_file: &Path) -> bool {
    let Ok(file) = OpenOptions::new().write(true).open(lock_file) else {
        return false;
    };
    matches!(file.try_lock(), Err(fs::TryLockError::WouldBlock))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_fails_until_first_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("typlet.lock");

        let first = acquire(&path).unwrap();
        assert!(first.is_some());
        assert!(is_running(&path));
        assert!(acquire(&path).unwrap().is_none());

        drop(first);
        assert!(!is_running(&path));
        assert!(acquire(&path).unwrap().is_some());
    }
}
