// SPDX-License-Identifier: GPL-3.0-or-later
//! The user-interface controller. Runs on the main thread, owns all windows
//! and the tray icon, and talks to the engine thread with commands.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use crossbeam_channel::Sender;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use typlet_core::snippet::validate_trigger;
use typlet_core::{
    DateLanguage, Document, FieldKind as CoreFieldKind, Hotkey, Language, Library, RenderContext,
    Settings, Snippet, SnippetId, ValidationError, nesting, render,
};
use typlet_platform::paths::Paths;
use typlet_platform::{Platform, autostart};

use crate::engine::{self, Attempt, AttemptResult, Command, Notice};
use crate::firstrun;
use crate::os;
use crate::report::{self, ReportInput};
use crate::snapshot::Snapshot;
use crate::ui::{
    AttemptItem, AttemptKind, CreateWindow, DiagnosticsWindow, ExampleItem, FieldKind, FormField,
    FormWindow, MessageKind, MessageWindow, SearchWindow, SetupWindow, SnippetItem, TrayIcon,
    TriggerProblem,
};
use crate::watcher::{Change, FileWatcher};

/// At most this many search results are shown.
const MAX_RESULTS: usize = 100;

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

/// Runs `f` with the app on the UI thread. Can be called from any thread.
fn post(f: impl FnOnce(&Rc<App>) + Send + 'static) {
    let result = slint::invoke_from_event_loop(move || {
        let app = APP.with(|cell| cell.borrow().clone());
        if let Some(app) = app {
            f(&app);
        }
    });
    if result.is_err() {
        log::debug!("event loop already stopped");
    }
}

/// Options from the command line.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartOptions {
    /// Open the set-up assistant.
    pub(crate) setup: bool,
    /// Open the quick search.
    pub(crate) search: bool,
}

/// State of the keyboard engine.
#[derive(Debug, Clone, PartialEq, Eq)]
enum EngineState {
    Starting,
    Running,
    Failed(String),
}

/// The open form.
struct FormState {
    id: SnippetId,
    names: Vec<String>,
}

struct Windows {
    tray: TrayIcon,
    search: SearchWindow,
    form: FormWindow,
    create: CreateWindow,
    setup: SetupWindow,
    diagnostics: DiagnosticsWindow,
    message: MessageWindow,
}

/// The controller.
pub(crate) struct App {
    paths: Paths,
    settings: RefCell<Arc<Settings>>,
    library: RefCell<Arc<Snapshot>>,
    engine: Sender<Command>,
    watcher: RefCell<Option<FileWatcher>>,
    windows: Windows,
    state: RefCell<EngineState>,
    search_hits: RefCell<Vec<usize>>,
    form: RefCell<Option<FormState>>,
    form_fields: Rc<VecModel<FormField>>,
    created: Cell<Option<SnippetId>>,
    message_kind: Cell<MessageKind>,
    diagnostics_wanted: Cell<bool>,
    settings_error_shown: Cell<bool>,
    quiet: Cell<bool>,
    test_expected: RefCell<String>,
    /// Shortcuts already reported as used by another program.
    taken_hotkeys: RefCell<HashSet<Hotkey>>,
}

/// Loads the settings; a broken file falls back to the defaults.
fn load_settings(paths: &Paths) -> (Settings, Option<String>) {
    match Settings::load(&paths.settings_file) {
        Ok(settings) => (settings, None),
        Err(error) => {
            log::warn!("{error}");
            (Settings::default(), Some(error.to_string()))
        }
    }
}

/// Whether the interface (and dates) should be German.
pub(crate) fn is_german(settings: &Settings) -> bool {
    match settings.language {
        Language::De => true,
        Language::En => false,
        Language::System => typlet_platform::system_language()
            .is_some_and(|lang| lang.to_lowercase().starts_with("de")),
    }
}

fn apply_language(settings: &Settings) {
    let language = if is_german(settings) { "de" } else { "en" };
    if let Err(error) = slint::select_bundled_translation(language) {
        log::warn!("cannot select language {language}: {error}");
    }
}

