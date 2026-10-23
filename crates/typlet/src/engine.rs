// SPDX-License-Identifier: GPL-3.0-or-later
//! The engine: turns keyboard events into expansions.
//!
//! The engine runs on its own thread. It owns the platform backend, the
//! trigger matcher and the current library and settings. The user interface
//! talks to it with [`Command`]s and receives [`Notice`]s.
//!
//! Privacy: the engine keeps typed characters only inside the matcher (at most
//! as many as the longest trigger). It never logs them. The diagnostics
//! history records which app was involved and what happened, never the text.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::NaiveDateTime;
use crossbeam_channel::{Receiver, Sender};
use typlet_core::{
    Action, EffectiveRule, Hotkey, Input, InsertMethod, InsertedText, Matcher, Outcome,
    RenderContext, Settings, Snippet, SnippetId, plan_expansion, plan_undo, render,
};
use typlet_platform::{Backend, Event, InjectOptions, Platform, PlatformError};

use crate::snapshot::Snapshot;

/// How many recent expansions diagnostics shows.
const HISTORY: usize = 20;

/// Time for the window manager to give the focus back to the target app
/// after one of Typlet's windows closed.
#[cfg(windows)]
const FOCUS_SETTLE: Duration = Duration::from_millis(120);
#[cfg(not(windows))]
const FOCUS_SETTLE: Duration = Duration::from_millis(250);

/// Requests from the user interface (and the library watcher).
#[derive(Debug)]
pub(crate) enum Command {
    /// The library changed on disk.
    Library(Arc<Snapshot>),
    /// The settings changed.
    Settings(Arc<Settings>),
    /// Insert a snippet chosen in the search or completed in a form.
    Insert {
        /// The snippet.
        id: SnippetId,
        /// Form values by field name.
        fields: HashMap<String, String>,
    },
    /// The user closed a form without inserting.
    FormCancelled,
    /// One of Typlet's windows closed; return the focus to the target app.
    RestoreFocus,
    /// `true` while a Typlet window with text fields (search, form, new
    /// snippet) is open, so triggers typed there do not expand.
    Quiet(bool),
    /// Send the diagnostics history.
    Diagnostics,
    /// Try again to start keyboard access after it failed.
    Retry,
    /// Stop the engine.
    Quit,
}

/// Messages from the engine to the user interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Notice {
    /// Keyboard access works.
    Started,
    /// Keyboard access could not be started; the text explains why.
    InputFailed(String),
    /// Open the quick search.
    ShowSearch,
    /// Ask for the form values of a snippet.
    ShowForm(SnippetId),
    /// The selected text for a new snippet (`None` if nothing was selected).
    Selection(Option<String>),
    /// Inserting failed; the text is a technical explanation.
    InsertFailed(String),
    /// The diagnostics history, newest first.
    Diagnostics(Vec<Attempt>),
}

/// What happened when a snippet was due to be inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttemptResult {
    /// Typed with simulated key presses.
    Typed,
    /// Pasted as plain text.
    Pasted,
    /// Pasted with formatting.
    PastedFormatted,
    /// A form was opened.
    FormOpened,
    /// The user undid the expansion with Backspace.
    Undone,
    /// Not inserted because the focus is in a password field.
    SkippedPassword,
    /// Not inserted because Typlet is turned off for this app.
    SkippedApp,
    /// Inserting failed.
    Failed(String),
}

/// One entry of the diagnostics history. Contains no snippet text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attempt {
    /// When it happened (local time).
    pub(crate) at: NaiveDateTime,
    /// The focused application, if known.
    pub(crate) app: Option<String>,
    /// The outcome.
    pub(crate) result: AttemptResult,
}

/// What a registered hotkey does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeyAction {
    Search,
    Selection,
    Snippet(SnippetId),
}

/// A form that is open; the trigger was already removed from the app.
#[derive(Debug)]
struct PendingForm {
    trigger: String,
}

