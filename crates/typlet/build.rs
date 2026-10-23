// SPDX-License-Identifier: GPL-3.0-or-later
//! Compiles the Slint user interface and bundles its translations.

use slint_build::{CompilerConfiguration, DefaultTranslationContext};

fn main() {
    let config = CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_bundled_translations("lang")
        // Translations are keyed by the English text only, so the same text
        // is translated once even if several windows use it.
        .with_default_translation_context(DefaultTranslationContext::None);
    if let Err(error) = slint_build::compile_with_config("ui/app.slint", config) {
        panic!("failed to compile the user interface: {error}");
    }
}