/// Keeps the autostart entry in line with the settings.
fn sync_autostart(enabled: bool) {
    if cfg!(debug_assertions) {
        // Development builds must not register target/debug/typlet.
        return;
    }
    if autostart::is_enabled() == enabled {
        return;
    }
    let result = std::env::current_exe()
        .map_err(typlet_platform::PlatformError::from)
        .and_then(|exe| autostart::set(enabled, &exe));
    if let Err(error) = result {
        log::warn!("cannot change autostart: {error}");
    }
}

/// The native handle of a shown window (an `HWND` on Windows).
fn native_handle(window: &slint::Window) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    let raw = handle.window_handle().ok()?.as_raw();
    match raw {
        RawWindowHandle::Win32(win) => Some(win.hwnd.get()),
        _ => None,
    }
}

/// Shows a window and brings it to the front.
fn present(window: &slint::Window, place: bool) {
    if let Err(error) = window.show() {
        log::warn!("cannot show window: {error}");
        return;
    }
    if let Some(handle) = native_handle(window) {
        if place {
            typlet_platform::place_window(handle);
        }
        typlet_platform::activate_window(handle);
    }
}

fn hide(window: &slint::Window) {
    if let Err(error) = window.hide() {
        log::warn!("cannot hide window: {error}");
    }
}

/// Shows "Typlet is already running" for a second start and returns when
/// the user closes it.
pub(crate) fn show_already_running() -> Result<(), slint::PlatformError> {
    let window = MessageWindow::new()?;
    let (settings, _) =
        Paths::resolve().map_or_else(|| (Settings::default(), None), |p| load_settings(&p));
    apply_language(&settings);
    window.set_kind(MessageKind::AlreadyRunning);
    window.on_dismiss(|| {
        let _ = slint::quit_event_loop();
    });
    window.window().on_close_requested(|| {
        let _ = slint::quit_event_loop();
        slint::CloseRequestResponse::HideWindow
    });
    present(window.window(), true);
    slint::run_event_loop()
}

/// Runs the background program until the user quits.
pub(crate) fn run(paths: Paths, options: StartOptions) -> Result<(), Box<dyn std::error::Error>> {
    let (settings, settings_error) = load_settings(&paths);
    let root = paths.library(&settings);
    let german = is_german(&settings);
    let first_run = !settings.first_run_complete;
    if first_run && firstrun::library_is_empty(&root) {
        match firstrun::create_examples(&root, german) {
            Ok(count) => log::info!("created {count} example snippets"),
            Err(error) => log::warn!("cannot create example snippets: {error}"),
        }
    }
    let library = Arc::new(Snapshot::load(&root));
    let settings = Arc::new(settings);
    sync_autostart(settings.start_with_system);

    let windows = Windows {
        tray: TrayIcon::new()?,
        search: SearchWindow::new()?,
        form: FormWindow::new()?,
        create: CreateWindow::new()?,
        setup: SetupWindow::new()?,
        diagnostics: DiagnosticsWindow::new()?,
        message: MessageWindow::new()?,
    };
    // Translations can only be selected once the UI exists.
    apply_language(&settings);

    let engine = engine::spawn(
        Arc::clone(&settings),
        Arc::clone(&library),
        paths.editor_focus_lock.clone(),
        |notice| post(move |app| app.on_notice(notice)),
    )?;

    let app = Rc::new(App {
        paths,
        settings: RefCell::new(settings),
        library: RefCell::new(library),
        engine,
        watcher: RefCell::new(None),
        windows,
        state: RefCell::new(EngineState::Starting),
        search_hits: RefCell::new(Vec::new()),
        form: RefCell::new(None),
        form_fields: Rc::new(VecModel::default()),
        created: Cell::new(None),
        message_kind: Cell::new(MessageKind::AlreadyRunning),
        diagnostics_wanted: Cell::new(false),
        settings_error_shown: Cell::new(false),
        quiet: Cell::new(false),
        test_expected: RefCell::new(String::new()),
        taken_hotkeys: RefCell::new(HashSet::new()),
    });
    APP.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&app)));

    let watcher = FileWatcher::start(&root, &app.paths.settings_file, |change| {
        post(move |app| match change {
            Change::Library => app.reload_library(),
            Change::Settings => app.reload_settings(),
        });
    });
    match watcher {
        Ok(watcher) => *app.watcher.borrow_mut() = Some(watcher),
        Err(error) => log::warn!("cannot watch for changes: {error}"),
    }

    app.wire();
    app.refresh_tray();
    app.windows.tray.show()?;

    if let Some(error) = settings_error {
        app.show_message(MessageKind::SettingsProblem, &error);
        app.settings_error_shown.set(true);
    } else if app.library.borrow().broken_files > 0 {
        app.show_message(MessageKind::LibraryProblem, "");
    }
    if first_run || options.setup {
        app.show_setup();
    } else if options.search {
        app.show_search();
    }

    slint::run_event_loop_until_quit()?;
    let _ = app.engine.send(Command::Quit);
    APP.with(|cell| cell.borrow_mut().take());
    Ok(())
}

