/// Render an email body to plain text for display.
///
/// Strategy:
/// 1. If text/plain is available, use it directly
/// 2. If only text/html, convert via html2text
pub fn render_body(text_plain: Option<&str>, text_html: Option<&str>) -> String {
    if let Some(plain) = text_plain {
        return plain.to_string();
    }

    if let Some(html) = text_html {
        return html_to_text(html);
    }

    "[No displayable content]".to_string()
}

/// Convert HTML email body to readable plain text.
fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 80).unwrap_or_default()
}

/// Max HTML input size before truncation (512 KB).
const MAX_HTML_BYTES: usize = 512 * 1024;
/// Max markdown output length in chars.
const MAX_MD_CHARS: usize = 200_000;

/// Render email body as Markdown for the preview widget.
///
/// Prefers text/plain when it looks like real content — most emails include both
/// parts and the plain version is usually fine. Falls back to the
/// HTML → ammonia → html2md pipeline when plain text is missing or looks like
/// a tracking stub.
pub fn render_body_markdown(text_plain: Option<&str>, text_html: Option<&str>) -> String {
    // Prefer plain text when it looks like real content
    if let Some(plain) = text_plain {
        if !plain_is_junk(plain) {
            return plain.to_string();
        }
    }

    // Fall back to sanitized HTML → markdown
    if let Some(html) = text_html {
        let html = &html[..html.len().min(MAX_HTML_BYTES)];
        let clean = clean_email_html(html);
        let mut md = html2md::parse_html(&clean);
        md.truncate(MAX_MD_CHARS);
        return md;
    }

    // Plain was junk but there's no HTML — show it anyway
    if let Some(plain) = text_plain {
        return plain.to_string();
    }

    "[No displayable content]".to_string()
}

/// Returns true if the plain-text part looks like a stub or tracking junk
/// rather than real email content.
fn plain_is_junk(s: &str) -> bool {
    let t = s.trim();
    t.is_empty() || t.len() < 40 || t.lines().count() <= 2
}

/// Strip email HTML down to semantic content only.
///
/// Marketing emails are 90% layout tables, tracking pixels, MSO conditionals,
/// and inline styles. ammonia::clean() default keeps all of that because it's
/// "safe" for browsers. But html2md faithfully converts every <table><tr><td>
/// into markdown table syntax, turning a 10-paragraph email into a monster.
///
/// We restrict ammonia to only semantic tags that html2md can meaningfully
/// convert. Text content inside stripped tags is preserved — only the tags
/// themselves are removed.
fn clean_email_html(html: &str) -> String {
    use std::collections::HashSet;
    let tags: HashSet<&str> = [
        // Block content
        "p", "br", "hr", "blockquote", "pre",
        // Headings
        "h1", "h2", "h3", "h4", "h5", "h6",
        // Inline formatting
        "b", "strong", "i", "em", "code", "s", "del", "u", "small", "sub", "sup",
        // Lists
        "ul", "ol", "li",
        // Links (ammonia will also sanitize href)
        "a",
    ]
    .iter()
    .copied()
    .collect();

    ammonia::Builder::new()
        .tags(tags)
        .clean(html)
        .to_string()
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
        // Short stub that plain_is_junk should catch
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
        // img is not in our allowed tag set, should be stripped
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

    #[test]
    fn markdown_truncates_huge_html() {
        let huge = "<p>".to_string() + &"x".repeat(MAX_HTML_BYTES + 1000) + "</p>";
        // Should not panic, just truncate
        let result = render_body_markdown(None, Some(&huge));
        assert!(result.len() <= MAX_MD_CHARS);
    }

    // ── plain_is_junk (tested via render_body_markdown) ──────────

    #[test]
    fn empty_plain_is_junk() {
        let result = render_body_markdown(Some(""), Some("<p>Fallback</p>"));
        assert!(result.contains("Fallback"));
    }

    #[test]
    fn whitespace_only_is_junk() {
        let result = render_body_markdown(Some("   \n\t  \n  "), Some("<p>Fallback</p>"));
        assert!(result.contains("Fallback"));
    }

    #[test]
    fn short_stub_is_junk() {
        let result = render_body_markdown(
            Some("Click here to view"),
            Some("<p>Full newsletter content here for your reading pleasure.</p>"),
        );
        assert!(result.contains("Full newsletter"));
    }

    #[test]
    fn multiline_real_content_not_junk() {
        let real = "Hey,\n\nJust wanted to follow up on our conversation from yesterday.\n\nLet me know what you think.\n\nThanks";
        let result = render_body_markdown(Some(real), Some("<p>HTML version</p>"));
        assert_eq!(result, real);
    }

    // ── Real-world fixture: 1Password invoice ────────────────────
    //
    // Marketing HTML with nested layout tables, MSO conditionals,
    // inline styles, tracking pixels — the kind of email that
    // produced markdown soup before ammonia stripping.

    const FIXTURE_PLAIN: &str =
        include_str!("../../tests/fixtures/1password_invoice_plain.txt");
    const FIXTURE_HTML: &str =
        include_str!("../../tests/fixtures/1password_invoice_html.txt");

    #[test]
    fn invoice_plain_text_not_flagged_as_junk() {
        // The plain part is a real invoice — should NOT be treated as junk
        assert!(!plain_is_junk(FIXTURE_PLAIN));
    }

    #[test]
    fn invoice_prefers_plain_over_html() {
        let result = render_body_markdown(Some(FIXTURE_PLAIN), Some(FIXTURE_HTML));
        // Should use plain text as-is, not the HTML path
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
        // (the pre-ammonia bug: hundreds of | cells from nested <table>)
        let pipe_count = result.matches('|').count();
        assert!(
            pipe_count < 10,
            "too many pipe chars ({pipe_count}) — layout tables leaking as markdown tables"
        );
    }

    #[test]
    fn invoice_html_strips_styles_and_mso() {
        let result = render_body_markdown(None, Some(FIXTURE_HTML));
        // CSS and MSO conditionals should be gone
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
        // 1000 lines of marketing HTML should not explode into
        // tens of thousands of chars of markdown
        assert!(
            result.len() < 5_000,
            "output too large ({} bytes) — likely layout cruft leaking through",
            result.len()
        );
    }
}
