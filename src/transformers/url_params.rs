use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Append query parameters to all absolute URLs in `<a>` tags.
/// Typically used for UTM tracking parameters.
pub fn process(html: &str, params: &[(String, String)]) -> Result<String, Error> {
    if params.is_empty() {
        return Ok(html.to_string());
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("a[href]", move |el| {
                if let Some(href) = el.get_attribute("href") {
                    if is_trackable(&href) {
                        let separator = if href.contains('?') { "&" } else { "?" };
                        el.set_attribute("href", &format!("{href}{separator}{query}"))
                            .map_err(|e| format!("{e}"))?;
                    }
                }
                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

/// Only add params to absolute HTTP(S) URLs.
fn is_trackable(url: &str) -> bool {
    let url = url.trim();
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_utm_params() {
        let html = r#"<a href="https://example.com/page">Link</a>"#;
        let params = vec![
            ("utm_source".into(), "email".into()),
            ("utm_medium".into(), "newsletter".into()),
        ];
        let result = process(html, &params).unwrap();
        assert!(result.contains("href=\"https://example.com/page?utm_source=email&utm_medium=newsletter\""));
    }

    #[test]
    fn test_existing_query_string() {
        let html = r#"<a href="https://example.com?foo=bar">Link</a>"#;
        let params = vec![("utm_source".into(), "email".into())];
        let result = process(html, &params).unwrap();
        assert!(result.contains("?foo=bar&utm_source=email"));
    }

    #[test]
    fn test_skip_mailto() {
        let html = r#"<a href="mailto:test@example.com">Email</a>"#;
        let params = vec![("utm_source".into(), "email".into())];
        let result = process(html, &params).unwrap();
        assert!(result.contains("href=\"mailto:test@example.com\""));
    }

    #[test]
    fn test_skip_relative() {
        let html = r#"<a href="/page">Link</a>"#;
        let params = vec![("utm_source".into(), "email".into())];
        let result = process(html, &params).unwrap();
        assert!(result.contains("href=\"/page\""));
    }
}
