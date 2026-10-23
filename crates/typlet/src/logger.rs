// SPDX-License-Identifier: GPL-3.0-or-later
//! A small file logger.
//!
//! The log helps with support requests. It must never contain typed text:
//! the engine only logs what happened (for example "inserting failed"), never
//! what was typed or inserted.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{Level, LevelFilter, Log, Metadata, Record};

/// The log is rotated when it grows beyond this size.
const MAX_BYTES: u64 = 1024 * 1024;

struct FileLogger {
    path: PathBuf,
    file: Mutex<Option<File>>,
    level: LevelFilter,
}

impl FileLogger {
    fn open(path: &Path) -> Option<File> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).ok()?;
        }
        if fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
            let _ = fs::rename(path, path.with_extension("old.log"));
        }
        OpenOptions::new().create(true).append(true).open(path).ok()
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
            // Third-party crates only log warnings and errors.
            && (metadata.target().starts_with("typlet") || metadata.level() <= Level::Warn)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:<5} {}: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            record.args()
        );
        if let Ok(mut file) = self.file.lock() {
            if file.is_none() {
                *file = Self::open(&self.path);
            }
            if let Some(f) = file.as_mut() {
                let _ = f.write_all(line.as_bytes());
            }
        }
        #[cfg(debug_assertions)]
        {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock()
            && let Some(f) = file.as_mut()
        {
            let _ = f.flush();
        }
    }
}

/// Installs the logger. `TYPLET_LOG=debug` enables more detail.
pub(crate) fn init(path: &Path) {
    let level = std::env::var("TYPLET_LOG")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(LevelFilter::Info);
    let logger = FileLogger {
        path: path.to_path_buf(),
        file: Mutex::new(FileLogger::open(path)),
        level,
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(level);
    }
}