/// Starts the engine thread. Notices are delivered through `notify`.
pub(crate) fn spawn(
    settings: Arc<Settings>,
    library: Arc<Snapshot>,
    editor_lock: PathBuf,
    notify: impl Fn(Notice) + Send + 'static,
) -> std::io::Result<Sender<Command>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::Builder::new()
        .name("typlet-engine".into())
        .spawn(move || run(&rx, settings, library, &editor_lock, notify))?;
    Ok(tx)
}

fn run(
    commands: &Receiver<Command>,
    mut settings: Arc<Settings>,
    mut library: Arc<Snapshot>,
    editor_lock: &Path,
    notify: impl Fn(Notice),
) {
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    loop {
        match start_platform(&event_tx) {
            Ok(platform) => {
                log::info!("keyboard access started");
                notify(Notice::Started);
                let mut engine = Engine::new(platform, notify, library, settings, now);
                engine.editor_lock = Some(editor_lock.to_path_buf());
                engine.run(commands, &event_rx);
                return;
            }
            Err(error) => {
                log::warn!("keyboard access failed: {error}");
                notify(Notice::InputFailed(error.to_string()));
                // Keep the latest state until the user retries.
                loop {
                    match commands.recv() {
                        Ok(Command::Retry) => break,
                        Ok(Command::Library(new)) => library = new,
                        Ok(Command::Settings(new)) => settings = new,
                        Ok(Command::Diagnostics) => notify(Notice::Diagnostics(Vec::new())),
                        Ok(Command::Quit) | Err(_) => return,
                        Ok(_) => {}
                    }
                }
            }
        }
    }
}

fn start_platform(events: &Sender<Event>) -> Result<Platform, PlatformError> {
    let mut platform = Platform::new()?;
    platform.start(events.clone())?;
    Ok(platform)
}

fn now() -> NaiveDateTime {
    chrono::Local::now().naive_local()
}

/// The expansion logic, independent of threads so it can be tested with a
/// fake backend.
pub(crate) struct Engine<B, N> {
    backend: B,
    notify: N,
    library: Arc<Snapshot>,
    settings: Arc<Settings>,
    matcher: Matcher,
    hotkeys: Vec<HotkeyAction>,
    last: Option<InsertedText>,
    pending_form: Option<PendingForm>,
    quiet: bool,
    history: VecDeque<Attempt>,
    clock: fn() -> NaiveDateTime,
    focus_settle: Duration,
    /// Lock file the editor holds while it has the focus.
    editor_lock: Option<PathBuf>,
}

/// Platform features the engine needs beyond [`Backend`].
pub(crate) trait EngineBackend: Backend {
    /// Replaces the registered hotkeys.
    fn set_hotkeys(&mut self, hotkeys: &[Hotkey]);
    /// Remembers the focused window before one of Typlet's windows opens.
    fn remember_focus(&mut self);
    /// Gives the focus back to the remembered window.
    fn restore_focus(&mut self);
}

impl EngineBackend for Platform {
    fn set_hotkeys(&mut self, hotkeys: &[Hotkey]) {
        Platform::set_hotkeys(self, hotkeys);
    }

    fn remember_focus(&mut self) {
        Platform::remember_focus(self);
    }

    fn restore_focus(&mut self) {
        Platform::restore_focus(self);
    }
}

impl<B: EngineBackend, N: FnMut(Notice)> Engine<B, N> {
    pub(crate) fn new(
        backend: B,
        notify: N,
        library: Arc<Snapshot>,
        settings: Arc<Settings>,
        clock: fn() -> NaiveDateTime,
    ) -> Self {
        let mut engine = Self {
            backend,
            notify,
            library,
            settings,
            matcher: Matcher::default(),
            hotkeys: Vec::new(),
            last: None,
            pending_form: None,
            quiet: false,
            history: VecDeque::with_capacity(HISTORY),
            clock,
            focus_settle: FOCUS_SETTLE,
            editor_lock: None,
        };
        engine.rebuild();
        engine
    }

