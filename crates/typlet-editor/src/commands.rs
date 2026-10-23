// SPDX-License-Identifier: GPL-3.0-or-later
//! Commands the editor's web interface can call.
//!
//! All file access happens here, in Rust, through `typlet-core`: the web
//! interface never receives or sends file paths for snippets, only ids and
//! category names, so it cannot touch files outside the library.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use typlet_core::{
    Conflict, DateLanguage, InsertMethod, Library, RenderContext, Settings, Snippet, SnippetId,
    find_conflicts, nesting, render,
};
use typlet_platform::paths::Paths;

/// Shared state of the editor.
pub(crate) struct EditorState {
    pub(crate) paths: Paths,
    pub(crate) start: StartArgs,
}

/// What the command line asked the editor to show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StartArgs {
    /// Open this snippet (`--open <id>`).
    pub(crate) open_snippet: Option<String>,
    /// Open the settings page (`--settings`).
    pub(crate) open_settings: bool,
    /// Only make sure the background program runs, without a window
    /// (`--autostart` or `--background`).
    pub(crate) background_only: bool,
    /// Check for a new version without a window and exit
    /// (`--check-updates`, started by the background program).
    pub(crate) check_updates: bool,
    /// Open the update page (`--update`).
    pub(crate) open_update: bool,
}

impl StartArgs {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut parsed = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--open" => parsed.open_snippet = args.next(),
                "--settings" => parsed.open_settings = true,
                "--autostart" | "--background" => parsed.background_only = true,
                "--check-updates" => parsed.check_updates = true,
                "--update" => parsed.open_update = true,
                _ => {}
            }
        }
        parsed
    }
}

/// Errors are shown to the user as text.
type CommandResult<T> = Result<T, String>;

fn settings(paths: &Paths) -> Settings {
    Settings::load(&paths.settings_file).unwrap_or_default()
}

fn library(paths: &Paths) -> Library {
    Library::new(paths.library(&settings(paths)))
}

// ----- app ------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct AppInfo {
    version: &'static str,
    library: PathBuf,
    settings_file: PathBuf,
    default_espanso_dir: Option<PathBuf>,
    open_snippet: Option<String>,
    open_settings: bool,
    open_update: bool,
    system_german: bool,
    background_running: bool,
}

#[tauri::command]
pub(crate) fn app_info(state: State<'_, EditorState>) -> AppInfo {
    let paths = &state.paths;
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        library: paths.library(&settings(paths)),
        settings_file: paths.settings_file.clone(),
        default_espanso_dir: espanso_dir(),
        open_snippet: state.start.open_snippet.clone(),
        open_settings: state.start.open_settings,
        open_update: state.start.open_update,
        system_german: typlet_platform::system_language()
            .is_some_and(|lang| lang.to_lowercase().starts_with("de")),
        background_running: typlet_platform::instance::is_running(&paths.lock_file),
    }
}

/// espanso's configuration folder, if it exists.
fn espanso_dir() -> Option<PathBuf> {
    let dir = directories::BaseDirs::new()?.config_dir().join("espanso");
    dir.is_dir().then_some(dir)
}

