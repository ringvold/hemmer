use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Attributes that contain URLs to be resolved.
const URL_ATTRS: &[(&str, &str)] = &[
    ("a", "href"),
    ("img", "src"),
    ("video", "src"),
    ("video", "poster"),
    ("source", "src"),
    ("link", "href"),
    ("td", "background"),
    ("table", "background"),
];

/// Prepend a base URL to all relative paths in HTML attributes.
pub fn process(html: &str, base_url: &str) -> Result<String, Error> {
    let base = base_url.trim_end_matches('/');

    let mut handlers = Vec::new();

    for &(tag, attr) in URL_ATTRS {
        let base = base.to_string();
        let attr = attr.to_string();
        let selector = format!("{tag}[{attr}]");

        handlers.push(element!(selector, move |el| {
            if let Some(val) = el.get_attribute(&attr) {
                if is_relative(&val) {
                    let resolved = if val.starts_with('/') {
                        format!("{base}{val}")
                    } else {
                        format!("{base}/{val}")
                    };
                    el.set_attribute(&attr, &resolved)
                        .map_err(|e| format!("{e}"))?;
                }
            }
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

/// Check if a URL is relative (not absolute, not a protocol, not a fragment, not a template).
fn is_relative(url: &str) -> bool {
    let url = url.trim();
    !url.is_empty()
        && !url.contains("://")
        && !url.starts_with("//")
        && !url.starts_with('#')
        && !url.starts_with("mailto:")
        && !url.starts_with("tel:")
        && !url.starts_with("data:")
        && !url.starts_with('{') // template expressions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_paths() {
        let html = r#"<img src="/images/logo.png"><a href="/about">About</a>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("src=\"https://example.com/images/logo.png\""));
        assert!(result.contains("href=\"https://example.com/about\""));
    }

    #[test]
    fn test_leave_absolute_urls() {
        let html = r#"<a href="https://other.com/page">Link</a>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("href=\"https://other.com/page\""));
    }

    #[test]
    fn test_leave_mailto() {
        let html = r#"<a href="mailto:test@example.com">Email</a>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("href=\"mailto:test@example.com\""));
    }

    #[test]
    fn test_resolve_no_leading_slash() {
        let html = r#"<img src="images/logo.png">"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("src=\"https://example.com/images/logo.png\""));
    }
}
