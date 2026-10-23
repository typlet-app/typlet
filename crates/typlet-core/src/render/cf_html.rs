// SPDX-License-Identifier: GPL-3.0-or-later
//! The Windows `CF_HTML` clipboard format.
//!
//! Windows expects HTML on the clipboard wrapped in a header that states byte
//! offsets of the document and of the pasted fragment. Getting these offsets
//! wrong makes applications paste nothing or garbage, so the wrapping is pure
//! and tested here rather than in the platform layer.
//!
//! Reference: "HTML Clipboard Format" in the Win32 documentation.

const PREFIX: &str = "<html>\r\n<body>\r\n<!--StartFragment-->";
const SUFFIX: &str = "<!--EndFragment-->\r\n</body>\r\n</html>";

/// Wraps an HTML fragment in the `CF_HTML` envelope. All offsets are byte
/// offsets into the returned UTF-8 string.
#[must_use]
pub fn wrap_cf_html(fragment: &str) -> String {
    let header_len = header(0, 0, 0, 0).len();
    let start_html = header_len;
    let start_fragment = start_html + PREFIX.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + SUFFIX.len();

    let mut out = header(start_html, end_html, start_fragment, end_fragment);
    out.push_str(PREFIX);
    out.push_str(fragment);
    out.push_str(SUFFIX);
    out
}

fn header(
    start_html: usize,
    end_html: usize,
    start_fragment: usize,
    end_fragment: usize,
) -> String {
    format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_fragment:010}\r\nEndFragment:{end_fragment:010}\r\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offset(cf: &str, key: &str) -> usize {
        let line = cf
            .lines()
            .find(|line| line.starts_with(key))
            .unwrap_or_else(|| panic!("missing {key}"));
        line[key.len() + 1..].parse().unwrap()
    }

    #[test]
    fn offsets_point_at_the_right_bytes() {
        let fragment = r#"<p style="margin:0"><strong>Grüße</strong> ✓</p>"#;
        let cf = wrap_cf_html(fragment);

        let start_html = offset(&cf, "StartHTML");
        let end_html = offset(&cf, "EndHTML");
        let start_fragment = offset(&cf, "StartFragment");
        let end_fragment = offset(&cf, "EndFragment");

        assert!(cf[start_html..].starts_with("<html>"));
        assert_eq!(end_html, cf.len());
        assert_eq!(&cf[start_fragment..end_fragment], fragment);
    }
}
