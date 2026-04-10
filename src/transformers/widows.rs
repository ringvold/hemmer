use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

/// Minimum number of words required before inserting a non-breaking space.
const MIN_WORDS: usize = 3;

/// Prevent orphaned single words on the last line of text.
///
/// Replaces the last regular space in text content with a non-breaking space
/// so the last two words stay together. Only applies to elements with the
/// `prevent-widows` attribute, which is removed from the output.
pub fn process(html: &str) -> Result<String, Error> {
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                // Replace last space with nbsp in text nodes inside [prevent-widows]
                text!("[prevent-widows]", |chunk| {
                    let t = chunk.as_str();
                    let words: Vec<&str> = t.split_whitespace().collect();
                    if words.len() >= MIN_WORDS {
                        if let Some(pos) = t.rfind(' ') {
                            let result = format!("{}\u{00a0}{}", &t[..pos], &t[pos + 1..]);
                            chunk.replace(&result, lol_html::html_content::ContentType::Text);
                        }
                    }
                    Ok(())
                }),
                // Remove the attribute
                element!("[prevent-widows]", |el| {
                    el.remove_attribute("prevent-widows");
                    Ok(())
                }),
            ],
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
}
