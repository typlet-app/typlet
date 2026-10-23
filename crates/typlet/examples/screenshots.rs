// SPDX-License-Identifier: GPL-3.0-or-later
//! Renders the windows of the background program with sample data into PNG
//! files for the documentation.
//!
//! ```sh
//! cargo run -p typlet --example screenshots -- docs/images
//! ```
//!
//! Rendering uses Slint's software renderer, so no display is needed and the
//! images look the same on every machine with the same fonts.

#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr
)]

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, ModelRc, PhysicalSize, Rgb8Pixel, SharedString, VecModel};

mod ui {
    #![allow(
        missing_docs,
        unreachable_pub,
        clippy::all,
        clippy::pedantic,
        clippy::todo
    )]
    slint::include_modules!();
}
use ui::*;

/// Device pixels per logical pixel, for crisp images on high-DPI screens.
const SCALE: f32 = 2.0;

struct ScreenshotPlatform {
    last: RefCell<Option<Rc<MinimalSoftwareWindow>>>,
}

impl Platform for ScreenshotPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        *self.last.borrow_mut() = Some(Rc::clone(&window));
        Ok(window)
    }
}

thread_local! {
    static PLATFORM: RefCell<Option<Rc<ScreenshotPlatform>>> = const { RefCell::new(None) };
}

struct Forward(Rc<ScreenshotPlatform>);

impl Platform for Forward {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        self.0.create_window_adapter()
    }
}

fn last_window() -> Rc<MinimalSoftwareWindow> {
    PLATFORM.with(|p| p.borrow().as_ref().unwrap().last.borrow().clone().unwrap())
}

/// Renders the most recently created window at `width` × `height` logical
/// pixels and writes it to `path`.
fn capture(component: &impl ComponentHandle, width: u32, height: u32, path: &Path) {
    let window = last_window();
    window.dispatch_event(WindowEvent::ScaleFactorChanged {
        scale_factor: SCALE,
    });
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let size = PhysicalSize::new(
        (width as f32 * SCALE) as u32,
        (height as f32 * SCALE) as u32,
    );
    window.set_size(size);
    component.show().unwrap();
    slint::platform::update_timers_and_animations();
    let mut buffer = vec![Rgb8Pixel::default(); (size.width * size.height) as usize];
    window.request_redraw();
    window.draw_if_needed(|renderer| {
        renderer.render(&mut buffer, size.width as usize);
    });
    component.hide().unwrap();

    let file = File::create(path).unwrap();
    let mut encoder = png::Encoder::new(BufWriter::new(file), size.width, size.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let bytes: Vec<u8> = buffer.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    writer.write_image_data(&bytes).unwrap();
    println!("wrote {}", path.display());
}

fn strings(items: &[&str]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .iter()
            .map(|s| SharedString::from(*s))
            .collect::<Vec<_>>(),
    ))
}

