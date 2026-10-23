// SPDX-License-Identifier: GPL-3.0-or-later
//! Small file-system helpers shared by the library and the settings store.

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

/// Writes `bytes` to `path` atomically.
///
/// The data goes to a temporary file in the same folder, is flushed to disk
/// and then renamed over the destination. Readers therefore see either the old
/// or the new content, never a half-written file. Missing parent folders are
/// created.
///
/// # Errors
///
/// Fails if the folder cannot be created or the file cannot be written.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".typlet-")
        .suffix(".tmp")
        .tempfile_in(dir)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("file.json");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");

        // No temporary files are left behind.
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }
}
