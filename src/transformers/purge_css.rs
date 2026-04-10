use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Default safelist patterns for email-client-specific selectors that
/// should never be purged. Mirrors Maizzle's email-comb defaults.
///
/// These prefixes/patterns are matched against the start of class names.
const DEFAULT_SAFELIST_PREFIXES: &[&str] = &[
    "gmail",      // Gmail
    "apple",      // Apple Mail
    "ios",        // iOS Mail
    "outlook",    // Outlook.com
    "ox-",        // Open-Xchange
    "bloop_",     // Airmail
    "moz-",       // Thunderbird
    "Singleton",  // Apple Mail 10
    "lang",       // Language code blocks
    "edo",        // Edison
    "mail-",      // Various webmail
];

/// Special exact selectors that are also always preserved.
const DEFAULT_SAFELIST_EXACT: &[&str] = &[
    "body",
    "unused",          // Notes 8
    "data-ogs",        // Outlook.com data attribute
];

/// Remove unused CSS rules from `<style>` blocks.
///
/// Walks the HTML to collect all tag names, class names, and id values
/// in use, then walks each `<style>` block and removes rules whose selectors
/// don't match any element. Selectors matching the safelist are always kept.
pub fn process(html: &str) -> Result<String, Error> {
    // 1. Collect all selectors that are "in use" in the HTML
    let usage = collect_usage(html)?;

    // 2. Walk style blocks and filter their rules
    let usage_for_closure = usage.clone();
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![text!("style", move |chunk| {
                let css = chunk.as_str();
                let purged = purge_css_rules(css, &usage_for_closure);
                if purged != css {
                    chunk.replace(&purged, lol_html::html_content::ContentType::Text);
                }
                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

#[derive(Debug, Clone, Default)]
struct Usage {
    tags: HashSet<String>,
    classes: HashSet<String>,
    ids: HashSet<String>,
}

impl Usage {
    fn has_class(&self, class: &str) -> bool {
        self.classes.contains(class)
    }
    fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_ascii_lowercase())
    }
    fn has_id(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
}

fn collect_usage(html: &str) -> Result<Usage, Error> {
    let usage = Rc::new(RefCell::new(Usage::default()));

    let collector = usage.clone();
    let _ = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("*", move |el| {
                let mut u = collector.borrow_mut();
                u.tags.insert(el.tag_name().to_ascii_lowercase());

                if let Some(class) = el.get_attribute("class") {
                    for c in class.split_whitespace() {
                        u.classes.insert(c.to_string());
                    }
                }

                if let Some(id) = el.get_attribute("id") {
                    u.ids.insert(id);
                }

                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))?;

    Ok(Rc::try_unwrap(usage)
        .map(|c| c.into_inner())
        .unwrap_or_default())
}

/// Walk a CSS stylesheet and remove rules whose selectors don't match
/// any element in `usage`. Preserves @media blocks, @font-face, @keyframes.
///
/// If the stylesheet contains template syntax (`{{...}}` or `{%...%}`),
/// it's returned unchanged because the template markers would confuse
/// the brace-based parser.
fn purge_css_rules(css: &str, usage: &Usage) -> String {
    if css.contains("{{") || css.contains("{%") {
        return css.to_string();
    }

    let mut result = String::with_capacity(css.len());
    let mut chars = css.char_indices().peekable();

    while let Some(&(i, c)) = chars.peek() {
        // Skip leading whitespace
        if c.is_whitespace() {
            result.push(c);
            chars.next();
            continue;
        }

        // @-rules: @media, @font-face, @keyframes, @import, etc.
        if c == '@' {
            let (rule, end) = read_at_rule(&css[i..]);
            if should_keep_at_rule(&rule, usage) {
                result.push_str(&rule);
            }
            // Skip past consumed chars
            while let Some(&(idx, _)) = chars.peek() {
                if idx >= i + end {
                    break;
                }
                chars.next();
            }
            continue;
        }

        // Regular rule: selector { declarations }
        let (rule_text, selector, end) = read_rule(&css[i..]);

        if rule_text.trim().is_empty() {
            while let Some(&(idx, _)) = chars.peek() {
                if idx >= i + end {
                    break;
                }
                chars.next();
            }
            continue;
        }

        if should_keep_selector(&selector, usage) {
            result.push_str(&rule_text);
        }

        while let Some(&(idx, _)) = chars.peek() {
            if idx >= i + end {
                break;
            }
            chars.next();
        }
    }

    result
}

/// Read an @-rule starting at the given position.
/// Returns (text, end_position).
fn read_at_rule(css: &str) -> (String, usize) {
    // Find the next `{` or `;` (for `@import` etc.)
    let mut depth = 0;
    let mut end = 0;
    let mut found_brace = false;

    for (i, c) in css.char_indices() {
        match c {
            '{' => {
                depth += 1;
                found_brace = true;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            ';' if !found_brace && depth == 0 => {
                end = i + 1;
                break;
            }
            _ => {}
        }
    }

    if end == 0 {
        end = css.len();
    }

    (css[..end].to_string(), end)
}

/// Read a regular CSS rule starting at the given position.
/// Returns (full_rule_text, selector_part, end_position).
fn read_rule(css: &str) -> (String, String, usize) {
    let Some(brace_pos) = css.find('{') else {
        return (css.to_string(), String::new(), css.len());
    };

    let selector = css[..brace_pos].trim().to_string();

    // Find the matching closing brace
    let mut depth = 1;
    let mut end = brace_pos + 1;
    for (i, c) in css[brace_pos + 1..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace_pos + 1 + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    (css[..end].to_string(), selector, end)
}

/// Should this @-rule be kept?
/// @media: always keep (we'd need to recursively purge inside)
/// @font-face, @keyframes, @import, @charset, @supports, @page: always keep
fn should_keep_at_rule(rule: &str, _usage: &Usage) -> bool {
    let trimmed = rule.trim_start();
    // For @media, we could recursively purge — but for now keep all @media rules.
    // Email clients handle @media for responsive design, and the cost of
    // matching media queries is low.
    trimmed.starts_with('@')
}

/// Should this CSS rule be kept based on its selector(s)?
/// A comma-separated selector list is kept if ANY of its parts matches.
fn should_keep_selector(selector: &str, usage: &Usage) -> bool {
    if selector.is_empty() {
        return false;
    }

    // Preserve template syntax — if the selector contains template markers,
    // we can't reliably analyze it
    if selector.contains("{{") || selector.contains("{%") {
        return true;
    }

    for part in selector.split(',') {
        if selector_part_matches(part.trim(), usage) {
            return true;
        }
    }
    false
}

/// Check if a single selector matches anything in usage.
/// Naive but effective: scan for class, id, and tag references in the
/// rightmost compound selector (which determines what the rule actually targets).
fn selector_part_matches(part: &str, usage: &Usage) -> bool {
    if part.is_empty() {
        return false;
    }

    // Take the rightmost compound selector (after the last combinator)
    // Combinators are space, >, +, ~
    let rightmost = part
        .rsplit(|c: char| c == ' ' || c == '>' || c == '+' || c == '~')
        .find(|s| !s.is_empty())
        .unwrap_or(part);

    // Extract simple selectors (.class, #id, tag, [attr], :pseudo)
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = rightmost.chars().peekable();

    while let Some(c) = chars.next() {
        if matches!(c, '.' | '#' | '[' | ':') {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            current.push(c);
            // For brackets, read until matching ]
            if c == '[' {
                while let Some(&n) = chars.peek() {
                    current.push(n);
                    chars.next();
                    if n == ']' {
                        break;
                    }
                }
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    // For each token, check if it matches usage. ALL tokens must match for the
    // compound selector to match (e.g., "div.foo" needs both div and .foo).
    let mut all_match = true;
    let mut had_any_check = false;

    for token in &tokens {
        if let Some(class) = token.strip_prefix('.') {
            had_any_check = true;
            if !usage.has_class(class) && !is_safelisted(class) {
                all_match = false;
                break;
            }
        } else if let Some(id) = token.strip_prefix('#') {
            had_any_check = true;
            if !usage.has_id(id) {
                all_match = false;
                break;
            }
        } else if token.starts_with('[') || token.starts_with(':') {
            // Attribute or pseudo-class — too hard to check, keep it
            // (we don't track attributes/pseudo-states)
            return true;
        } else if token == "*" {
            // Universal selector — always matches
            return true;
        } else {
            // Tag name
            had_any_check = true;
            if !usage.has_tag(token) {
                all_match = false;
                break;
            }
        }
    }

    // If we had no checks at all (e.g., empty rightmost), keep to be safe
    if !had_any_check {
        return true;
    }

    all_match
}

/// Check if a class name is safelisted (always preserved).
fn is_safelisted(class: &str) -> bool {
    for prefix in DEFAULT_SAFELIST_PREFIXES {
        if class.starts_with(prefix) {
            return true;
        }
    }
    for exact in DEFAULT_SAFELIST_EXACT {
        if class == *exact {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_unused_class() {
        // Note: avoid using "unused" as the class name — it's in the safelist
        // (Notes 8 client). Use a clearly arbitrary name instead.
        let html = r#"<html><head><style>
            .used { color: red; }
            .extra-rule { color: blue; }
        </style></head>
        <body><div class="used">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".used"));
        assert!(!result.contains(".extra-rule"));
    }

    #[test]
    fn test_keeps_used_classes() {
        let html = r#"<html><head><style>
            .a { color: red; }
            .b { color: blue; }
        </style></head>
        <body><div class="a b">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".a"));
        assert!(result.contains(".b"));
    }

    #[test]
    fn test_removes_unused_tag() {
        let html = r#"<html><head><style>
            div { color: red; }
            blockquote { color: green; }
        </style></head>
        <body><div>Hi</div></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains("div { color: red"));
        assert!(!result.contains("blockquote"));
    }

    #[test]
    fn test_keeps_safelisted_gmail_class() {
        let html = r#"<html><head><style>
            .gmail-fix { font-size: 14px; }
            .unused-thing { color: blue; }
        </style></head><body></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".gmail-fix"));
        assert!(!result.contains(".unused-thing"));
    }

    #[test]
    fn test_keeps_apple_safelist() {
        let html = r#"<html><head><style>
            .apple-link { color: blue; }
        </style></head><body></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".apple-link"));
    }

    #[test]
    fn test_keeps_id_selector() {
        let html = r##"<html><head><style>
            #header { color: red; }
            #footer { color: blue; }
        </style></head>
        <body><div id="header">Hi</div></body></html>"##;
        let result = process(html).unwrap();
        assert!(result.contains("#header"));
        assert!(!result.contains("#footer"));
    }

    #[test]
    fn test_keeps_media_query() {
        let html = r#"<html><head><style>
            @media (max-width: 600px) {
                .responsive { width: 100%; }
            }
        </style></head>
        <body></body></html>"#;
        let result = process(html).unwrap();
        // We keep all @media blocks regardless of usage
        assert!(result.contains("@media"));
        assert!(result.contains("max-width: 600px"));
    }

    #[test]
    fn test_keeps_template_syntax() {
        let html = r#"<html><head><style>
            .{{class_name}} { color: red; }
        </style></head><body></body></html>"#;
        let result = process(html).unwrap();
        // Template syntax is preserved
        assert!(result.contains("{{class_name}}"));
    }

    #[test]
    fn test_comma_separated_selector_kept_if_one_matches() {
        let html = r#"<html><head><style>
            .foo, .bar { color: red; }
        </style></head>
        <body><div class="foo">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        // Should keep the rule because .foo matches
        assert!(result.contains(".foo"));
    }

    #[test]
    fn test_pseudo_class_selector_kept() {
        let html = r#"<html><head><style>
            a:hover { color: red; }
        </style></head>
        <body><a href="x">link</a></body></html>"#;
        let result = process(html).unwrap();
        // a:hover should be kept (pseudo-classes are too hard to check)
        assert!(result.contains("a:hover"));
    }

    #[test]
    fn test_descendant_selector() {
        let html = r#"<html><head><style>
            .parent .child { color: red; }
        </style></head>
        <body><div class="parent"><span class="child">Hi</span></div></body></html>"#;
        let result = process(html).unwrap();
        // The rightmost selector is .child which is in usage → keep
        assert!(result.contains(".child"));
    }
}
