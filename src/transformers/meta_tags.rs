use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

const DOCTYPE: &str = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">\n";

const CHARSET_META: &str = r#"<meta http-equiv="Content-Type" content="text/html; charset=UTF-8">"#;
const VIEWPORT_META: &str = r#"<meta name="viewport" content="width=device-width, initial-scale=1.0">"#;
const X_UA_META: &str = r#"<meta http-equiv="X-UA-Compatible" content="IE=edge">"#;
const FORMAT_DETECTION_META: &str = r#"<meta name="format-detection" content="telephone=no, date=no, address=no, email=no, url=no">"#;

/// Inject standard email meta tags into `<head>` if missing.
///
/// Adds:
/// - DOCTYPE (XHTML 1.0 Transitional, the email standard)
/// - charset (UTF-8)
/// - viewport (responsive)
/// - X-UA-Compatible (IE edge mode)
/// - format-detection (prevent iOS auto-linking)
pub fn process(html: &str) -> Result<String, Error> {
    let has_doctype = html.trim_start().to_lowercase().starts_with("<!doctype");
    let lower = html.to_lowercase();

    let needs_charset = !lower.contains("charset=");
    let needs_viewport = !lower.contains("name=\"viewport\"") && !lower.contains("name='viewport'");
    let needs_x_ua = !lower.contains("x-ua-compatible");
    let needs_format_detection = !lower.contains("name=\"format-detection\"")
        && !lower.contains("name='format-detection'");

    let mut to_inject = String::new();
    if needs_charset {
        to_inject.push_str(CHARSET_META);
    }
    if needs_viewport {
        to_inject.push_str(VIEWPORT_META);
    }
    if needs_x_ua {
        to_inject.push_str(X_UA_META);
    }
    if needs_format_detection {
        to_inject.push_str(FORMAT_DETECTION_META);
    }

    let needs_head_inject = !to_inject.is_empty();

    let result = if needs_head_inject {
        rewrite_str(
            html,
            RewriteStrSettings {
                element_content_handlers: vec![element!("head", |el| {
                    el.prepend(&to_inject, lol_html::html_content::ContentType::Html);
                    Ok(())
                })],
                ..RewriteStrSettings::new()
            },
        )
        .map_err(|e| Error::HtmlRewrite(e.to_string()))?
    } else {
        html.to_string()
    };

    if !has_doctype {
        Ok(format!("{DOCTYPE}{result}"))
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_doctype() {
        let html = "<html><head></head><body></body></html>";
        let result = process(html).unwrap();
        assert!(result.starts_with("<!DOCTYPE"));
    }

    #[test]
    fn test_keep_existing_doctype() {
        let html = "<!DOCTYPE html><html><head></head><body></body></html>";
        let result = process(html).unwrap();
        // Should not have two doctypes
        assert_eq!(result.matches("DOCTYPE").count(), 1);
    }

    #[test]
    fn test_inject_charset() {
        let html = "<html><head></head><body></body></html>";
        let result = process(html).unwrap();
        assert!(result.contains("charset=UTF-8"));
    }

    #[test]
    fn test_inject_viewport() {
        let html = "<html><head></head><body></body></html>";
        let result = process(html).unwrap();
        assert!(result.contains("name=\"viewport\""));
    }

    #[test]
    fn test_keep_existing_charset() {
        let html = r#"<html><head><meta charset="ISO-8859-1"></head><body></body></html>"#;
        let result = process(html).unwrap();
        // Should keep existing charset, not add UTF-8
        assert!(result.contains("ISO-8859-1"));
        assert!(!result.contains("UTF-8"));
    }

    #[test]
    fn test_inject_format_detection() {
        let html = "<html><head></head><body></body></html>";
        let result = process(html).unwrap();
        assert!(result.contains("format-detection"));
    }
}