    fn run(&mut self, commands: &Receiver<Command>, events: &Receiver<Event>) {
        loop {
            crossbeam_channel::select! {
                recv(events) -> event => match event {
                    Ok(event) => self.handle_event(event),
                    Err(_) => return,
                },
                recv(commands) -> command => match command {
                    Ok(Command::Quit) | Err(_) => return,
                    Ok(command) => self.handle_command(command),
                },
            }
        }
    }

    /// Rebuilds the matcher and hotkeys after the library or settings changed.
    fn rebuild(&mut self) {
        self.matcher = Matcher::new(self.library.snippets.iter());
        self.last = None;

        let mut keys = Vec::new();
        let mut actions = Vec::new();
        let mut add = |hotkey: Hotkey, action: HotkeyAction| {
            if keys.contains(&hotkey) {
                log::warn!("hotkey {hotkey} is assigned twice; only the first one works");
            } else {
                keys.push(hotkey);
                actions.push(action);
            }
        };
        if let Some(hotkey) = self.settings.search_hotkey {
            add(hotkey, HotkeyAction::Search);
        }
        if let Some(hotkey) = self.settings.selection_hotkey {
            add(hotkey, HotkeyAction::Selection);
        }
        for snippet in self.library.snippets.iter().filter(|s| s.enabled) {
            if let Some(hotkey) = snippet.hotkey {
                add(hotkey, HotkeyAction::Snippet(snippet.id));
            }
        }
        self.backend.set_hotkeys(&keys);
        self.hotkeys = actions;
    }

    pub(crate) fn handle_event(&mut self, event: Event) {
        match event {
            Event::Char(c) => {
                if self.settings.paused || self.quiet || self.pending_form.is_some() {
                    return;
                }
                if let Outcome::Expand(expansion) = self.matcher.feed(Input::Char(c)) {
                    self.expand(expansion.snippet);
                }
            }
            Event::Backspace { suppressed } => self.backspace(suppressed),
            Event::Reset => {
                self.matcher.feed(Input::Reset);
                self.last = None;
                self.backend.arm_undo(false);
            }
            Event::Hotkey(index) => {
                if let Some(&action) = self.hotkeys.get(index) {
                    self.hotkey(action);
                }
            }
        }
    }

    pub(crate) fn handle_command(&mut self, command: Command) {
        match command {
            Command::Library(library) => {
                self.library = library;
                self.rebuild();
            }
            Command::Settings(settings) => {
                self.settings = settings;
                self.rebuild();
            }
            Command::Insert { id, fields } => {
                self.pending_form = None;
                self.backend.restore_focus();
                thread::sleep(self.focus_settle);
                if let Some(snippet) = self.library.get(id).cloned() {
                    self.insert_checked(&snippet, &fields, "");
                }
            }
            Command::FormCancelled => {
                self.backend.restore_focus();
                if let Some(pending) = self.pending_form.take()
                    && !pending.trigger.is_empty()
                {
                    // Give the trigger back, as if the form never opened.
                    thread::sleep(self.focus_settle);
                    let options = inject_options(&self.settings.rule_for(None));
                    if let Err(error) = self
                        .backend
                        .execute(&[Action::Type(pending.trigger)], &options)
                    {
                        log::warn!("cannot restore the trigger: {error}");
                    }
                }
            }
            Command::RestoreFocus => self.backend.restore_focus(),
            Command::Quiet(quiet) => {
                self.quiet = quiet;
                self.matcher.feed(Input::Reset);
                self.last = None;
            }
            Command::Diagnostics => {
                let history = self.history.iter().rev().cloned().collect();
                (self.notify)(Notice::Diagnostics(history));
            }
            Command::Retry | Command::Quit => {}
        }
    }