struct Texts {
    lang: &'static str,
    results: Vec<(&'static str, &'static str, &'static str, bool)>,
    query: &'static str,
    form_label: &'static str,
    form_fields: Vec<(&'static str, FieldKind, &'static str, Vec<&'static str>)>,
    selection: &'static str,
    new_trigger: &'static str,
    examples: Vec<(&'static str, &'static str)>,
}

fn german() -> Texts {
    Texts {
        lang: "de",
        query: "gr",
        results: vec![
            ("Grußformel", "Mit freundlichen Grüßen", ":mfg", false),
            (
                "E-Mail-Signatur (formatiert)",
                "Viele Grüße Max Mustermann Musterfirma GmbH",
                ":sig",
                false,
            ),
            (
                "Rückruf-Notiz (Formular)",
                "Rückruf vom 23.10.2026, 09:41 • Anrufer: ‹anrufer› • Anliegen: ‹anliegen›",
                ":rr",
                true,
            ),
        ],
        form_label: "Rückruf-Notiz",
        form_fields: vec![
            (
                "Wer hat angerufen?",
                FieldKind::Text,
                "Frau Schmidt",
                vec![],
            ),
            (
                "Anliegen",
                FieldKind::Choice,
                "Termin",
                vec!["Angebot", "Reklamation", "Termin"],
            ),
            ("Rückruf erwünscht?", FieldKind::YesNo, "Ja", vec![]),
        ],
        selection: "Vielen Dank für Ihre Anfrage. Wir melden uns innerhalb von zwei Werktagen bei Ihnen.",
        new_trigger: ":anfrage",
        examples: vec![
            (":mfg", "Grußformel"),
            (":datum", "Heutiges Datum"),
            (":adr", "Meine Adresse (bitte anpassen)"),
            (":sig", "E-Mail-Signatur (formatiert)"),
            (":rr", "Rückruf-Notiz (Formular)"),
        ],
    }
}

fn english() -> Texts {
    Texts {
        lang: "en",
        query: "re",
        results: vec![
            ("Closing", "Best regards", ":br", false),
            (
                "Email signature (formatted)",
                "Kind regards Jane Doe Example Ltd",
                ":sig",
                false,
            ),
            (
                "Call note (form)",
                "Call on October 23, 2026, 09:41 • Caller: ‹caller› • Topic: ‹topic›",
                ":call",
                true,
            ),
        ],
        form_label: "Call note",
        form_fields: vec![
            ("Who called?", FieldKind::Text, "Ms Smith", vec![]),
            (
                "Topic",
                FieldKind::Choice,
                "Appointment",
                vec!["Quote", "Complaint", "Appointment"],
            ),
            ("Call back?", FieldKind::YesNo, "Yes", vec![]),
        ],
        selection: "Thank you for your request. We will get back to you within two working days.",
        new_trigger: ":request",
        examples: vec![
            (":br", "Closing"),
            (":date", "Today's date"),
            (":addr", "My address (please edit)"),
            (":sig", "Email signature (formatted)"),
            (":call", "Call note (form)"),
        ],
    }
}

fn render_all(texts: &Texts, out: &Path) {
    std::fs::create_dir_all(out).unwrap();

    let search = SearchWindow::new().unwrap();
    slint::select_bundled_translation(texts.lang).unwrap();
    let items: Vec<SnippetItem> = texts
        .results
        .iter()
        .map(|(label, preview, trigger, form)| SnippetItem {
            id: SharedString::new(),
            trigger: (*trigger).into(),
            label: (*label).into(),
            preview: (*preview).into(),
            has_form: *form,
        })
        .collect();
    search.set_results(ModelRc::new(VecModel::from(items)));
    search.set_query(texts.query.into());
    capture(&search, 600, 440, &out.join("search.png"));

    let form = FormWindow::new().unwrap();
    let fields: Vec<FormField> = texts
        .form_fields
        .iter()
        .map(|(label, kind, value, options)| FormField {
            label: (*label).into(),
            kind: *kind,
            value: (*value).into(),
            options: strings(options),
        })
        .collect();
    form.set_snippet_label(texts.form_label.into());
    form.set_trigger(texts.results[2].2.into());
    form.set_fields(ModelRc::new(VecModel::from(fields)));
    capture(&form, 460, 400, &out.join("form.png"));

    let create = CreateWindow::new().unwrap();
    create.set_text(texts.selection.into());
    create.set_trigger(texts.new_trigger.into());
    create.set_problem(TriggerProblem::None);
    capture(&create, 480, 470, &out.join("new-snippet.png"));

    let setup = SetupWindow::new().unwrap();
    let examples: Vec<ExampleItem> = texts
        .examples
        .iter()
        .map(|(trigger, label)| ExampleItem {
            trigger: (*trigger).into(),
            label: (*label).into(),
        })
        .collect();
    setup.set_examples(ModelRc::new(VecModel::from(examples)));
    setup.set_keyboard_ok(true);
    setup.set_typing_ok(true);
    setup.set_focus_ok(true);
    setup.set_test_trigger(texts.examples[0].0.into());
    setup.set_search_shortcut(
        if texts.lang == "de" {
            "Strg+Umschalt+Leertaste"
        } else {
            "Ctrl+Shift+Space"
        }
        .into(),
    );
    capture(&setup, 560, 520, &out.join("setup-welcome.png"));
    setup.set_page(2);
    setup.set_test_text(
        if texts.lang == "de" {
            "Mit freundlichen Grüßen"
        } else {
            "Best regards"
        }
        .into(),
    );
    setup.set_test_passed(true);
    capture(&setup, 560, 520, &out.join("setup-test.png"));

    let diagnostics = DiagnosticsWindow::new().unwrap();
    diagnostics.set_keyboard_ok(true);
    diagnostics.set_typing_ok(true);
    diagnostics.set_focus_ok(true);
    diagnostics.set_snippet_count(5);
    diagnostics.set_version(env!("CARGO_PKG_VERSION").into());
    diagnostics.set_attempts(ModelRc::new(VecModel::from(vec![
        AttemptItem {
            time: "09:41:12".into(),
            app: "outlook.exe".into(),
            kind: AttemptKind::PastedFormatted,
            detail: SharedString::new(),
        },
        AttemptItem {
            time: "09:40:03".into(),
            app: "ms-teams.exe".into(),
            kind: AttemptKind::Typed,
            detail: SharedString::new(),
        },
        AttemptItem {
            time: "09:38:47".into(),
            app: "keepass.exe".into(),
            kind: AttemptKind::SkippedPassword,
            detail: SharedString::new(),
        },
    ])));
    diagnostics.set_report("Typlet 0.1.0 on windows x86_64\nkeyboard input: yes | inserting: yes | app detection: yes | paused: no".into());
    capture(&diagnostics, 620, 600, &out.join("diagnostics.png"));
}

fn main() {
    let out: PathBuf = std::env::args()
        .nth(1)
        .map_or_else(|| "docs/images".into(), PathBuf::from);
    let platform = Rc::new(ScreenshotPlatform {
        last: RefCell::new(None),
    });
    PLATFORM.with(|p| *p.borrow_mut() = Some(Rc::clone(&platform)));
    slint::platform::set_platform(Box::new(Forward(platform))).unwrap();

    render_all(&english(), &out.join("en"));
    render_all(&german(), &out.join("de"));
}