impl App {
    fn german(&self) -> bool {
        is_german(&self.settings.borrow())
    }

    /// Tells the engine whether a Typlet window with text fields is open, so
    /// that triggers typed into it do not expand. The set-up assistant is
    /// left out on purpose: its test field must expand.
    fn update_quiet(&self) {
        let w = &self.windows;
        let quiet = w.search.window().is_visible()
            || w.form.window().is_visible()
            || w.create.window().is_visible();
        if self.quiet.replace(quiet) != quiet {
            self.send(Command::Quiet(quiet));
        }
    }

    fn show_window(&self, window: &slint::Window, place: bool) {
        present(window, place);
        self.update_quiet();
    }

    fn hide_window(&self, window: &slint::Window) {
        hide(window);
        self.update_quiet();
    }

    fn date_language(&self) -> DateLanguage {
        if self.german() {
            DateLanguage::German
        } else {
            DateLanguage::English
        }
    }

    fn send(&self, command: Command) {
        if self.engine.send(command).is_err() {
            log::warn!("the engine has stopped");
        }
    }

    // ----- wiring -------------------------------------------------------

    #[allow(clippy::too_many_lines)] // one block per window keeps it readable
    fn wire(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let with = move |f: fn(&Rc<App>)| {
            let weak = weak.clone();
            move || {
                if let Some(app) = weak.upgrade() {
                    f(&app);
                }
            }
        };

        let tray = &self.windows.tray;
        tray.on_search(with(|app| app.show_search()));
        tray.on_new_snippet(with(|app| app.show_create(String::new())));
        tray.on_open_editor(with(|app| app.open_editor(&[])));
        tray.on_open_folder(with(|app| os::open_folder(&app.library.borrow().root)));
        tray.on_toggle_pause(with(|app| {
            app.update_settings(|settings| settings.paused = !settings.paused);
        }));
        tray.on_toggle_autostart(with(|app| app.toggle_autostart()));
        tray.on_setup(with(|app| app.show_setup()));
        tray.on_diagnostics(with(|app| app.show_diagnostics()));
        tray.on_help(with(|app| os::open_url(os::help_url(app.german()))));
        tray.on_quit(|| {
            let _ = slint::quit_event_loop();
        });

        // Quick search.
        let search = &self.windows.search;
        let weak = Rc::downgrade(self);
        search.on_search(move |query| {
            if let Some(app) = weak.upgrade() {
                app.update_search(&query);
            }
        });
        let weak = Rc::downgrade(self);
        search.on_choose(move |index| {
            if let Some(app) = weak.upgrade() {
                app.choose(index);
            }
        });
        search.on_cancel(with(|app| {
            app.hide_window(app.windows.search.window());
            app.send(Command::RestoreFocus);
        }));
        search.on_open_editor(with(|app| {
            app.hide_window(app.windows.search.window());
            app.open_editor(&[]);
        }));
        let weak = Rc::downgrade(self);
        search.window().on_close_requested(move || {
            if let Some(app) = weak.upgrade() {
                app.hide_window(app.windows.search.window());
                app.send(Command::RestoreFocus);
            }
            slint::CloseRequestResponse::KeepWindowShown
        });

        // Forms.
        let form = &self.windows.form;
        form.set_fields(ModelRc::from(Rc::clone(&self.form_fields)));
        let fields = Rc::clone(&self.form_fields);
        form.on_field_edited(move |index, value| {
            let Ok(index) = usize::try_from(index) else {
                return;
            };
            if let Some(mut row) = fields.row_data(index) {
                row.value = value;
                fields.set_row_data(index, row);
            }
        });
        form.on_submit(with(|app| app.submit_form()));
        form.on_cancel(with(|app| app.cancel_form()));
        let weak = Rc::downgrade(self);
        form.window().on_close_requested(move || {
            if let Some(app) = weak.upgrade() {
                app.cancel_form();
                app.hide_window(app.windows.form.window());
            }
            slint::CloseRequestResponse::KeepWindowShown
        });

        // New snippet.
        let create = &self.windows.create;
        let weak = Rc::downgrade(self);
        create.on_trigger_edited(move |trigger| {
            if let Some(app) = weak.upgrade() {
                let problem = app.trigger_problem(&trigger);
                app.windows.create.set_problem(problem);
            }
        });
        create.on_save(with(|app| app.save_created()));
        create.on_open_editor(with(|app| {
            let id = app.created.get().map(|id| id.to_string());
            app.hide_window(app.windows.create.window());
            match id {
                Some(id) => app.open_editor(&["--open", &id]),
                None => app.open_editor(&[]),
            }
        }));
        create.on_dismiss(with(|app| {
            app.hide_window(app.windows.create.window());
            app.send(Command::RestoreFocus);
        }));
        let weak = Rc::downgrade(self);
        create.window().on_close_requested(move || {
            if let Some(app) = weak.upgrade() {
                app.hide_window(app.windows.create.window());
                app.send(Command::RestoreFocus);
            }
            slint::CloseRequestResponse::KeepWindowShown
        });

        // Set-up assistant.
        let setup = &self.windows.setup;
        setup.on_recheck(with(|app| {
            if matches!(*app.state.borrow(), EngineState::Failed(_)) {
                app.send(Command::Retry);
            }
            app.refresh_setup();
        }));
        setup.on_toggle_autostart(with(|app| app.toggle_autostart()));
        setup.on_open_editor(with(|app| app.open_editor(&[])));
        setup.on_finish(with(|app| {
            app.hide_window(app.windows.setup.window());
            app.update_settings(|settings| settings.first_run_complete = true);
        }));
        let weak = Rc::downgrade(self);
        setup.on_test_changed(move |text| {
            if let Some(app) = weak.upgrade() {
                let expected = app.test_expected.borrow();
                let passed = !expected.is_empty() && text.contains(expected.as_str());
                app.windows.setup.set_test_passed(passed);
            }
        });
        let weak = Rc::downgrade(self);
        setup.window().on_close_requested(move || {
            if let Some(app) = weak.upgrade() {
                app.update_settings(|settings| settings.first_run_complete = true);
            }
            slint::CloseRequestResponse::HideWindow
        });

        // Diagnostics.
        let diagnostics = &self.windows.diagnostics;
        diagnostics.on_refresh(with(|app| app.show_diagnostics()));
        diagnostics.on_open_folder(with(|app| os::open_folder(&app.paths.data_dir)));
        diagnostics.on_dismiss(with(|app| {
            app.hide_window(app.windows.diagnostics.window())
        }));

        // Messages.
        let message = &self.windows.message;
        message.on_dismiss(with(|app| app.hide_window(app.windows.message.window())));
        message.on_primary_action(with(|app| match app.message_kind.get() {
            MessageKind::InputFailed => app.show_setup(),
            MessageKind::InsertFailed | MessageKind::LibraryProblem => app.show_diagnostics(),
            MessageKind::HotkeyTaken => app.open_editor(&["--settings"]),
            _ => {}
        }));
    }

