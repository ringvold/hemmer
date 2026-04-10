use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

/// Characters that are problematic in email client CSS parsers.
/// In HTML class attributes, these appear literally: `w-1/2`, `sm:text-base`
/// In CSS selectors, they appear escaped: `.w-1\/2`, `.sm\:text-base`
const UNSAFE_CHARS: &[char] = &['/', ':', '{', '}'];

/// CSS escape sequences to replace in stylesheets.
/// These are the escaped forms of the unsafe chars above.
const CSS_ESCAPE_REPLACEMENTS: &[(&str, &str)] = &[
    ("\\/", "-"),
    ("\\:", "-"),
    ("\\{", "-"),
    ("\\}", "-"),
    ("\\.", "-"),
];

fn make_safe(class: &str) -> String {
    let mut result = class.to_string();
    for &ch in UNSAFE_CHARS {
        result = result.replace(ch, "-");
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
                    let mut result = text.to_string();
                    for &(from, to) in CSS_ESCAPE_REPLACEMENTS {
                        result = result.replace(from, to);
                    }
                    chunk.replace(&result, lol_html::html_content::ContentType::Text);
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}