    fn backspace(&mut self, suppressed: bool) {
        let outcome = self.matcher.feed(Input::Backspace);
        self.backend.arm_undo(false);
        let last = self.last.take();
        if let (Outcome::Undo(_), Some(last)) = (outcome, last)
            && let Some(actions) = plan_undo(&last, !suppressed)
        {
            let options = inject_options(&self.settings.rule_for(None));
            match self.backend.execute(&actions, &options) {
                Ok(()) => self.record(None, AttemptResult::Undone),
                Err(error) => self.record(None, AttemptResult::Failed(error.to_string())),
            }
            return;
        }
        if suppressed {
            // The platform held the key back for an undo that is not
            // possible; deliver it now.
            let options = inject_options(&self.settings.rule_for(None));
            if let Err(error) = self.backend.execute(&[Action::Backspace(1)], &options) {
                log::warn!("cannot deliver backspace: {error}");
            }
        }
    }

    fn expand(&mut self, id: SnippetId) {
        let Some(snippet) = self.library.get(id).cloned() else {
            return;
        };
        let Some((app, rule)) = self.target() else {
            self.matcher.feed(Input::Reset);
            return;
        };
        if snippet.fields.is_empty() {
            self.insert(&snippet, &HashMap::new(), &snippet.trigger, app, &rule);
        } else {
            self.open_form(&snippet, &snippet.trigger, app, &rule);
        }
    }

    fn hotkey(&mut self, action: HotkeyAction) {
        if self.pending_form.is_some() {
            return;
        }
        self.matcher.feed(Input::Reset);
        self.last = None;
        match action {
            HotkeyAction::Search => {
                self.backend.remember_focus();
                (self.notify)(Notice::ShowSearch);
            }
            HotkeyAction::Selection => {
                let text = self.backend.copy_selection();
                self.backend.remember_focus();
                (self.notify)(Notice::Selection(text));
            }
            HotkeyAction::Snippet(id) => {
                let Some(snippet) = self.library.get(id).cloned() else {
                    return;
                };
                let Some((app, rule)) = self.target() else {
                    return;
                };
                if snippet.fields.is_empty() {
                    self.insert(&snippet, &HashMap::new(), "", app, &rule);
                } else {
                    self.open_form(&snippet, "", app, &rule);
                }
            }
        }
    }

    /// Checks whether Typlet may insert into the focused app and returns the
    /// app and its rule. Records why not otherwise.
    fn target(&mut self) -> Option<(Option<String>, EffectiveRule)> {
        // Typing a trigger into the editor (for example as a new snippet's
        // trigger) must not expand it.
        if self
            .editor_lock
            .as_deref()
            .is_some_and(typlet_platform::instance::is_running)
        {
            return None;
        }
        let focused = self.backend.focused_app();
        if focused.password_field {
            self.record(focused.app, AttemptResult::SkippedPassword);
            return None;
        }
        let rule = self.settings.rule_for(focused.app.as_deref());
        if !rule.enabled {
            self.record(focused.app, AttemptResult::SkippedApp);
            return None;
        }
        Some((focused.app, rule))
    }

    fn open_form(
        &mut self,
        snippet: &Snippet,
        trigger: &str,
        app: Option<String>,
        rule: &EffectiveRule,
    ) {
        let chars = trigger.chars().count();
        if chars > 0
            && let Err(error) = self
                .backend
                .execute(&[Action::Backspace(chars)], &inject_options(rule))
        {
            self.fail(app, &error);
            return;
        }
        self.matcher.feed(Input::Reset);
        self.last = None;
        self.backend.remember_focus();
        self.pending_form = Some(PendingForm {
            trigger: trigger.to_owned(),
        });
        self.record(app, AttemptResult::FormOpened);
        (self.notify)(Notice::ShowForm(snippet.id));
    }

    fn insert_checked(
        &mut self,
        snippet: &Snippet,
        fields: &HashMap<String, String>,
        trigger: &str,
    ) {
        if let Some((app, rule)) = self.target() {
            self.insert(snippet, fields, trigger, app, &rule);
        }
    }