    // ----- engine notices -------------------------------------------------

    fn on_notice(self: &Rc<Self>, notice: Notice) {
        match notice {
            Notice::Started => {
                *self.state.borrow_mut() = EngineState::Running;
                self.refresh_setup();
            }
            Notice::InputFailed(error) => {
                let first = *self.state.borrow() == EngineState::Starting;
                *self.state.borrow_mut() = EngineState::Failed(error.clone());
                self.refresh_setup();
                // The set-up assistant explains the problem on first run.
                if first && !self.windows.setup.window().is_visible() {
                    self.show_message(MessageKind::InputFailed, &error);
                }
            }
            Notice::ShowSearch => self.show_search(),
            Notice::ShowForm(id) => self.show_form(id),
            Notice::Selection(Some(text)) => self.show_create(text),
            Notice::Selection(None) => {
                if self.settings.borrow().notifications {
                    self.show_message(MessageKind::NoSelection, "");
                }
            }
            Notice::InsertFailed(error) => {
                // Failures are always in diagnostics; the message is optional.
                if self.settings.borrow().notifications {
                    self.show_message(MessageKind::InsertFailed, &error);
                }
            }
            Notice::Diagnostics(history) => self.fill_diagnostics(&history),
            Notice::HotkeyTaken(hotkey) => {
                // Once per shortcut and session; the engine reports it again
                // whenever the snippets or settings are reloaded.
                if self.taken_hotkeys.borrow_mut().insert(hotkey) {
                    let label = self.shortcut_label(hotkey);
                    self.show_message(MessageKind::HotkeyTaken, &label);
                }
            }
        }
    }

