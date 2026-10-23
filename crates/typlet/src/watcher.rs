// SPDX-License-Identifier: GPL-3.0-or-later
//! Watches the library folder, the settings file and the quit request.
//!
//! The editor saves snippets and settings as files; this watcher notices the
//! change, waits until writing has settled and reports it. There is no other
//! connection between the editor and the background program.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// Changes are reported after this much quiet time, so a burst of file
/// operations (an editor saving several snippets) causes one reload.
const SETTLE: Duration = Duration::from_millis(300);

/// What changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// Something below the library folder.
    Library,
    /// The settings file.
    Settings,
    /// The editor asks the background program to quit (after installing an
    /// update on Linux, so the new version can start).
    Quit,
}

enum Message {
    Fs(notify::Result<notify::Event>),
    WatchLibrary(PathBuf),
}

/// Handle to the watcher thread.
pub(crate) struct FileWatcher {
    control: Sender<Message>,
}

impl FileWatcher {
    /// Starts watching. `on_change` runs on the watcher thread.
    pub(crate) fn start(
        library: &Path,
        settings_file: &Path,
        quit_request: &Path,
        on_change: impl Fn(Change) + Send + 'static,
    ) -> notify::Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let fs_tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = fs_tx.send(Message::Fs(event));
        })?;
        watch_library(&mut watcher, None, library);
        let config_dir = settings_file.parent().unwrap_or(settings_file);
        let _ = std::fs::create_dir_all(config_dir);
        watcher.watch(config_dir, RecursiveMode::NonRecursive)?;

        let state = State {
            watcher,
            library: library.to_path_buf(),
            settings_file: settings_file.to_path_buf(),
            quit_request: quit_request.to_path_buf(),
        };
        thread::Builder::new()
            .name("typlet-watcher".into())
            .spawn(move || state.run(&rx, &on_change))?;
        Ok(Self { control: tx })
    }

    /// Switches to another library folder.
    pub(crate) fn watch_library(&self, library: &Path) {
        let _ = self
            .control
            .send(Message::WatchLibrary(library.to_path_buf()));
    }
}

struct State {
    watcher: RecommendedWatcher,
    library: PathBuf,
    settings_file: PathBuf,
    quit_request: PathBuf,
}

impl State {
    fn run(mut self, rx: &Receiver<Message>, on_change: &impl Fn(Change)) {
        let mut library_due: Option<Instant> = None;
        let mut settings_due: Option<Instant> = None;
        loop {
            let deadline = [library_due, settings_due].into_iter().flatten().min();
            let message = match deadline {
                Some(deadline) => {
                    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                }
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match message {
                Ok(Message::Fs(Ok(event))) if !is_change(&event) => {}
                Ok(Message::Fs(Ok(event))) => {
                    let due = Instant::now() + SETTLE;
                    for path in &event.paths {
                        if path.starts_with(&self.library) {
                            library_due = Some(due);
                        } else if *path == self.settings_file {
                            settings_due = Some(due);
                        } else if *path == self.quit_request && path.exists() {
                            on_change(Change::Quit);
                        }
                    }
                }
                Ok(Message::Fs(Err(error))) => log::warn!("file watcher: {error}"),
                Ok(Message::WatchLibrary(path)) => {
                    let old = std::mem::replace(&mut self.library, path);
                    watch_library(&mut self.watcher, Some(&old), &self.library);
                    library_due = Some(Instant::now());
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            let now = Instant::now();
            if library_due.is_some_and(|due| due <= now) {
                library_due = None;
                on_change(Change::Library);
            }
            if settings_due.is_some_and(|due| due <= now) {
                settings_due = None;
                on_change(Change::Settings);
            }
        }
    }
}

/// Only real changes count. Reading files (including Typlet's own reloads)
/// produces access events; reacting to them would reload forever.
fn is_change(event: &notify::Event) -> bool {
    !event.kind.is_access() && !event.kind.is_other()
}

fn watch_library(watcher: &mut RecommendedWatcher, old: Option<&Path>, new: &Path) {
    if let Some(old) = old {
        let _ = watcher.unwatch(old);
    }
    let _ = std::fs::create_dir_all(new);
    if let Err(error) = watcher.watch(new, RecursiveMode::Recursive) {
        log::warn!("cannot watch the library {}: {error}", new.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_library_settings_and_quit_requests() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("snippets");
        let settings = dir.path().join("config").join("settings.json");
        let quit = dir.path().join("config").join("quit-request");
        let (tx, rx) = crossbeam_channel::unbounded();
        let _watcher = FileWatcher::start(&library, &settings, &quit, move |change| {
            let _ = tx.send(change);
        })
        .unwrap();

        std::fs::write(library.join("a.json"), "{}").unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(Change::Library));

        std::fs::write(&settings, "{}").unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Ok(Change::Settings)
        );

        std::fs::write(&quit, "update").unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(Change::Quit));
        // Writing may report several events; removing the request must not
        // count as another one.
        thread::sleep(Duration::from_millis(300));
        while rx.try_recv().is_ok() {}
        std::fs::remove_file(&quit).unwrap();
        assert_ne!(
            rx.recv_timeout(Duration::from_millis(500)),
            Ok(Change::Quit)
        );
    }

    #[test]
    fn reading_files_is_not_a_change() {
        // Regression: reloading read the files, which reported access
        // events, which triggered the next reload, forever.
        use notify::event::{AccessKind, AccessMode, CreateKind, EventKind};
        let event = notify::Event::new;
        assert!(!is_change(&event(EventKind::Access(AccessKind::Open(
            AccessMode::Read
        )))));
        assert!(!is_change(&event(EventKind::Access(AccessKind::Close(
            AccessMode::Read
        )))));
        assert!(is_change(&event(EventKind::Create(CreateKind::File))));
        assert!(is_change(&event(EventKind::Any)));
    }
}