    fn insert(
        &mut self,
        snippet: &Snippet,
        fields: &HashMap<String, String>,
        trigger: &str,
        app: Option<String>,
        rule: &EffectiveRule,
    ) {
        let clipboard = if snippet.content.uses_clipboard() {
            self.backend.clipboard_text()
        } else {
            None
        };
        let fields = with_defaults(snippet, fields);
        let context = RenderContext {
            now: (self.clock)(),
            clipboard: clipboard.as_deref(),
            fields: &fields,
        };
        let rendered = match render(snippet, &context) {
            Ok(rendered) => rendered,
            Err(error) => {
                self.record(app, AttemptResult::Failed(error.to_string()));
                (self.notify)(Notice::InsertFailed(error.to_string()));
                return;
            }
        };
        let mode = rule.insert_mode.unwrap_or(snippet.insert_mode);
        let method = rendered.insert_method(mode);
        let (actions, inserted) = plan_expansion(trigger, &rendered, method, rule.plain_only);

        match self.backend.execute(&actions, &inject_options(rule)) {
            Ok(()) => {
                let result = match method {
                    InsertMethod::Type => AttemptResult::Typed,
                    InsertMethod::Paste if rendered.formatted && !rule.plain_only => {
                        AttemptResult::PastedFormatted
                    }
                    InsertMethod::Paste => AttemptResult::Pasted,
                };
                self.record(app, result);
                if trigger.is_empty() {
                    self.matcher.feed(Input::Reset);
                    self.last = None;
                } else {
                    self.backend.arm_undo(inserted.undoable);
                    self.last = Some(inserted);
                }
            }
            Err(error) => self.fail(app, &error),
        }
    }

    fn fail(&mut self, app: Option<String>, error: &PlatformError) {
        log::warn!("inserting failed: {error}");
        self.matcher.feed(Input::Reset);
        self.last = None;
        self.record(app, AttemptResult::Failed(error.to_string()));
        (self.notify)(Notice::InsertFailed(error.to_string()));
    }

    fn record(&mut self, app: Option<String>, result: AttemptResult) {
        if self.history.len() == HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(Attempt {
            at: (self.clock)(),
            app,
            result,
        });
    }
}

/// Fills in defaults (or empty text) for fields without a value, so a form
/// that was skipped never makes rendering fail.
fn with_defaults(snippet: &Snippet, fields: &HashMap<String, String>) -> HashMap<String, String> {
    let mut values = fields.clone();
    for field in &snippet.fields {
        values
            .entry(field.name.clone())
            .or_insert_with(|| field.default.clone().unwrap_or_default());
    }
    values
}

