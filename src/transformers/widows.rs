use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

/// Minimum number of words required before inserting a non-breaking space.
/// Matches the default in posthtml-widows.
const MIN_WORDS: usize = 4;

/// Attributes that mark an element for widow prevention.
/// Both names are accepted by posthtml-widows.
const TRIGGER_ATTRS: &[&str] = &["prevent-widows", "no-widows"];

/// Prevent orphaned single words on the last line of text.
///
/// Replaces the last regular space in text content with a non-breaking space
/// so the last two words stay together. Applies to elements that have either
/// the `prevent-widows` or `no-widows` attribute, which is removed from the output.
pub fn process(html: &str) -> Result<String, Error> {
    let mut handlers = Vec::new();

    for attr in TRIGGER_ATTRS {
        let attr = *attr;
        let selector = format!("[{attr}]");

        handlers.push(text!(selector.clone(), |chunk| {
            let t = chunk.as_str();
            let words: Vec<&str> = t.split_whitespace().collect();
            if words.len() >= MIN_WORDS {
                if let Some(pos) = t.rfind(' ') {
                    let result = format!("{}\u{00a0}{}", &t[..pos], &t[pos + 1..]);
                    chunk.replace(&result, lol_html::html_content::ContentType::Text);
                }
            }
            Ok(())
        }));

        handlers.push(element!(selector, move |el| {
            el.remove_attribute(attr);
            Ok(())
        }));
    }

    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: handlers,
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prevent_widows() {
        let html = r#"<p prevent-widows>This is a longer sentence here</p>"#;
        let result = process(html).unwrap();
        // Attribute should be removed
        assert!(!result.contains("prevent-widows"));
        // Last space should be nbsp
        assert!(result.contains("sentence\u{00a0}here"));
    }

    #[test]
    fn test_short_text_unchanged() {
        let html = r#"<p prevent-widows>Two words</p>"#;
        let result = process(html).unwrap();
        assert!(result.contains("Two words"));
        assert!(!result.contains("prevent-widows"));
    }

    #[test]
    fn test_no_attribute_unchanged() {
        let html = "<p>This is a longer sentence here</p>";
        let result = process(html).unwrap();
        assert!(result.contains("sentence here"));
    }

    // ─── posthtml-widows defaults ──────────────────────────────

    #[test]
    fn test_min_words_is_4() {
        // 3 words = should NOT be processed (under default minWords=4)
        let html = r#"<p prevent-widows>Three short words</p>"#;
        let result = process(html).unwrap();
        // Attribute removed
        assert!(!result.contains("prevent-widows"));
        // But no nbsp inserted (only 3 words)
        assert!(result.contains("Three short words"));
        assert!(!result.contains("\u{00a0}"));
    }

    #[test]
    fn test_4_words_processed() {
        // Exactly 4 words = should be processed
        let html = r#"<p prevent-widows>Four words right here</p>"#;
        let result = process(html).unwrap();
        assert!(result.contains("right\u{00a0}here"));
    }

    #[test]
    fn test_no_widows_alias() {
        // posthtml-widows also accepts no-widows attribute
        let html = r#"<p no-widows>This is a longer sentence here</p>"#;
        let result = process(html).unwrap();
        assert!(!result.contains("no-widows"));
        assert!(result.contains("sentence\u{00a0}here"));
    }

    #[test]
    fn test_no_widows_attribute_removed() {
        let html = r#"<p no-widows>Two words</p>"#;
        let result = process(html).unwrap();
        assert!(!result.contains("no-widows"));
    }
}
