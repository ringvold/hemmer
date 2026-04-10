use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

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

/// Prepend a base URL to all relative paths in HTML attributes,
/// inline `style` attributes (CSS `url()`), and `<style>` tag contents.
pub fn process(html: &str, base_url: &str) -> Result<String, Error> {
    let base = base_url.trim_end_matches('/');

    let mut handlers = Vec::new();

    // 1. HTML attributes
    for &(tag, attr) in URL_ATTRS {
        let base = base.to_string();
        let attr = attr.to_string();
        let selector = format!("{tag}[{attr}]");

        handlers.push(element!(selector, move |el| {
            if let Some(val) = el.get_attribute(&attr) {
                if is_relative(&val) {
                    let resolved = join_url(&base, &val);
                    el.set_attribute(&attr, &resolved)
                        .map_err(|e| format!("{e}"))?;
                }
            }
            Ok(())
        }));
    }

    // 2. Inline style attributes — resolve url() inside
    {
        let base = base.to_string();
        handlers.push(element!("[style]", move |el| {
            if let Some(style) = el.get_attribute("style") {
                let resolved = resolve_css_urls(&style, &base);
                if resolved != style {
                    el.set_attribute("style", &resolved)
                        .map_err(|e| format!("{e}"))?;
                }
            }
            Ok(())
        }));
    }

    // 3. <style> tag contents — resolve url() inside
    {
        let base = base.to_string();
        handlers.push(text!("style", move |chunk| {
            let css = chunk.as_str();
            let resolved = resolve_css_urls(css, &base);
            if resolved != css {
                chunk.replace(&resolved, lol_html::html_content::ContentType::Text);
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

/// Join a base URL with a relative path.
fn join_url(base: &str, path: &str) -> String {
    if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

/// Walk a CSS string and replace `url(...)` arguments with absolute URLs.
/// Preserves the original quoting style (none, single, double).
fn resolve_css_urls(css: &str, base: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut remaining = css;

    while let Some(idx) = remaining.find("url(") {
        result.push_str(&remaining[..idx]);
        let after_open = &remaining[idx + 4..];

        let Some(close_pos) = after_open.find(')') else {
            result.push_str("url(");
            remaining = after_open;
            continue;
        };

        let inner = &after_open[..close_pos];
        let after_close = &after_open[close_pos + 1..];

        let (quote, raw_url) = strip_quotes(inner.trim());

        if is_relative(raw_url) {
            let resolved = join_url(base, raw_url);
            result.push_str("url(");
            match quote {
                Some(q) => {
                    result.push(q);
                    result.push_str(&resolved);
                    result.push(q);
                }
                None => result.push_str(&resolved),
            }
            result.push(')');
        } else {
            // Leave as-is (preserve original)
            result.push_str("url(");
            result.push_str(inner);
            result.push(')');
        }

        remaining = after_close;
    }

    result.push_str(remaining);
    result
}

/// Strip surrounding quotes from a CSS url() argument.
/// Returns the quote character (if any) and the unquoted string.
fn strip_quotes(s: &str) -> (Option<char>, &str) {
    if s.len() >= 2 {
        let first = s.chars().next().unwrap();
        let last = s.chars().last().unwrap();
        if (first == '\'' || first == '"') && first == last {
            return (Some(first), &s[1..s.len() - 1]);
        }
    }
    (None, s)
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

    // ─── CSS url() resolution ──────────────────────────────────

    #[test]
    fn test_inline_style_background_image() {
        let html = r#"<div style="background-image: url(/img/bg.png)">Hi</div>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(https://example.com/img/bg.png)"));
    }

    #[test]
    fn test_inline_style_background_shorthand() {
        let html = r#"<div style="background: url('/img/bg.png') center">Hi</div>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url('https://example.com/img/bg.png')"));
    }

    #[test]
    fn test_inline_style_double_quoted_url() {
        // Note: lol_html serializes attributes with double quotes by default,
        // so any double quotes inside become &quot; entities. That's correct HTML.
        let html = r#"<div style='background: url("img/bg.png")'>Hi</div>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("https://example.com/img/bg.png"));
        // The URL got resolved; the quote escaping is lol_html's serializer choice.
    }

    #[test]
    fn test_inline_style_absolute_url_unchanged() {
        let html = r#"<div style="background: url(https://other.com/bg.png)">Hi</div>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(https://other.com/bg.png)"));
        // Should not double-prefix
        assert!(!result.contains("example.com/https"));
    }

    #[test]
    fn test_style_tag_url() {
        let html = r#"<style>.bg { background-image: url(/img/bg.png); }</style>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(/img/bg.png)") == false);
        assert!(result.contains("url(https://example.com/img/bg.png)"));
    }

    #[test]
    fn test_style_tag_font_face_src() {
        let html = r#"<style>@font-face { src: url(/fonts/foo.woff2); }</style>"#;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(https://example.com/fonts/foo.woff2)"));
    }

    #[test]
    fn test_style_tag_multiple_urls() {
        let html = r##"<style>
            .a { background: url(/a.png); }
            .b { background: url(/b.png); }
        </style>"##;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(https://example.com/a.png)"));
        assert!(result.contains("url(https://example.com/b.png)"));
    }

    #[test]
    fn test_data_url_unchanged() {
        let html = r##"<div style="background: url(data:image/png;base64,iVBOR)">Hi</div>"##;
        let result = process(html, "https://example.com").unwrap();
        assert!(result.contains("url(data:image/png;base64,iVBOR)"));
    }
}