    // ----- settings and library ------------------------------------------

    fn update_settings(&self, change: impl FnOnce(&mut Settings)) {
        let mut settings = Settings::clone(&self.settings.borrow());
        change(&mut settings);
        if settings == **self.settings.borrow() {
            return;
        }
        if let Err(error) = settings.save(&self.paths.settings_file) {
            log::warn!("cannot save settings: {error}");
        }
        self.apply_settings(settings);
    }

    fn apply_settings(&self, settings: Settings) {
        let old = Arc::clone(&self.settings.borrow());
        let settings = Arc::new(settings);
        *self.settings.borrow_mut() = Arc::clone(&settings);
        if old.language != settings.language {
            apply_language(&settings);
        }
        if old.start_with_system != settings.start_with_system {
            sync_autostart(settings.start_with_system);
        }
        if old.library_path != settings.library_path {
            let root = self.paths.library(&settings);
            if let Some(watcher) = self.watcher.borrow().as_ref() {
                watcher.watch_library(&root);
            }
            self.reload_library();
        }
        self.send(Command::Settings(settings));
        self.refresh_tray();
    }

    fn reload_settings(&self) {
        match Settings::load(&self.paths.settings_file) {
            Ok(settings) => {
                self.settings_error_shown.set(false);
                if settings != **self.settings.borrow() {
                    log::info!("settings changed");
                    self.apply_settings(settings);
                }
            }
            Err(error) => {
                log::warn!("{error}");
                if !self.settings_error_shown.replace(true) {
                    self.show_message(MessageKind::SettingsProblem, &error.to_string());
                }
            }
        }
    }

    fn reload_library(&self) {
        let root = self.paths.library(&self.settings.borrow());
        let snapshot = Arc::new(Snapshot::load(&root));
        log::info!(
            "library loaded: {} snippets, {} issues",
            snapshot.snippets.len(),
            snapshot.issues.len()
        );
        *self.library.borrow_mut() = Arc::clone(&snapshot);
        self.send(Command::Library(snapshot));
        if self.windows.search.window().is_visible() {
            let query = self.windows.search.get_query();
            self.update_search(&query);
        }
    }

    fn toggle_autostart(&self) {
        self.update_settings(|settings| settings.start_with_system = !settings.start_with_system);
        self.refresh_tray();
    }

    fn hotkey_label(&self) -> String {
        let search = self.settings.borrow().search_hotkey;
        search.map_or_else(String::new, |hotkey| self.shortcut_label(hotkey))
    }

    /// A shortcut as the user reads it (`Strg+Umschalt+Leertaste`).
    fn shortcut_label(&self, hotkey: Hotkey) -> String {
        if self.german() {
            hotkey.german_label()
        } else {
            hotkey.to_string()
        }
    }

    fn refresh_tray(&self) {
        let settings = Arc::clone(&self.settings.borrow());
        let tray = &self.windows.tray;
        tray.set_paused(settings.paused);
        tray.set_autostart(settings.start_with_system);
        tray.set_search_shortcut(self.hotkey_label().into());
        self.windows.setup.set_autostart(settings.start_with_system);
        self.windows.diagnostics.set_paused(settings.paused);
    }

    fn open_editor(&self, args: &[&str]) {
        if !os::launch_editor(args) {
            self.show_message(MessageKind::EditorMissing, "");
            os::open_folder(&self.library.borrow().root);
        }
    }

    // ----- quick search ------------------------------------------------

    fn show_search(&self) {
        let search = &self.windows.search;
        search.set_query(SharedString::new());
        self.update_search("");
        search.invoke_prepare();
        self.show_window(search.window(), true);
    }

