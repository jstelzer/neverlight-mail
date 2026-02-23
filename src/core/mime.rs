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

/// Open a URL in the system browser.
pub fn open_link(url: &str) {
    let _ = open::that(url);
}
