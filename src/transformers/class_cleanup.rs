use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Remove `class` attributes from elements where the class names are not
/// referenced by `@media` rules in the remaining `<style>` blocks.
///
/// After CSS inlining, classes are no longer needed for styling — except
/// for media query selectors which only apply when the viewport matches.
/// This cleanup reduces email size and noise.
pub fn process(html: &str) -> Result<String, Error> {
    // 1. Collect all class names referenced inside @media blocks
    let media_classes = Rc::new(RefCell::new(HashSet::<String>::new()));
    let collector = media_classes.clone();

    let _ = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![text!("style", move |chunk| {
                let css = chunk.as_str();
                for class in extract_media_classes(css) {
                    collector.borrow_mut().insert(class);
                }
                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))?;

    // 2. Filter class attributes — keep only classes referenced in media queries
    let media_classes_final = media_classes.clone();
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("[class]", move |el| {
                if let Some(classes) = el.get_attribute("class") {
                    let kept: Vec<&str> = classes
                        .split_whitespace()
                        .filter(|c| media_classes_final.borrow().contains(*c))
                        .collect();

                    if kept.is_empty() {
                        el.remove_attribute("class");
                    } else {
                        el.set_attribute("class", &kept.join(" "))
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

/// Extract class names from `@media` rules in CSS.
/// Naive implementation: scans for `.classname` patterns inside `@media { ... }` blocks.
fn extract_media_classes(css: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = css.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c == '@' {
            // Check if this is @media
            let rest = &css[chars.peek().map(|(i, _)| *i).unwrap_or(css.len())..];
            if rest.starts_with("media") {
                // Find the matching closing brace
                if let Some(brace_pos) = rest.find('{') {
                    let after_brace = &rest[brace_pos + 1..];
                    if let Some(end) = find_matching_brace(after_brace) {
                        let media_block = &after_brace[..end];
                        result.extend(extract_class_selectors(media_block));
                    }
                }
            }
        }
    }

    result
}

/// Find the position of the matching closing brace, accounting for nesting.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract `.classname` selectors from a CSS block.
fn extract_class_selectors(css: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = css.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '.' {
            let rest = &css[i + 1..];
            let end = rest
                .find(|c: char| {
                    !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '\\'
                })
                .unwrap_or(rest.len());
            if end > 0 {
                let class = &rest[..end];
                // Strip CSS escapes for matching with HTML class names
                let unescaped = class.replace('\\', "");
                if !unescaped.is_empty() {
                    result.push(unescaped);
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_class_selectors() {
        let css = ".foo { color: red; } .bar-baz { padding: 10px; }";
        let classes = extract_class_selectors(css);
        assert!(classes.contains(&"foo".to_string()));
        assert!(classes.contains(&"bar-baz".to_string()));
    }

    #[test]
    fn test_extract_media_classes() {
        let css = r#"
            .always { color: red; }
            @media (max-width: 600px) {
                .mobile-only { display: block; }
                .responsive { width: 100%; }
            }
        "#;
        let classes = extract_media_classes(css);
        assert!(classes.contains(&"mobile-only".to_string()));
        assert!(classes.contains(&"responsive".to_string()));
        // Classes outside @media should not be included
        assert!(!classes.contains(&"always".to_string()));
    }

    #[test]
    fn test_remove_inlined_classes() {
        let html = r#"<html><head><style>.foo { color: red; }</style></head>
        <body><div class="foo">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        // .foo is not in a @media query, so the class attribute should be removed
        assert!(!result.contains("class=\"foo\""));
    }

    #[test]
    fn test_keep_media_classes() {
        let html = r#"<html><head><style>
            .static-class { color: red; }
            @media (max-width: 600px) { .mobile { display: none; } }
        </style></head>
        <body><div class="static-class mobile">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        // 'mobile' should be kept (in @media), 'static-class' should be removed from class attr
        assert!(result.contains("class=\"mobile\""));
        // The class attribute should not contain static-class anymore
        assert!(!result.contains("class=\"static-class mobile\""));
        assert!(!result.contains("class=\"mobile static-class\""));
    }

    #[test]
    fn test_no_style_block() {
        let html = r#"<div class="foo">Hi</div>"#;
        let result = process(html).unwrap();
        // No <style> block means no media classes, so all classes are removed
        assert!(!result.contains("class="));
    }
}