    fn update_search(&self, query: &str) {
        let library = Arc::clone(&self.library.borrow());
        let now = chrono::Local::now().naive_local();
        let language = self.date_language();
        let mut hits = library.search.search(query);
        hits.truncate(MAX_RESULTS);
        let items: Vec<SnippetItem> = hits
            .iter()
            .map(|&i| {
                let snippet = &library.snippets[i];
                SnippetItem {
                    id: snippet.id.to_string().into(),
                    trigger: snippet.trigger.as_str().into(),
                    label: snippet.label.as_str().into(),
                    preview: snippet
                        .content
                        .preview(90, now, language, &library.snippets)
                        .into(),
                    has_form: !nesting::form_fields(snippet, &library.snippets).is_empty(),
                }
            })
            .collect();
        let search = &self.windows.search;
        search.set_results(ModelRc::new(VecModel::from(items)));
        search.set_library_empty(library.enabled_count() == 0);
        *self.search_hits.borrow_mut() = hits;
    }

    fn choose(&self, index: i32) {
        let hit = usize::try_from(index)
            .ok()
            .and_then(|i| self.search_hits.borrow().get(i).copied());
        let Some(hit) = hit else {
            return;
        };
        let library = Arc::clone(&self.library.borrow());
        let Some(snippet) = library.snippets.get(hit) else {
            return;
        };
        self.hide_window(self.windows.search.window());
        if nesting::form_fields(snippet, &library.snippets).is_empty() {
            self.send(Command::Insert {
                id: snippet.id,
                fields: HashMap::new(),
            });
        } else {
            self.show_form(snippet.id);
        }
    }

    // ----- forms ----------------------------------------------------------

    fn show_form(&self, id: SnippetId) {
        let library = Arc::clone(&self.library.borrow());
        let Some(snippet) = library.get(id) else {
            // The snippet was deleted meanwhile; give the trigger back.
            self.send(Command::FormCancelled);
            return;
        };
        let form = &self.windows.form;
        let yes = form.get_yes_word().to_string();
        let no = form.get_no_word().to_string();
        // The snippet's fields and those of the snippets it includes.
        let fields = nesting::form_fields(snippet, &library.snippets);
        let rows: Vec<FormField> = fields
            .iter()
            .map(|field| {
                let default = field.default.clone().unwrap_or_default();
                let (kind, value, options) = match &field.kind {
                    CoreFieldKind::Text => (FieldKind::Text, default, Vec::new()),
                    CoreFieldKind::MultilineText => (FieldKind::Multiline, default, Vec::new()),
                    CoreFieldKind::Choice { options } => {
                        let value = if options.contains(&default) {
                            default
                        } else {
                            options.first().cloned().unwrap_or_default()
                        };
                        (FieldKind::Choice, value, options.clone())
                    }
                    CoreFieldKind::YesNo => {
                        let value = if is_yes(&default) {
                            yes.clone()
                        } else {
                            no.clone()
                        };
                        (FieldKind::YesNo, value, Vec::new())
                    }
                };
                FormField {
                    label: field.label.as_str().into(),
                    kind,
                    value: value.into(),
                    options: ModelRc::new(VecModel::from(
                        options
                            .into_iter()
                            .map(SharedString::from)
                            .collect::<Vec<_>>(),
                    )),
                }
            })
            .collect();
        self.form_fields.set_vec(rows);
        *self.form.borrow_mut() = Some(FormState {
            id,
            names: fields.iter().map(|f| f.name.clone()).collect(),
        });
        form.set_snippet_label(snippet.label.as_str().into());
        form.set_trigger(snippet.trigger.as_str().into());
        self.show_window(form.window(), true);
    }