fn inject_options(rule: &EffectiveRule) -> InjectOptions {
    InjectOptions {
        newline: rule.newline,
        restore_delay: Duration::from_millis(u64::from(rule.restore_delay_ms)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use typlet_core::{
        AppRule, Block, ClipboardContent, Document, FieldDef, FieldKind, Inline, Marks, NewlineMode,
    };
    use typlet_platform::FocusedApp;

    #[derive(Default)]
    struct Log {
        actions: Vec<Vec<Action>>,
        options: Vec<InjectOptions>,
        hotkeys: Vec<Hotkey>,
        armed: Vec<bool>,
        remembered: usize,
        restored: usize,
    }

    #[derive(Default)]
    struct Fake {
        log: Rc<RefCell<Log>>,
        app: Option<String>,
        password: bool,
        clipboard: Option<String>,
        selection: Option<String>,
        fail: bool,
    }

    impl Backend for Fake {
        fn execute(
            &mut self,
            actions: &[Action],
            options: &InjectOptions,
        ) -> Result<(), PlatformError> {
            if self.fail {
                return Err(PlatformError::Other("blocked".into()));
            }
            let mut log = self.log.borrow_mut();
            log.actions.push(actions.to_vec());
            log.options.push(*options);
            Ok(())
        }

        fn focused_app(&mut self) -> FocusedApp {
            FocusedApp {
                app: self.app.clone(),
                password_field: self.password,
            }
        }

        fn clipboard_text(&mut self) -> Option<String> {
            self.clipboard.clone()
        }

        fn copy_selection(&mut self) -> Option<String> {
            self.selection.clone()
        }

        fn arm_undo(&self, armed: bool) {
            self.log.borrow_mut().armed.push(armed);
        }
    }

    impl EngineBackend for Fake {
        fn set_hotkeys(&mut self, hotkeys: &[Hotkey]) {
            self.log.borrow_mut().hotkeys = hotkeys.to_vec();
        }

        fn remember_focus(&mut self) {
            self.log.borrow_mut().remembered += 1;
        }

        fn restore_focus(&mut self) {
            self.log.borrow_mut().restored += 1;
        }
    }

    fn fixed_clock() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
            .and_then(|d| d.and_hms_opt(9, 30, 0))
            .unwrap()
    }

    type Notices = Rc<RefCell<Vec<Notice>>>;
    type Setup = (
        Engine<Fake, Box<dyn FnMut(Notice)>>,
        Rc<RefCell<Log>>,
        Notices,
    );

    fn setup(snippets: Vec<Snippet>, settings: Settings, backend: Fake) -> Setup {
        let log = Rc::clone(&backend.log);
        let notices: Notices = Rc::default();
        let sink = Rc::clone(&notices);
        let library = Arc::new(Snapshot::from_parts("lib".into(), snippets, Vec::new(), 0));
        let notify: Box<dyn FnMut(Notice)> = Box::new(move |notice| sink.borrow_mut().push(notice));
        let mut engine = Engine::new(backend, notify, library, Arc::new(settings), fixed_clock);
        engine.focus_settle = Duration::ZERO;
        (engine, log, notices)
    }

    fn type_text<B: EngineBackend, N: FnMut(Notice)>(engine: &mut Engine<B, N>, text: &str) {
        for c in text.chars() {
            engine.handle_event(Event::Char(c));
        }
    }

    fn plain(trigger: &str, text: &str) -> Snippet {
        Snippet::new(trigger, trigger, Document::from_plain_text(text))
    }

    fn form_snippet() -> Snippet {
        let mut snippet = Snippet::new(
            ":rr",
            "Callback",
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![Inline::text("Call "), Inline::Field { name: "who".into() }],
                }],
            },
        );
        snippet.fields = vec![FieldDef {
            name: "who".into(),
            label: "Who?".into(),
            kind: FieldKind::Text,
            default: Some("nobody".into()),
        }];
        snippet
    }

    #[test]
    fn typing_a_trigger_replaces_it() {
        let (mut engine, log, _) = setup(
            vec![plain(":mfg", "Grüße")],
            Settings::default(),
            Fake::default(),
        );
        type_text(&mut engine, "x:mfg");
        let log = log.borrow();
        assert_eq!(
            log.actions,
            vec![vec![Action::Backspace(4), Action::Type("Grüße".into())]]
        );
        assert_eq!(log.armed, vec![true]);
        assert_eq!(
            engine.history.back().map(|a| &a.result),
            Some(&AttemptResult::Typed)
        );
    }

    #[test]
    fn paused_engine_does_nothing() {
        let settings = Settings {
            paused: true,
            ..Settings::default()
        };
        let (mut engine, log, _) = setup(vec![plain(":mfg", "Grüße")], settings, Fake::default());
        type_text(&mut engine, ":mfg");
        assert!(log.borrow().actions.is_empty());
    }

    #[test]
    fn password_fields_and_disabled_apps_are_skipped() {
        let backend = Fake {
            password: true,
            ..Fake::default()
        };
        let (mut engine, log, _) =
            setup(vec![plain(":pw", "secret")], Settings::default(), backend);
        type_text(&mut engine, ":pw");
        assert!(log.borrow().actions.is_empty());
        assert_eq!(
            engine.history.back().map(|a| &a.result),
            Some(&AttemptResult::SkippedPassword)
        );

        let settings = Settings {
            app_rules: vec![AppRule {
                app: "game.exe".into(),
                enabled: false,
                insert_mode: None,
                newline: None,
                plain_only: None,
                restore_delay_ms: None,
            }],
            ..Settings::default()
        };
        let backend = Fake {
            app: Some("Game.exe".into()),
            ..Fake::default()
        };
        let (mut engine, log, _) = setup(vec![plain(":pw", "secret")], settings, backend);
        type_text(&mut engine, ":pw");
        assert!(log.borrow().actions.is_empty());
        assert_eq!(
            engine.history.back().map(|a| &a.result),
            Some(&AttemptResult::SkippedApp)
        );
    }

    #[test]
    fn backspace_right_after_expansion_restores_the_trigger() {
        let (mut engine, log, _) = setup(
            vec![plain(":hi", "Hello")],
            Settings::default(),
            Fake::default(),
        );
        type_text(&mut engine, ":hi");
        // Linux: the Backspace reached the app and deleted one character.
        engine.handle_event(Event::Backspace { suppressed: false });
        assert_eq!(
            log.borrow().actions[1],
            vec![Action::Backspace(4), Action::Type(":hi".into())]
        );

        // A second Backspace is an ordinary key press again.
        engine.handle_event(Event::Backspace { suppressed: false });
        assert_eq!(log.borrow().actions.len(), 2);
    }

    #[test]
    fn suppressed_backspace_is_delivered_when_undo_is_impossible() {
        let mut bold = Snippet::new(
            ":b",
            "Bold",
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![Inline::styled(
                        "Wichtig",
                        Marks {
                            bold: true,
                            ..Marks::default()
                        },
                    )],
                }],
            },
        );
        bold.insert_mode = typlet_core::InsertMode::Paste;
        let (mut engine, log, _) = setup(vec![bold], Settings::default(), Fake::default());
        type_text(&mut engine, ":b");
        assert!(matches!(
            log.borrow().actions[0][1],
            Action::Paste(ClipboardContent { html: Some(_), .. })
        ));
        assert_eq!(log.borrow().armed, vec![false]);
        engine.handle_event(Event::Backspace { suppressed: true });
        assert_eq!(log.borrow().actions[1], vec![Action::Backspace(1)]);
    }

    #[test]
    fn forms_remove_the_trigger_then_insert_the_values() {
        let snippet = form_snippet();
        let id = snippet.id;
        let (mut engine, log, notices) = setup(vec![snippet], Settings::default(), Fake::default());
        type_text(&mut engine, ":rr");
        assert_eq!(log.borrow().actions, vec![vec![Action::Backspace(3)]]);
        assert_eq!(*notices.borrow(), vec![Notice::ShowForm(id)]);
        assert_eq!(log.borrow().remembered, 1);

        // Typing while the form is open does not expand anything.
        type_text(&mut engine, ":rr");
        assert_eq!(log.borrow().actions.len(), 1);

        let fields = HashMap::from([("who".to_owned(), "Anna".to_owned())]);
        engine.handle_command(Command::Insert { id, fields });
        assert_eq!(
            log.borrow().actions[1],
            vec![Action::Type("Call Anna".into())]
        );
        assert_eq!(log.borrow().restored, 1);
    }

    #[test]
    fn cancelled_forms_restore_the_trigger() {
        let (mut engine, log, _) =
            setup(vec![form_snippet()], Settings::default(), Fake::default());
        type_text(&mut engine, ":rr");
        engine.handle_command(Command::FormCancelled);
        assert_eq!(log.borrow().actions[1], vec![Action::Type(":rr".into())]);
        // The engine listens again.
        type_text(&mut engine, ":rr");
        assert_eq!(log.borrow().actions.len(), 3);
    }

    #[test]
    fn hotkeys_open_search_and_insert_snippets() {
        let mut snippet = plain(":hi", "Hello");
        snippet.hotkey = Some("Ctrl+Alt+H".parse().unwrap());
        let (mut engine, log, notices) = setup(vec![snippet], Settings::default(), Fake::default());
        assert_eq!(log.borrow().hotkeys.len(), 3);

        engine.handle_event(Event::Hotkey(0));
        assert_eq!(*notices.borrow(), vec![Notice::ShowSearch]);

        engine.handle_event(Event::Hotkey(2));
        assert_eq!(
            log.borrow().actions,
            vec![vec![Action::Type("Hello".into())]]
        );
    }

    #[test]
    fn selection_hotkey_reports_the_selected_text() {
        let backend = Fake {
            selection: Some("Selected".into()),
            ..Fake::default()
        };
        let (mut engine, _, notices) = setup(Vec::new(), Settings::default(), backend);
        engine.handle_event(Event::Hotkey(1));
        assert_eq!(
            *notices.borrow(),
            vec![Notice::Selection(Some("Selected".into()))]
        );
    }

    #[test]
    fn app_rules_choose_line_breaks_and_delays() {
        let backend = Fake {
            app: Some("Teams.exe".into()),
            ..Fake::default()
        };
        let snippet = Snippet::new(":two", "Two lines", Document::from_plain_text("eins\nzwei"));
        let (mut engine, log, _) = setup(vec![snippet], Settings::default(), backend);
        type_text(&mut engine, ":two");
        let log = log.borrow();
        assert_eq!(log.options[0].newline, NewlineMode::ShiftEnter);
        assert!(matches!(log.actions[0][1], Action::Paste(_)));
    }

    #[test]
    fn clipboard_placeholders_read_the_clipboard() {
        let backend = Fake {
            clipboard: Some("from clipboard".into()),
            ..Fake::default()
        };
        let snippet = Snippet::new(
            ":cb",
            "Clipboard",
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![Inline::text("> "), Inline::Clipboard],
                }],
            },
        );
        let (mut engine, log, _) = setup(vec![snippet], Settings::default(), backend);
        type_text(&mut engine, ":cb");
        assert_eq!(
            log.borrow().actions[0][1],
            Action::Type("> from clipboard".into())
        );
    }

    #[test]
    fn failures_are_reported() {
        let backend = Fake {
            fail: true,
            ..Fake::default()
        };
        let (mut engine, _, notices) = setup(vec![plain(":x", "y")], Settings::default(), backend);
        type_text(&mut engine, ":x");
        assert_eq!(
            *notices.borrow(),
            vec![Notice::InsertFailed("blocked".into())]
        );
        engine.handle_command(Command::Diagnostics);
        let notices = notices.borrow();
        let Notice::Diagnostics(history) = &notices[1] else {
            panic!("expected diagnostics");
        };
        assert_eq!(history[0].result, AttemptResult::Failed("blocked".into()));
    }

    #[test]
    fn quiet_mode_ignores_triggers_typed_in_typlet_windows() {
        let (mut engine, log, _) = setup(
            vec![plain(":hi", "Hello")],
            Settings::default(),
            Fake::default(),
        );
        engine.handle_command(Command::Quiet(true));
        type_text(&mut engine, ":hi");
        assert!(log.borrow().actions.is_empty());
        engine.handle_command(Command::Quiet(false));
        type_text(&mut engine, ":hi");
        assert_eq!(log.borrow().actions.len(), 1);
    }

    #[test]
    fn nothing_expands_while_the_editor_has_the_focus() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("editor-focus.lock");
        let (mut engine, log, _) = setup(
            vec![plain(":hi", "Hello")],
            Settings::default(),
            Fake::default(),
        );
        engine.editor_lock = Some(lock.clone());
        let guard = typlet_platform::instance::acquire(&lock).unwrap();
        type_text(&mut engine, ":hi");
        assert!(log.borrow().actions.is_empty());
        drop(guard);
        type_text(&mut engine, ":hi");
        assert_eq!(log.borrow().actions.len(), 1);
    }

    #[test]
    fn library_changes_rebuild_the_matcher() {
        let (mut engine, log, _) = setup(Vec::new(), Settings::default(), Fake::default());
        type_text(&mut engine, ":new");
        assert!(log.borrow().actions.is_empty());
        let library = Snapshot::from_parts("lib".into(), vec![plain(":new", "neu")], Vec::new(), 0);
        engine.handle_command(Command::Library(Arc::new(library)));
        type_text(&mut engine, ":new");
        assert_eq!(log.borrow().actions.len(), 1);
    }
}
