use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Resolve CSS custom properties (`var(--name)`) to their static values.
///
/// 1. Scans `<style>` blocks for `:root { --name: value; }` declarations
/// 2. Replaces `var(--name)` references in inline `style` attributes
/// 3. Replaces `var(--name)` references inside `<style>` blocks
///
/// This is needed because Outlook desktop (Word engine) does not support
/// CSS custom properties.
pub fn process(html: &str) -> Result<String, Error> {
    // Phase 1: collect variables from <style> blocks
    let vars = Rc::new(RefCell::new(HashMap::<String, String>::new()));
    let collector = vars.clone();

    let _ = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![text!("style", move |chunk| {
                let css = chunk.as_str();
                for (name, value) in extract_root_vars(css) {
                    collector.borrow_mut().insert(name, value);
                }
                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))?;

    let vars_map = vars.borrow().clone();
    if vars_map.is_empty() {
        return Ok(html.to_string());
    }

    // Phase 2: replace var() references in style attributes and style tags
    let style_vars = vars_map.clone();
    let attr_vars = vars_map.clone();

    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("[style]", move |el| {
                    if let Some(style) = el.get_attribute("style") {
                        let resolved = resolve_vars(&style, &attr_vars);
                        if resolved != style {
                            el.set_attribute("style", &resolved)
                                .map_err(|e| format!("{e}"))?;
                        }
                    }
                    Ok(())
                }),
                text!("style", move |chunk| {
                    let css = chunk.as_str();
                    let resolved = resolve_vars(css, &style_vars);
                    if resolved != css {
                        chunk.replace(&resolved, lol_html::html_content::ContentType::Text);
                    }
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

/// Extract `--name: value` declarations from `:root` (or `*`) selectors.
fn extract_root_vars(css: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut remaining = css;

    while let Some(idx) = remaining.find(':') {
        let after = &remaining[idx + 1..];
        if after.starts_with("root") || after.starts_with(" root") {
            // Found :root — find opening brace
            let after_root = after.trim_start_matches("root").trim_start();
            if let Some(brace_pos) = after_root.find('{') {
                let block_start = &after_root[brace_pos + 1..];
                if let Some(end) = find_matching_brace(block_start) {
                    let block = &block_start[..end];
                    for (name, value) in parse_var_declarations(block) {
                        result.push((name, value));
                    }
                    remaining = &block_start[end + 1..];
                    continue;
                }
            }
        }
        remaining = &remaining[idx + 1..];
    }

    result
}

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

/// Parse `--name: value;` declarations from a CSS block body.
fn parse_var_declarations(block: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for declaration in block.split(';') {
        let declaration = declaration.trim();
        if let Some((prop, value)) = declaration.split_once(':') {
            let prop = prop.trim();
            if let Some(name) = prop.strip_prefix("--") {
                result.push((name.to_string(), value.trim().to_string()));
            }
        }
    }
    result
}

/// Replace `var(--name)` and `var(--name, fallback)` with the resolved value.
fn resolve_vars(input: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(idx) = remaining.find("var(") {
        result.push_str(&remaining[..idx]);
        let after_open = &remaining[idx + 4..];

        // Find matching close paren (no nesting expected for var())
        let Some(close_pos) = after_open.find(')') else {
            result.push_str("var(");
            remaining = after_open;
            continue;
        };

        let inner = &after_open[..close_pos];
        let after_close = &after_open[close_pos + 1..];

        // Parse "--name" or "--name, fallback"
        let (name_part, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };

        let var_name = name_part.strip_prefix("--").unwrap_or(name_part);

        if let Some(value) = vars.get(var_name) {
            result.push_str(value);
        } else if let Some(fb) = fallback {
            result.push_str(fb);
        } else {
            // Unknown variable, no fallback — leave as-is
            result.push_str("var(");
            result.push_str(inner);
            result.push(')');
        }

        remaining = after_close;
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_root_vars() {
        let css = ":root { --primary: #4f46e5; --spacing: 16px; }";
        let vars = extract_root_vars(css);
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&("primary".to_string(), "#4f46e5".to_string())));
        assert!(vars.contains(&("spacing".to_string(), "16px".to_string())));
    }

    #[test]
    fn test_resolve_simple_var() {
        let mut vars = HashMap::new();
        vars.insert("primary".to_string(), "#4f46e5".to_string());

        assert_eq!(
            resolve_vars("color: var(--primary)", &vars),
            "color: #4f46e5"
        );
    }

    #[test]
    fn test_resolve_var_with_fallback() {
        let vars = HashMap::new();
        // Variable not defined, fallback should be used
        assert_eq!(
            resolve_vars("color: var(--missing, red)", &vars),
            "color: red"
        );
    }

    #[test]
    fn test_resolve_unknown_var_no_fallback() {
        let vars = HashMap::new();
        // No variable, no fallback — leave as-is
        assert_eq!(
            resolve_vars("color: var(--missing)", &vars),
            "color: var(--missing)"
        );
    }

    #[test]
    fn test_full_html_pipeline() {
        let html = r#"<html><head><style>:root { --primary: #4f46e5; }</style></head>
        <body><div style="color: var(--primary);">Hi</div></body></html>"#;
        let result = process(html).unwrap();
        assert!(result.contains("color: #4f46e5"));
        assert!(!result.contains("var(--primary)"));
    }

    #[test]
    fn test_no_vars_unchanged() {
        let html = r#"<div style="color: red">Hi</div>"#;
        let result = process(html).unwrap();
        assert_eq!(result, html);
    }
}
