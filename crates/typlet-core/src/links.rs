// SPDX-License-Identifier: GPL-3.0-or-later
//! Typlet's web addresses, shared by the background program and the editor.
//!
//! These pages are opened in the user's browser. Typlet itself only goes
//! online in the editor, to check for updates
//! ([ADR 0015](https://github.com/typlet-app/typlet/blob/main/docs/adr/0015-automatic-updates.md)).

/// The source code and issue tracker.
pub const REPOSITORY: &str = "https://github.com/typlet-app/typlet";

/// The newest release with its installers.
pub const LATEST_RELEASE: &str = "https://github.com/typlet-app/typlet/releases/latest";

/// Where people can support Typlet's development with a donation.
pub const DONATE: &str = "https://ko-fi.com/finnbach";

/// The online user guide in the interface language.
#[must_use]
pub fn user_guide(german: bool) -> &'static str {
    if german {
        "https://github.com/typlet-app/typlet/blob/main/docs/user-guide.de.md"
    } else {
        "https://github.com/typlet-app/typlet/blob/main/docs/user-guide.md"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_https_and_point_to_the_project() {
        for url in [
            REPOSITORY,
            LATEST_RELEASE,
            DONATE,
            user_guide(true),
            user_guide(false),
        ] {
            assert!(url.starts_with("https://"), "{url}");
        }
        assert!(LATEST_RELEASE.starts_with(REPOSITORY));
        assert!(user_guide(true).starts_with(REPOSITORY));
        assert!(user_guide(true).ends_with(".de.md"));
    }
}
