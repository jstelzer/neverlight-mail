/// Render an email body to plain text for display.
///
/// Prefers text/plain when available; falls back to sanitized HTML conversion.
pub fn render_body(text_plain: Option<&str>, text_html: Option<&str>) -> String {
    html_safe_md::render_email_plain(text_plain, text_html)
}

/// Render email body as Markdown for the preview widget.
///
/// Prefers text/plain when it looks like real content — most emails include both
/// parts and the plain version is usually fine. Falls back to the sanitized
/// HTML → markdown pipeline when plain text is missing or looks like a tracking
/// stub.
pub fn render_body_markdown(text_plain: Option<&str>, text_html: Option<&str>) -> String {
    html_safe_md::render_email(text_plain, text_html)
}

/// Open a URL in the system browser.
pub fn open_link(url: &str) {
    let _ = open::that(url);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── render_body (plain text output) ──────────────────────────

    #[test]
    fn plain_text_preferred_over_html() {
        let result = render_body(Some("Hello, world"), Some("<p>Hello, world</p>"));
        assert_eq!(result, "Hello, world");
    }

    #[test]
    fn falls_back_to_html_when_no_plain() {
        let result = render_body(None, Some("<p>Hello</p>"));
        assert!(!result.is_empty());
        assert!(result.contains("Hello"));
        // Should not contain raw HTML tags
        assert!(!result.contains("<p>"));
    }

    #[test]
    fn no_content_when_both_none() {
        let result = render_body(None, None);
        assert_eq!(result, "[No displayable content]");
    }

    #[test]
    fn plain_text_returned_verbatim() {
        let input = "Line one\n\nLine two\n  indented";
        assert_eq!(render_body(Some(input), None), input);
    }

    // ── render_body_markdown ─────────────────────────────────────

    #[test]
    fn markdown_prefers_real_plain_text() {
        let plain = "Hey,\n\nThis is a real email body with enough content to pass the junk filter.\n\nCheers";
        let html = "<p>HTML version</p>";
        let result = render_body_markdown(Some(plain), Some(html));
        assert_eq!(result, plain);
    }

    #[test]
    fn markdown_skips_junk_plain_for_html() {
        // Short stub that is_junk_plain should catch
        let junk = "View online";
        let html = "<p>This is the <strong>real</strong> email content right here.</p>";
        let result = render_body_markdown(Some(junk), Some(html));
        // Should have used the HTML path, not the junk plain text
        assert_ne!(result, junk);
        assert!(result.contains("real"));
    }

    #[test]
    fn markdown_shows_junk_plain_when_no_html() {
        let junk = "View online";
        let result = render_body_markdown(Some(junk), None);
        // No HTML to fall back to, so junk is shown as-is
        assert_eq!(result, junk);
    }

    #[test]
    fn markdown_no_content_fallback() {
        assert_eq!(render_body_markdown(None, None), "[No displayable content]");
    }

    #[test]
    fn markdown_strips_tracking_pixels() {
        let html = r#"<p>Real content</p><img src="https://track.example.com/open.gif" width="1" height="1">"#;
        let result = render_body_markdown(None, Some(html));
        assert!(result.contains("Real content"));
        // img is not in the allowed tag set, should be stripped
        assert!(!result.contains("track.example.com"));
    }

    #[test]
    fn markdown_strips_layout_tables() {
        let html = r#"
            <table><tr><td>
                <p>Actual message</p>
            </td></tr></table>
        "#;
        let result = render_body_markdown(None, Some(html));
        assert!(result.contains("Actual message"));
        // table tags stripped, so no markdown table syntax
        assert!(!result.contains("|"));
    }

    #[test]
    fn markdown_preserves_links() {
        let html = r#"<p>Click <a href="https://example.com">here</a></p>"#;
        let result = render_body_markdown(None, Some(html));
        assert!(result.contains("https://example.com"));
        assert!(result.contains("here"));
    }

    #[test]
    fn markdown_preserves_formatting() {
        let html = "<p>This is <strong>bold</strong> and <em>italic</em></p>";
        let result = render_body_markdown(None, Some(html));
        assert!(result.contains("**bold**") || result.contains("__bold__"));
        assert!(result.contains("*italic*") || result.contains("_italic_"));
    }

    #[test]
    fn markdown_strips_style_and_script() {
        let html = r#"
            <style>.foo { color: red; }</style>
            <script>alert('xss')</script>
            <p>Safe content</p>
        "#;
        let result = render_body_markdown(None, Some(html));
        assert!(result.contains("Safe content"));
        assert!(!result.contains("color: red"));
        assert!(!result.contains("alert"));
    }

    // ── Real-world fixture: 1Password invoice ────────────────────
    //
    // Marketing HTML with nested layout tables, MSO conditionals,
    // inline styles, tracking pixels — the kind of email that
    // produced markdown soup before sanitization.

    const FIXTURE_PLAIN: &str =
        include_str!("../../tests/fixtures/1password_invoice_plain.txt");
    const FIXTURE_HTML: &str =
        include_str!("../../tests/fixtures/1password_invoice_html.txt");

    #[test]
    fn invoice_plain_text_not_flagged_as_junk() {
        assert!(!html_safe_md::is_junk_plain(FIXTURE_PLAIN));
    }

    #[test]
    fn invoice_prefers_plain_over_html() {
        let result = render_body_markdown(Some(FIXTURE_PLAIN), Some(FIXTURE_HTML));
        assert_eq!(result, FIXTURE_PLAIN);
    }

    #[test]
    fn invoice_html_renders_without_table_soup() {
        // Force the HTML path by passing no plain text
        let result = render_body_markdown(None, Some(FIXTURE_HTML));

        // Should contain the actual invoice content
        assert!(result.contains("63.44"));
        assert!(result.contains("Families Plan"));

        // Should NOT produce markdown table syntax from layout tables
        let pipe_count = result.matches('|').count();
        assert!(
            pipe_count < 10,
            "too many pipe chars ({pipe_count}) — layout tables leaking as markdown tables"
        );
    }

    #[test]
    fn invoice_html_strips_styles_and_mso() {
        let result = render_body_markdown(None, Some(FIXTURE_HTML));
        assert!(!result.contains("mso-table"));
        assert!(!result.contains("border-collapse"));
        assert!(!result.contains("background-color"));
    }

    #[test]
    fn invoice_html_preserves_links() {
        let result = render_body_markdown(None, Some(FIXTURE_HTML));
        assert!(result.contains("testfamily.1password.com"));
    }

    #[test]
    fn invoice_html_output_is_reasonable_size() {
        let result = render_body_markdown(None, Some(FIXTURE_HTML));
        assert!(
            result.len() < 5_000,
            "output too large ({} bytes) — likely layout cruft leaking through",
            result.len()
        );
    }
}
