// SPDX-License-Identifier: GPL-3.0-or-later
//! Import of espanso match files.
//!
//! espanso stores snippets ("matches") in YAML files below
//! `<config>/match/`. This module reads them and converts every trigger to
//! a Typlet snippet. Supported:
//!
//! | espanso | Typlet |
//! |---|---|
//! | `replace`, `markdown`, `html` | text with formatting |
//! | `$\|$` | cursor position |
//! | `date` variable (`format`, `offset` in seconds) | date placeholder |
//! | `clipboard` variable | clipboard placeholder |
//! | `echo` variable | fixed text |
//! | `choice` variable, `form`, `form` variable | form fields |
//! | `label` | name |
//! | `global_vars` | resolved in every file |
//!
//! Skipped with a reason: `regex` triggers, images, `shell`, `script`,
//! `random` and other variable types.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use typlet_core::{FieldDef, FieldKind, Inline, Snippet};

use crate::rich::{self, DocBuilder, Placeholder};
use crate::{ImportReport, Imported, SkipReason, Skipped, Warning};

const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct MatchFile {
    matches: Vec<Match>,
    global_vars: Vec<Var>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Match {
    trigger: Option<String>,
    triggers: Vec<String>,
    regex: Option<String>,
    replace: Option<String>,
    html: Option<String>,
    markdown: Option<String>,
    form: Option<String>,
    form_fields: HashMap<String, FieldSpec>,
    image_path: Option<String>,
    vars: Vec<Var>,
    label: Option<String>,
    word: bool,
    propagate_case: bool,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct Var {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    params: Params,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct Params {
    format: Option<String>,
    offset: Option<i64>,
    echo: Option<Scalar>,
    layout: Option<String>,
    fields: HashMap<String, FieldSpec>,
    values: Option<Values>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct FieldSpec {
    #[serde(rename = "type")]
    kind: Option<String>,
    values: Option<Values>,
    default: Option<Scalar>,
    multiline: bool,
}

/// espanso accepts choice values as a list or as one line per value.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Values {
    List(Vec<Scalar>),
    Lines(String),
}

impl Values {
    fn to_vec(&self) -> Vec<String> {
        match self {
            Self::List(items) => items.iter().map(Scalar::text).collect(),
            Self::Lines(text) => text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect(),
        }
    }
}

/// A YAML scalar that is used as text.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Scalar {
    Text(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

impl Scalar {
    fn text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Integer(n) => n.to_string(),
            Self::Float(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
        }
    }
}

/// Converts the matches of one file. `source` names the file in the
/// report; the category is derived from it.
#[must_use]
pub fn parse_file(text: &str, source: &str) -> ImportReport {
    let mut report = ImportReport {
        files: 1,
        ..ImportReport::default()
    };
    match serde_saphyr::from_str::<MatchFile>(text) {
        Ok(file) => {
            let globals = file.global_vars.clone();
            convert_file(&file, source, &globals, &mut report);
        }
        Err(error) => report.skipped.push(unreadable(source, &error.to_string())),
    }
    report
}

/// Imports every match file below an espanso configuration folder (or its
/// `match` sub-folder). Files whose name starts with `_` are skipped, as
/// espanso only loads them when another file imports them.
#[must_use]
pub fn import_dir(dir: &Path) -> ImportReport {
    let root = if dir.join("match").is_dir() {
        dir.join("match")
    } else {
        dir.to_path_buf()
    };
    let mut paths = Vec::new();
    collect_yaml(&root, &mut paths);
    paths.sort();

    let mut report = ImportReport::default();
    let mut files = Vec::new();
    for path in paths {
        let source = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        report.files += 1;
        match fs::read_to_string(&path) {
            Ok(text) => match serde_saphyr::from_str::<MatchFile>(&text) {
                Ok(file) => files.push((source, file)),
                Err(error) => report.skipped.push(unreadable(&source, &error.to_string())),
            },
            Err(error) => report.skipped.push(unreadable(&source, &error.to_string())),
        }
    }
    // Global variables apply to every file.
    let globals: Vec<Var> = files
        .iter()
        .flat_map(|(_, file)| file.global_vars.iter().cloned())
        .collect();
    for (source, file) in &files {
        convert_file(file, source, &globals, &mut report);
    }
    report
}

fn collect_yaml(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_yaml(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
        {
            out.push(path);
        }
    }
}

fn unreadable(source: &str, error: &str) -> Skipped {
    Skipped {
        source: source.to_owned(),
        trigger: String::new(),
        reason: SkipReason::Unreadable(error.lines().next().unwrap_or_default().to_owned()),
    }
}

fn category(source: &str) -> Vec<String> {
    let without_ext = source.rsplit_once('.').map_or(source, |(stem, _)| stem);
    std::iter::once("espanso".to_owned())
        .chain(
            without_ext
                .split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_owned),
        )
        .collect()
}

fn convert_file(file: &MatchFile, source: &str, globals: &[Var], report: &mut ImportReport) {
    for entry in &file.matches {
        let triggers: Vec<&String> = entry.trigger.iter().chain(&entry.triggers).collect();
        if entry.regex.is_some() {
            report.skipped.push(Skipped {
                source: source.to_owned(),
                trigger: entry.regex.clone().unwrap_or_default(),
                reason: SkipReason::Regex,
            });
            continue;
        }
        for trigger in triggers {
            match convert_match(entry, trigger, globals) {
                Ok((snippet, warnings)) => report.imported.push(Imported {
                    snippet,
                    source: source.to_owned(),
                    category: category(source),
                    warnings,
                }),
                Err(reason) => report.skipped.push(Skipped {
                    source: source.to_owned(),
                    trigger: trigger.clone(),
                    reason,
                }),
            }
        }
    }
}

fn convert_match(
    entry: &Match,
    trigger: &str,
    globals: &[Var],
) -> Result<(Snippet, Vec<Warning>), SkipReason> {
    if entry.image_path.is_some() {
        return Err(SkipReason::Image);
    }
    if let Err(error) = typlet_core::snippet::validate_trigger(trigger) {
        return Err(SkipReason::InvalidTrigger(error.to_string()));
    }

    // Match variables override global ones with the same name.
    let mut vars: HashMap<&str, &Var> = globals.iter().map(|v| (v.name.as_str(), v)).collect();
    vars.extend(entry.vars.iter().map(|v| (v.name.as_str(), v)));

    let fields: RefCell<Vec<FieldDef>> = RefCell::new(Vec::new());
    let rounded = Cell::new(false);
    let resolve = |name: &str| -> Result<Placeholder, SkipReason> {
        if let Some((form, field)) = name.split_once('.') {
            let var = vars
                .get(form)
                .filter(|v| v.kind == "form")
                .ok_or_else(|| SkipReason::UnsupportedVariable(name.to_owned()))?;
            add_field(&fields, field, var.params.fields.get(field));
            return Ok(Placeholder::Field(field.to_owned()));
        }
        let var = vars
            .get(name)
            .ok_or_else(|| SkipReason::UnsupportedVariable(name.to_owned()))?;
        match var.kind.as_str() {
            "date" => {
                let seconds = var.params.offset.unwrap_or(0);
                if seconds % SECONDS_PER_DAY != 0 {
                    rounded.set(true);
                }
                #[allow(clippy::cast_possible_truncation)] // whole days fit easily
                let offset_days = (seconds as f64 / SECONDS_PER_DAY as f64).round() as i64;
                Ok(Placeholder::Date {
                    format: var.params.format.clone().unwrap_or_else(|| "%x".to_owned()),
                    offset_days,
                })
            }
            "clipboard" => Ok(Placeholder::Clipboard),
            "echo" => Ok(Placeholder::Text(
                var.params
                    .echo
                    .as_ref()
                    .map(Scalar::text)
                    .unwrap_or_default(),
            )),
            "choice" => {
                let spec = FieldSpec {
                    kind: Some("choice".to_owned()),
                    values: var.params.values.clone(),
                    ..FieldSpec::default()
                };
                add_field(&fields, name, Some(&spec));
                Ok(Placeholder::Field(name.to_owned()))
            }
            other => Err(SkipReason::UnsupportedVariable(other.to_owned())),
        }
    };

    let is_form = entry.form.is_some();
    let mut builder = DocBuilder::new(&resolve, is_form);
    let source_text = if let Some(text) = &entry.form {
        rich::from_plain(&mut builder, text)?;
        text
    } else if let Some(text) = &entry.markdown {
        rich::from_markdown(&mut builder, text)?;
        text
    } else if let Some(text) = &entry.html {
        rich::from_html(&mut builder, text)?;
        text
    } else if let Some(text) = &entry.replace {
        rich::from_plain(&mut builder, text)?;
        text
    } else {
        return Err(SkipReason::NoText);
    };
    let (content, simplified) = builder.finish();
    if content.blocks.is_empty() {
        return Err(SkipReason::NoText);
    }

    // `[[name]]` fields of a top-level form.
    for inline in content.inlines() {
        if let Inline::Field { name } = inline {
            add_field(&fields, name, entry.form_fields.get(name));
        }
    }

    let label = entry
        .label
        .clone()
        .filter(|l| !l.trim().is_empty())
        .unwrap_or_else(|| label_from(source_text, trigger));
    let mut snippet = Snippet::new(trigger, label, content);
    snippet.fields = fields.into_inner();
    snippet
        .validate()
        .map_err(|error| SkipReason::Invalid(error.to_string()))?;

    let mut warnings = Vec::new();
    if entry.word {
        warnings.push(Warning::WordBoundary);
    }
    if entry.propagate_case {
        warnings.push(Warning::PropagateCase);
    }
    if simplified {
        warnings.push(Warning::FormattingSimplified);
    }
    if rounded.get() {
        warnings.push(Warning::OffsetRounded);
    }
    Ok((snippet, warnings))
}

fn add_field(fields: &RefCell<Vec<FieldDef>>, name: &str, spec: Option<&FieldSpec>) {
    let mut fields = fields.borrow_mut();
    if fields.iter().any(|f| f.name == name) {
        return;
    }
    let spec = spec.cloned().unwrap_or_default();
    let options = spec.values.as_ref().map(Values::to_vec).unwrap_or_default();
    let kind = match spec.kind.as_deref() {
        Some("choice" | "list") if !options.is_empty() => FieldKind::Choice { options },
        _ if spec.multiline => FieldKind::MultilineText,
        _ => FieldKind::Text,
    };
    fields.push(FieldDef {
        name: name.to_owned(),
        label: humanise(name),
        kind,
        default: spec.default.as_ref().map(Scalar::text),
    });
}

/// `first_name` → `First name`.
fn humanise(name: &str) -> String {
    let spaced = name.replace(['_', '-'], " ");
    let mut chars = spaced.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => name.to_owned(),
    }
}

/// A name for snippets without `label`: the first line of their text.
fn label_from(text: &str, trigger: &str) -> String {
    let line = text
        .lines()
        .map(strip_markup)
        .map(|l| l.trim().to_owned())
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    if line.is_empty() {
        return trigger.to_owned();
    }
    let mut label: String = line.chars().take(40).collect();
    if line.chars().count() > 40 {
        label = label.trim_end().to_owned();
        label.push('…');
    }
    label
}

/// Removes tags, placeholders and Markdown symbols for a readable name.
fn strip_markup(line: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in line.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            '*' | '_' | '#' | '`' if !in_tag => {}
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    let mut result = out.replace("$|$", "");
    while let (Some(start), Some(end)) = (result.find("{{"), result.find("}}")) {
        if end < start {
            break;
        }
        result.replace_range(start..end + 2, "…");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use typlet_core::Block;

    const BASE: &str = r#"
matches:
  - trigger: ":espanso"
    replace: "Hi there!"
  - triggers: [":hello", ":hi"]
    replace: "Hello $|$ world"
    label: Greeting
  - trigger: ":tomorrow"
    replace: "Due {{date}}"
    vars:
      - name: date
        type: date
        params:
          format: "%d.%m.%Y"
          offset: 86400
  - trigger: ":quote"
    replace: "> {{clipboard}}"
    vars:
      - name: clipboard
        type: clipboard
  - trigger: ":ip"
    replace: "{{output}}"
    vars:
      - name: output
        type: shell
        params:
          cmd: "curl ifconfig.me"
  - regex: ":(?P<n>\\d+)x"
    replace: "{{n}}"
  - trigger: ":bold"
    markdown: "**Important**: read [this](https://example.org)"
    word: true
  - trigger: ":sig"
    html: "<p>Best,<br><b>Jane</b></p>"
  - trigger: "bad trigger"
    replace: "spaces"
  - trigger: ":multi"
    replace: |
      line one
      line two
global_vars:
  - name: myname
    type: echo
    params:
      echo: Jane
"#;

    #[test]
    fn converts_common_matches() {
        let report = parse_file(BASE, "base.yml");
        let triggers: Vec<&str> = report
            .imported
            .iter()
            .map(|i| i.snippet.trigger.as_str())
            .collect();
        assert_eq!(
            triggers,
            [
                ":espanso",
                ":hello",
                ":hi",
                ":tomorrow",
                ":quote",
                ":bold",
                ":sig",
                ":multi"
            ]
        );
        assert_eq!(report.imported[0].category, ["espanso", "base"]);
        assert_eq!(report.imported[0].snippet.label, "Hi there!");
        assert_eq!(report.imported[1].snippet.label, "Greeting");
        assert!(
            report.imported[1]
                .snippet
                .content
                .inlines()
                .any(|i| *i == Inline::Cursor)
        );

        let tomorrow = &report.imported[3].snippet;
        assert!(tomorrow.content.inlines().any(|i| *i
            == Inline::DateTime {
                format: "%d.%m.%Y".into(),
                offset_days: 1
            }));
        assert!(report.imported[4].snippet.content.uses_clipboard());
        assert_eq!(report.imported[5].warnings, [Warning::WordBoundary]);
        assert!(report.imported[5].snippet.content.is_formatted());
        assert!(report.imported[6].snippet.content.is_formatted());
        assert_eq!(report.imported[7].snippet.content.blocks.len(), 2);

        let reasons: Vec<&SkipReason> = report.skipped.iter().map(|s| &s.reason).collect();
        assert_eq!(reasons.len(), 3);
        assert_eq!(*reasons[0], SkipReason::UnsupportedVariable("shell".into()));
        assert_eq!(*reasons[1], SkipReason::Regex);
        assert!(matches!(reasons[2], SkipReason::InvalidTrigger(_)));
    }

    #[test]
    fn converts_forms() {
        let yaml = r#"
matches:
  - trigger: ":greet"
    form: |
      Hey [[name]],
      [[text]]
    form_fields:
      name:
        default: Bob
      text:
        type: choice
        values:
          - Happy birthday!
          - Get well soon!
  - trigger: ":form2"
    replace: "Hi {{form1.name}}, you chose {{choice}}"
    vars:
      - name: form1
        type: form
        params:
          layout: "Name: [[name]]"
          fields:
            name:
              multiline: true
      - name: choice
        type: choice
        params:
          values: "red\ngreen"
"#;
        let report = parse_file(yaml, "forms.yml");
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        let greet = &report.imported[0].snippet;
        assert_eq!(greet.fields.len(), 2);
        assert_eq!(greet.fields[0].default.as_deref(), Some("Bob"));
        assert_eq!(
            greet.fields[1].kind,
            FieldKind::Choice {
                options: vec!["Happy birthday!".into(), "Get well soon!".into()]
            }
        );
        let form2 = &report.imported[1].snippet;
        assert_eq!(form2.fields[0].kind, FieldKind::MultilineText);
        assert_eq!(form2.fields[0].label, "Name");
        assert_eq!(
            form2.fields[1].kind,
            FieldKind::Choice {
                options: vec!["red".into(), "green".into()]
            }
        );
    }

    #[test]
    fn global_vars_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let matches = dir.path().join("match");
        fs::create_dir_all(matches.join("work")).unwrap();
        fs::write(
            matches.join("base.yml"),
            "global_vars:\n  - name: me\n    type: echo\n    params:\n      echo: Jane\n",
        )
        .unwrap();
        fs::write(
            matches.join("work").join("mail.yml"),
            "matches:\n  - trigger: ':me'\n    replace: 'I am {{me}}'\n",
        )
        .unwrap();
        fs::write(
            matches.join("_private.yml"),
            "matches:\n  - trigger: ':no'\n    replace: x\n",
        )
        .unwrap();
        fs::write(matches.join("broken.yml"), "matches: [").unwrap();

        let report = import_dir(dir.path());
        assert_eq!(report.files, 3);
        assert_eq!(report.imported.len(), 1);
        let me = &report.imported[0];
        assert_eq!(me.category, ["espanso", "work", "mail"]);
        assert_eq!(
            me.snippet.content.blocks,
            vec![Block::Paragraph {
                content: vec![Inline::text("I am Jane")]
            }]
        );
        assert!(matches!(
            report.skipped[0].reason,
            SkipReason::Unreadable(_)
        ));
    }

    #[test]
    fn rounds_partial_day_offsets() {
        let yaml = "matches:\n  - trigger: ':soon'\n    replace: '{{d}}'\n    vars:\n      - name: d\n        type: date\n        params:\n          format: '%H:%M'\n          offset: 3600\n";
        let report = parse_file(yaml, "x.yml");
        assert_eq!(report.imported[0].warnings, [Warning::OffsetRounded]);
    }
}