    fn submit_form(&self) {
        let Some(state) = self.form.borrow_mut().take() else {
            return;
        };
        let fields = state
            .names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let value = self
                    .form_fields
                    .row_data(i)
                    .map(|row| row.value.to_string())
                    .unwrap_or_default();
                (name.clone(), value)
            })
            .collect();
        self.hide_window(self.windows.form.window());
        self.send(Command::Insert {
            id: state.id,
            fields,
        });
    }

    fn cancel_form(&self) {
        if self.form.borrow_mut().take().is_some() {
            self.hide_window(self.windows.form.window());
            self.send(Command::FormCancelled);
        }
    }

    // ----- new snippet ------------------------------------------------------

    fn show_create(&self, text: String) {
        let create = &self.windows.create;
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        create.set_text(text.trim_end().into());
        create.set_trigger(":".into());
        create.set_label(SharedString::new());
        create.set_saved(false);
        create.set_error(SharedString::new());
        create.set_problem(self.trigger_problem(":"));
        self.created.set(None);
        self.show_window(create.window(), true);
    }

    fn trigger_problem(&self, trigger: &str) -> TriggerProblem {
        match validate_trigger(trigger) {
            Err(ValidationError::EmptyTrigger) => TriggerProblem::Empty,
            Err(ValidationError::TriggerTooLong) => TriggerProblem::TooLong,
            Err(_) => TriggerProblem::Spaces,
            // A trigger that is only a symbol fires on every use of it.
            Ok(()) if !trigger.chars().any(char::is_alphanumeric) => TriggerProblem::Empty,
            Ok(()) if self.library.borrow().trigger_taken(trigger) => TriggerProblem::Taken,
            Ok(()) => TriggerProblem::None,
        }
    }

    fn save_created(&self) {
        let create = &self.windows.create;
        let trigger = create.get_trigger().to_string();
        let text = create.get_text().replace('\r', "");
        if self.trigger_problem(&trigger) != TriggerProblem::None || text.trim().is_empty() {
            return;
        }
        let mut label = create.get_label().trim().to_owned();
        if label.is_empty() {
            label = Document::from_plain_text(&text).preview(
                40,
                chrono::Local::now().naive_local(),
                self.date_language(),
                &[],
            );
        }
        let snippet = Snippet::new(trigger, label, Document::from_plain_text(&text));
        let library = Library::new(self.library.borrow().root.clone());
        match library.save(&snippet, &[]) {
            Ok(path) => {
                log::info!("created snippet {}", path.display());
                self.created.set(Some(snippet.id));
                create.set_saved(true);
                self.reload_library();
            }
            Err(error) => create.set_error(error.to_string().into()),
        }
    }

    // ----- set-up assistant -------------------------------------------------

    fn show_setup(&self) {
        let setup = &self.windows.setup;
        let library = Arc::clone(&self.library.borrow());
        let examples: Vec<ExampleItem> = library
            .snippets
            .iter()
            .filter(|s| s.enabled)
            .take(6)
            .map(|s| ExampleItem {
                trigger: s.trigger.as_str().into(),
                label: s.label.as_str().into(),
            })
            .collect();
        setup.set_examples(ModelRc::new(VecModel::from(examples)));

        let (trigger, expected) = self.test_snippet(&library);
        setup.set_test_trigger(trigger.into());
        *self.test_expected.borrow_mut() = expected;
        setup.set_test_text(SharedString::new());
        setup.set_test_passed(false);
        setup.set_search_shortcut(self.hotkey_label().into());
        setup.set_linux(cfg!(target_os = "linux"));
        setup.set_page(0);
        self.refresh_setup();
        self.show_window(setup.window(), true);
    }

    /// Picks the snippet the user should try: the example for the current
    /// language, or the first simple snippet of the library.
    fn test_snippet(&self, library: &Snapshot) -> (String, String) {
        let example = firstrun::test_snippet(self.german());
        let candidate = library
            .snippets
            .iter()
            .find(|s| s.enabled && s.trigger == example.trigger)
            .or_else(|| {
                library.snippets.iter().find(|s| {
                    s.enabled
                        && nesting::form_fields(s, &library.snippets).is_empty()
                        && !nesting::uses_clipboard(s, &library.snippets)
                })
            });
        let Some(snippet) = candidate else {
            return (String::new(), String::new());
        };
        let fields = HashMap::new();
        let context = RenderContext {
            now: chrono::Local::now().naive_local(),
            clipboard: None,
            fields: &fields,
            date_language: self.date_language(),
            snippets: &library.snippets,
        };
        let expected = render(snippet, &context)
            .map(|rendered| rendered.plain.lines().next().unwrap_or_default().to_owned())
            .unwrap_or_default();
        (snippet.trigger.clone(), expected)
    }

    fn refresh_setup(&self) {
        let status = Platform::status();
        let state = self.state.borrow().clone();
        let failed = matches!(state, EngineState::Failed(_));
        let setup = &self.windows.setup;
        setup.set_keyboard_ok(status.keyboard_access && !failed);
        setup.set_typing_ok(status.injection_access && !failed);
        setup.set_focus_ok(status.focus_detection);
        let problems = if status.keyboard_access && status.injection_access && !failed {
            String::new()
        } else if let Some(commands) = typlet_platform::setup_commands() {
            commands.to_owned()
        } else {
            let mut lines = status.problems.clone();
            if let EngineState::Failed(error) = state {
                lines.push(error);
            }
            lines.join("\n")
        };
        setup.set_problems(problems.into());
    }

    // ----- diagnostics ------------------------------------------------------

    fn show_diagnostics(&self) {
        self.diagnostics_wanted.set(true);
        self.send(Command::Diagnostics);
    }

    fn fill_diagnostics(&self, history: &[Attempt]) {
        if !self.diagnostics_wanted.replace(false) {
            return;
        }
        let status = Platform::status();
        let state = self.state.borrow().clone();
        let engine_error = match &state {
            EngineState::Failed(error) => Some(error.as_str()),
            _ => None,
        };
        let library = Arc::clone(&self.library.borrow());
        let paused = self.settings.borrow().paused;
        let window = &self.windows.diagnostics;
        window.set_keyboard_ok(status.keyboard_access && engine_error.is_none());
        window.set_typing_ok(status.injection_access && engine_error.is_none());
        window.set_focus_ok(status.focus_detection);
        window.set_paused(paused);
        window.set_snippet_count(i32::try_from(library.enabled_count()).unwrap_or(i32::MAX));
        let mut problems = status.problems.clone();
        if let Some(error) = engine_error {
            problems.push(error.to_owned());
        }
        window.set_problems(problems.join("\n").into());
        let issues: Vec<SharedString> = library.issues.iter().map(|i| i.as_str().into()).collect();
        window.set_library_issues(ModelRc::new(VecModel::from(issues)));
        let attempts: Vec<AttemptItem> = history
            .iter()
            .map(|attempt| {
                let (kind, detail) = attempt_kind(&attempt.result);
                AttemptItem {
                    time: attempt.at.format("%H:%M:%S").to_string().into(),
                    app: attempt.app.as_deref().unwrap_or("–").into(),
                    kind,
                    detail: detail.into(),
                }
            })
            .collect();
        window.set_attempts(ModelRc::new(VecModel::from(attempts)));
        window.set_report(
            report::build(&ReportInput {
                status: &status,
                engine_error,
                paused,
                library: &library,
                history,
            })
            .into(),
        );
        window.set_version(env!("CARGO_PKG_VERSION").into());
        self.show_window(window.window(), false);
    }

    // ----- messages ----------------------------------------------------------

    fn show_message(&self, kind: MessageKind, detail: &str) {
        self.message_kind.set(kind);
        let message = &self.windows.message;
        message.set_kind(kind);
        message.set_detail(detail.into());
        self.show_window(message.window(), true);
    }
}