// ----- library --------------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct LoadedView {
    snippet: Snippet,
    category: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ConflictView {
    Duplicate {
        trigger: String,
        snippets: [String; 2],
    },
    Shadowed {
        by: String,
        shadowed: String,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct LibraryView {
    root: PathBuf,
    snippets: Vec<LoadedView>,
    errors: Vec<String>,
    conflicts: Vec<ConflictView>,
}

fn load_view(library: &Library) -> LibraryView {
    let report = library.load();
    let snippets: Vec<Snippet> = report.snippets.iter().map(|l| l.snippet.clone()).collect();
    let conflicts = find_conflicts(&snippets)
        .into_iter()
        .map(|conflict| match conflict {
            Conflict::Duplicate { trigger, snippets } => ConflictView::Duplicate {
                trigger,
                snippets: [snippets[0].to_string(), snippets[1].to_string()],
            },
            Conflict::Shadowed { by, shadowed } => ConflictView::Shadowed {
                by: by.to_string(),
                shadowed: shadowed.to_string(),
            },
        })
        .collect();
    LibraryView {
        root: library.root().to_path_buf(),
        errors: report.errors.iter().map(ToString::to_string).collect(),
        snippets: report
            .snippets
            .into_iter()
            .map(|l| LoadedView {
                snippet: l.snippet,
                category: l.category,
            })
            .collect(),
        conflicts,
    }
}

#[tauri::command]
pub(crate) fn load_library(state: State<'_, EditorState>) -> LibraryView {
    load_view(&library(&state.paths))
}

/// Saves a snippet into a category, moving it if its category changed.
#[tauri::command]
pub(crate) fn save_snippet(
    state: State<'_, EditorState>,
    snippet: Snippet,
    category: Vec<String>,
) -> CommandResult<()> {
    save_into(&library(&state.paths), &snippet, &category)
}

fn save_into(library: &Library, snippet: &Snippet, category: &[String]) -> CommandResult<()> {
    let previous = library.find(snippet.id).map(|loaded| loaded.path);
    library
        .save_replacing(snippet, category, previous.as_deref())
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn delete_snippet(state: State<'_, EditorState>, id: String) -> CommandResult<()> {
    delete_by_id(&library(&state.paths), &id)
}

fn delete_by_id(library: &Library, id: &str) -> CommandResult<()> {
    let id = parse_id(id)?;
    let loaded = library
        .find(id)
        .ok_or_else(|| "snippet not found".to_owned())?;
    library.delete(&loaded).map_err(|error| error.to_string())
}

fn parse_id(id: &str) -> CommandResult<SnippetId> {
    serde_json::from_value(serde_json::Value::String(id.to_owned()))
        .map_err(|_| format!("invalid snippet id `{id}`"))
}

/// All snippets of the library, for including one in another.
fn library_snippets(paths: &Paths) -> Vec<Snippet> {
    library(paths)
        .load()
        .snippets
        .into_iter()
        .map(|loaded| loaded.snippet)
        .collect()
}

/// Checks a snippet without saving it, including the snippets it includes.
/// Returns the problem, if any.
#[tauri::command]
pub(crate) fn validate_snippet(state: State<'_, EditorState>, snippet: Snippet) -> Option<String> {
    check_snippet(&snippet, &library_snippets(&state.paths))
}

fn check_snippet(snippet: &Snippet, library: &[Snippet]) -> Option<String> {
    if let Err(error) = snippet.validate() {
        return Some(error.to_string());
    }
    nesting::check(snippet, library)
        .err()
        .map(|error| error.to_string())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct Preview {
    plain: String,
    html: String,
    method: &'static str,
    formatted: bool,
}

/// Renders a snippet as it would be inserted now. Fields use their defaults
/// unless `fields` has a value; the clipboard shows as a marker.
#[tauri::command]
pub(crate) fn preview(
    state: State<'_, EditorState>,
    snippet: Snippet,
    fields: HashMap<String, String>,
) -> CommandResult<Preview> {
    render_preview(
        snippet,
        fields,
        date_language(&settings(&state.paths)),
        &library_snippets(&state.paths),
    )
}

/// Day and month names follow the interface language.
fn date_language(settings: &Settings) -> DateLanguage {
    let german = match settings.language {
        typlet_core::Language::De => true,
        typlet_core::Language::En => false,
        typlet_core::Language::System => typlet_platform::system_language()
            .is_some_and(|lang| lang.to_lowercase().starts_with("de")),
    };
    if german {
        DateLanguage::German
    } else {
        DateLanguage::English
    }
}

fn render_preview(
    snippet: Snippet,
    fields: HashMap<String, String>,
    date_language: DateLanguage,
    library: &[Snippet],
) -> CommandResult<Preview> {
    let mut values = fields;
    for field in nesting::form_fields(&snippet, library) {
        values.entry(field.name.clone()).or_insert_with(|| {
            field
                .default
                .clone()
                .unwrap_or_else(|| format!("‹{}›", field.label))
        });
    }
    let context = RenderContext {
        now: chrono::Local::now().naive_local(),
        clipboard: Some("📋"),
        fields: &values,
        date_language,
        snippets: library,
    };
    let rendered = render(&snippet, &context).map_err(|error| error.to_string())?;
    Ok(Preview {
        method: match rendered.insert_method(snippet.insert_mode) {
            InsertMethod::Type => "type",
            InsertMethod::Paste => "paste",
        },
        formatted: rendered.formatted,
        plain: rendered.plain,
        html: rendered.html,
    })
}

// ----- settings -------------------------------------------------------------

#[tauri::command]
pub(crate) fn load_settings(state: State<'_, EditorState>) -> CommandResult<Settings> {
    Settings::load(&state.paths.settings_file).map_err(|error| error.to_string())
}

/// Saves the settings. The background program notices the change and
/// applies it (language, hotkeys, autostart, app rules).
#[tauri::command]
pub(crate) fn save_settings(
    state: State<'_, EditorState>,
    settings: Settings,
) -> CommandResult<()> {
    // Hotkeys arrive already parsed: serde rejects invalid ones.
    settings
        .save(&state.paths.settings_file)
        .map_err(|error| error.to_string())
}

/// Checks a hotkey written by the user and returns it in normal form.
#[tauri::command]
pub(crate) fn parse_hotkey(text: String) -> CommandResult<String> {
    text.parse::<typlet_core::Hotkey>()
        .map(|hotkey| hotkey.to_string())
        .map_err(|error| error.to_string())
}

// ----- import ---------------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct ImportedView {
    trigger: String,
    label: String,
    source: String,
    warnings: Vec<typlet_import::Warning>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ImportSummary {
    files: usize,
    imported: Vec<ImportedView>,
    skipped: Vec<typlet_import::Skipped>,
    existing: Vec<String>,
    saved: usize,
}

/// Imports espanso matches from `dir` into the library. Triggers that
/// already exist are left alone.
#[tauri::command]
pub(crate) fn import_espanso(
    state: State<'_, EditorState>,
    dir: String,
) -> CommandResult<ImportSummary> {
    import_into(&library(&state.paths), Path::new(&dir))
}

fn import_into(library: &Library, dir: &Path) -> CommandResult<ImportSummary> {
    if !dir.is_dir() {
        return Err(format!("{} is not a folder", dir.display()));
    }
    let report = typlet_import::import_dir(dir);
    // The snippet that ends up with each trigger. A trigger that already
    // exists is not imported again; snippets that include it get the one
    // that is kept.
    let mut kept: HashMap<String, SnippetId> = library
        .load()
        .snippets
        .into_iter()
        .map(|l| (l.snippet.trigger, l.snippet.id))
        .collect();
    let mut relinked = HashMap::new();
    let mut new = HashSet::new();
    for item in &report.imported {
        let snippet = &item.snippet;
        match kept.get(&snippet.trigger) {
            Some(id) => {
                relinked.insert(snippet.id, *id);
            }
            None => {
                kept.insert(snippet.trigger.clone(), snippet.id);
                new.insert(snippet.id);
            }
        }
    }
    let mut summary = ImportSummary {
        files: report.files,
        imported: Vec::new(),
        skipped: report.skipped,
        existing: Vec::new(),
        saved: 0,
    };
    for mut item in report.imported {
        if !new.contains(&item.snippet.id) {
            summary.existing.push(item.snippet.trigger);
            continue;
        }
        nesting::relink(&mut item.snippet.content, &relinked);
        library
            .save(&item.snippet, &item.category)
            .map_err(|error| error.to_string())?;
        summary.saved += 1;
        summary.imported.push(ImportedView {
            trigger: item.snippet.trigger,
            label: item.snippet.label,
            source: item.source,
            warnings: item.warnings,
        });
    }
    Ok(summary)
}

// ----- folders and the background program -------------------------------------

#[tauri::command]
pub(crate) fn open_folder(state: State<'_, EditorState>, which: String) {
    let paths = &state.paths;
    let dir = match which.as_str() {
        "library" => paths.library(&settings(paths)),
        _ => paths.data_dir.clone(),
    };
    crate::os::open_folder(&dir);
}

/// Where `open_link` leads. The interface names a page; it never passes a
/// URL, so it cannot open arbitrary addresses.
fn link(which: &str) -> Option<&'static str> {
    match which {
        "download" => Some("https://github.com/typlet-app/typlet/releases/latest"),
        _ => None,
    }
}

/// Opens one of Typlet's web pages in the browser.
#[tauri::command]
pub(crate) fn open_link(which: String) -> CommandResult<()> {
    let url = link(&which).ok_or_else(|| format!("unknown link: {which}"))?;
    crate::os::open_url(url);
    Ok(())
}

/// Starts the background program if it is not running.
#[tauri::command]
pub(crate) fn start_background(state: State<'_, EditorState>) -> bool {
    crate::os::ensure_background(&state.paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typlet_core::Document;

    fn snippet(trigger: &str) -> Snippet {
        Snippet::new(trigger, trigger, Document::from_plain_text("text"))
    }

    #[test]
    fn links_are_a_fixed_list() {
        assert_eq!(
            link("download"),
            Some("https://github.com/typlet-app/typlet/releases/latest")
        );
        assert_eq!(link("https://example.com"), None);
    }

    #[test]
    fn start_arguments() {
        let args = StartArgs::parse([
            "--open".to_owned(),
            "abc".to_owned(),
            "--settings".to_owned(),
        ]);
        assert_eq!(args.open_snippet.as_deref(), Some("abc"));
        assert!(args.open_settings);
        assert!(!args.background_only);
        assert!(StartArgs::parse(["--autostart".to_owned()]).background_only);
        assert!(StartArgs::parse(["--check-updates".to_owned()]).check_updates);
        assert!(StartArgs::parse(["--update".to_owned()]).open_update);
    }

    #[test]
    fn saving_moves_between_categories_and_deletes_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path());
        let s = snippet(":a");
        save_into(&library, &s, &[]).unwrap();
        save_into(&library, &s, &["Work".to_owned()]).unwrap();
        let view = load_view(&library);
        assert_eq!(view.snippets.len(), 1);
        assert_eq!(view.snippets[0].category, ["Work"]);
        assert!(save_into(&library, &s, &["../evil".to_owned()]).is_err());

        delete_by_id(&library, &s.id.to_string()).unwrap();
        assert!(load_view(&library).snippets.is_empty());
        assert!(delete_by_id(&library, "not-an-id").is_err());
    }

    #[test]
    fn conflicts_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path());
        let short = snippet(":c");
        let long = snippet(":cc");
        save_into(&library, &short, &[]).unwrap();
        save_into(&library, &long, &[]).unwrap();
        assert_eq!(
            load_view(&library).conflicts,
            vec![ConflictView::Shadowed {
                by: short.id.to_string(),
                shadowed: long.id.to_string()
            }]
        );
    }

    #[test]
    fn preview_uses_defaults_and_reports_method() {
        let mut s = snippet(":p");
        s.content = Document::from_plain_text("eins\nzwei");
        let preview = render_preview(s, HashMap::new(), DateLanguage::English, &[]).unwrap();
        assert_eq!(preview.plain, "eins\nzwei");
        assert_eq!(preview.method, "paste");
    }

    #[test]
    fn included_snippets_are_previewed_and_checked() {
        let mut signature = snippet(":sig");
        signature.content = Document::from_plain_text("Anna");
        let mut letter = snippet(":brief");
        letter.content = Document {
            blocks: vec![typlet_core::Block::Paragraph {
                content: vec![
                    typlet_core::Inline::text("Grüße, "),
                    typlet_core::Inline::Snippet { id: signature.id },
                ],
            }],
        };
        let library = vec![signature.clone()];
        let preview = render_preview(
            letter.clone(),
            HashMap::new(),
            DateLanguage::German,
            &library,
        )
        .unwrap();
        assert_eq!(preview.plain, "Grüße, Anna");
        assert_eq!(check_snippet(&letter, &library), None);
        assert!(check_snippet(&letter, &[]).is_some());
    }

    #[test]
    fn import_skips_existing_triggers() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("lib"));
        save_into(&library, &snippet(":hi"), &[]).unwrap();
        let espanso = dir.path().join("espanso").join("match");
        std::fs::create_dir_all(&espanso).unwrap();
        std::fs::write(
            espanso.join("base.yml"),
            "matches:\n  - trigger: ':hi'\n    replace: Hi\n  - trigger: ':new'\n    replace: New\n",
        )
        .unwrap();
        let summary = import_into(&library, &dir.path().join("espanso")).unwrap();
        assert_eq!(summary.saved, 1);
        assert_eq!(summary.existing, [":hi"]);
        let view = load_view(&library);
        let new = view
            .snippets
            .iter()
            .find(|s| s.snippet.trigger == ":new")
            .unwrap();
        assert_eq!(new.category, ["espanso", "base"]);
    }

    #[test]
    fn imported_nested_matches_use_existing_snippets() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("lib"));
        let mut sig = snippet(":sig");
        sig.content = Document::from_plain_text("Anna");
        save_into(&library, &sig, &[]).unwrap();
        let espanso = dir.path().join("espanso").join("match");
        std::fs::create_dir_all(&espanso).unwrap();
        std::fs::write(
            espanso.join("base.yml"),
            "matches:\n  - trigger: ':mail'\n    replace: 'Hi {{s}}'\n    vars:\n      - name: s\n        type: match\n        params:\n          trigger: ':sig'\n  - trigger: ':sig'\n    replace: Bob\n",
        )
        .unwrap();
        let summary = import_into(&library, &dir.path().join("espanso")).unwrap();
        assert_eq!(summary.existing, [":sig"]);
        let view = load_view(&library);
        let mail = view
            .snippets
            .iter()
            .find(|s| s.snippet.trigger == ":mail")
            .unwrap();
        let all: Vec<Snippet> = view.snippets.iter().map(|s| s.snippet.clone()).collect();
        let preview = render_preview(
            mail.snippet.clone(),
            HashMap::new(),
            DateLanguage::English,
            &all,
        )
        .unwrap();
        assert_eq!(preview.plain, "Hi Anna");
    }
}
