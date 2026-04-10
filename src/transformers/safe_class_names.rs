use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

/// Character → replacement string mappings.
///
/// Mirrors the defaults from `posthtml-safe-class-names` so Tailwind utility
/// classes (including arbitrary values like `bg-[#fff]`) become safe for
/// email client CSS parsers.
///
/// Applied to:
/// - `class=""` attributes (literal characters: `bg-[#fff]`)
/// - `<style>` tag contents (CSS-escaped form: `.bg-\[\#fff\]`)
const REPLACEMENTS: &[(char, &str)] = &[
    (':', "-"),
    ('/', "-"),
    ('%', "pc"),
    ('.', "_"),
    (',', "_"),
    ('#', "_"),
    ('[', ""),
    (']', ""),
    ('(', ""),
    (')', ""),
    ('{', ""),
    ('}', ""),
    ('!', "i-"),
    ('&', "and-"),
    ('<', "lt-"),
    ('=', "eq-"),
    ('>', "gt-"),
    ('|', "or-"),
    ('@', "at-"),
    ('?', "q-"),
    ('\\', "-"),
    ('"', "-"),
    ('\'', "-"),
    ('*', "-"),
    ('+', "-"),
    (';', "-"),
    ('^', "-"),
    ('`', "-"),
    ('~', "-"),
    ('$', "-"),
];

fn make_safe(class: &str) -> String {
    let mut result = String::with_capacity(class.len());
    'outer: for ch in class.chars() {
        for &(from, to) in REPLACEMENTS {
            if ch == from {
                result.push_str(to);
                continue 'outer;
            }
        }
        result.push(ch);
    }
    result
}

/// In CSS, the unsafe chars appear escaped: `.bg-\[\#1da1f1\]`
/// We need to find each `\<char>` sequence and replace with the safe form.
fn replace_in_css(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                let mut replaced = false;
                for &(from, to) in REPLACEMENTS {
                    if next == from {
                        result.push_str(to);
                        chars.next();
                        replaced = true;
                        break;
                    }
                }
                if !replaced {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Rewrite class names in both `class` attributes and `<style>` tag contents.
pub fn process(html: &str) -> Result<String, Error> {
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                // Rewrite class attributes on elements
                element!("[class]", |el| {
                    if let Some(classes) = el.get_attribute("class") {
                        let safe_classes: String = classes
                            .split_whitespace()
                            .map(make_safe)
                            .collect::<Vec<_>>()
                            .join(" ");
                        el.set_attribute("class", &safe_classes)
                            .map_err(|e| format!("{e}"))?;
                    }
                    Ok(())
                }),
                // Rewrite CSS escape sequences inside <style> tags
                text!("style", |chunk| {
                    let text = chunk.as_str();
                    let result = replace_in_css(text);
                    if result != text {
                        chunk.replace(&result, lol_html::html_content::ContentType::Text);
                    }
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

    // ─── Class attribute replacements ───────────────────────────

    #[test]
    fn test_slash_to_dash() {
        let html = r#"<div class="w-1/2">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"w-1-2\""));
    }

    #[test]
    fn test_colon_to_dash() {
        let html = r#"<div class="sm:text-base">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"sm-text-base\""));
    }

    #[test]
    fn test_percent_to_pc() {
        let html = r#"<div class="w-50%">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"w-50pc\""));
    }

    #[test]
    fn test_dot_to_underscore() {
        let html = r#"<div class="text-1.5xl">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"text-1_5xl\""));
    }

    #[test]
    fn test_brackets_removed() {
        let html = r#"<div class="bg-[#fff]">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"bg-_fff\""));
    }

    #[test]
    fn test_parens_removed() {
        let html = r#"<div class="w-(100px)">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"w-100px\""));
    }

    #[test]
    fn test_curly_braces_removed() {
        let html = r#"<div class="w-{100}">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"w-100\""));
    }

    #[test]
    fn test_bang_to_i_dash() {
        let html = r#"<div class="!important">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"i-important\""));
    }

    #[test]
    fn test_ampersand_to_and() {
        let html = r#"<div class="this&that">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"thisand-that\""));
    }

    #[test]
    fn test_hash_to_underscore() {
        let html = r#"<div class="bg-#fff">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"bg-_fff\""));
    }

    #[test]
    fn test_lt_gt_to_named() {
        let html = r#"<div class="w-<10">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"w-lt-10\""));
    }

    #[test]
    fn test_realistic_tailwind_arbitrary() {
        // The killer test: bg-[#1da1f1] with multiple special chars
        let html = r#"<div class="bg-[#1da1f1]">Hi</div>"#;
        let result = process(html).unwrap();
        // [ → "", # → _, ] → ""  =>  bg-_1da1f1
        assert!(result.contains("class=\"bg-_1da1f1\""));
    }

    // ─── Style tag (escaped CSS) replacements ──────────────────

    #[test]
    fn test_style_escaped_slash() {
        let html = r#"<style>.w-1\/2 { width: 50%; }</style>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".w-1-2"));
        assert!(!result.contains("\\/"));
    }

    #[test]
    fn test_style_escaped_colon() {
        let html = r#"<style>.sm\:text-base { font-size: 16px; }</style>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".sm-text-base"));
        assert!(!result.contains("\\:"));
    }

    #[test]
    fn test_style_escaped_brackets() {
        // Tailwind v4 arbitrary value: .bg-\[\#1da1f1\]
        let html = r#"<style>.bg-\[\#1da1f1\] { background: #1da1f1; }</style>"#;
        let result = process(html).unwrap();
        // [ → "", # → _, ] → ""
        assert!(result.contains(".bg-_1da1f1"));
        assert!(!result.contains("\\["));
        assert!(!result.contains("\\]"));
        assert!(!result.contains("\\#"));
    }

    #[test]
    fn test_style_escaped_percent() {
        let html = r#"<style>.w-50\% { width: 50%; }</style>"#;
        let result = process(html).unwrap();
        assert!(result.contains(".w-50pc"));
        // Make sure non-escaped % in width: 50%; is preserved
        assert!(result.contains("width: 50%"));
    }

    // ─── Round-trip: class attr + style tag stay matched ───────

    #[test]
    fn test_class_and_style_stay_matched() {
        let html = r##"<html><head><style>
        .bg-\[\#1da1f1\] { background-color: #1da1f1; }
        .w-1\/2 { width: 50%; }
        </style></head><body>
        <div class="bg-[#1da1f1] w-1/2">Hi</div>
        </body></html>"##;
        let result = process(html).unwrap();

        // Both should now be the same safe form
        assert!(result.contains(".bg-_1da1f1"));
        assert!(result.contains(".w-1-2"));
        assert!(result.contains("class=\"bg-_1da1f1 w-1-2\""));
    }

    // ─── Things that should NOT change ─────────────────────────

    #[test]
    fn test_normal_classes_unchanged() {
        let html = r#"<div class="text-lg font-bold">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("class=\"text-lg font-bold\""));
    }

    #[test]
    fn test_no_class_attribute_unchanged() {
        let html = "<div>Hi</div>";
        let result = process(html).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_css_values_unchanged() {
        // Make sure we don't break CSS values like "50%" inside @media queries
        let html = r#"<style>@media (max-width: 600px) { .x { width: 50%; } }</style>"#;
        let result = process(html).unwrap();
        assert!(result.contains("max-width: 600px"));
        assert!(result.contains("width: 50%"));
    }
}