fn attempt_kind(result: &AttemptResult) -> (AttemptKind, &str) {
    match result {
        AttemptResult::Typed => (AttemptKind::Typed, ""),
        AttemptResult::Pasted => (AttemptKind::Pasted, ""),
        AttemptResult::PastedFormatted => (AttemptKind::PastedFormatted, ""),
        AttemptResult::FormOpened => (AttemptKind::FormOpened, ""),
        AttemptResult::Undone => (AttemptKind::Undone, ""),
        AttemptResult::SkippedPassword => (AttemptKind::SkippedPassword, ""),
        AttemptResult::SkippedApp => (AttemptKind::SkippedApp, ""),
        AttemptResult::Failed(error) => (AttemptKind::Failed, error),
    }
}

/// Accepts the yes words of both interface languages as a default value.
fn is_yes(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "ja" | "yes" | "true" | "1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yes_defaults_are_recognised_in_both_languages() {
        assert!(is_yes("Ja"));
        assert!(is_yes(" yes "));
        assert!(!is_yes("Nein"));
        assert!(!is_yes(""));
    }

    #[test]
    fn german_follows_the_setting() {
        let mut settings = Settings {
            language: Language::De,
            ..Settings::default()
        };
        assert!(is_german(&settings));
        settings.language = Language::En;
        assert!(!is_german(&settings));
    }
}
