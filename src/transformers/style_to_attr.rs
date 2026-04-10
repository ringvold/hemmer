use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Mappings from CSS properties to HTML attributes for elements that
/// benefit from having both (Outlook compatibility).
///
/// Format: (selector, css_property, html_attribute, value_filter)
struct Mapping {
    selector: &'static str,
    css_prop: &'static str,
    attr: &'static str,
    /// If true, only copy values without units (e.g. "600" not "600px")
    strip_unit: bool,
}

const MAPPINGS: &[Mapping] = &[
    // Width on table/td/img — Outlook needs HTML attribute
    Mapping { selector: "table", css_prop: "width", attr: "width", strip_unit: true },
    Mapping { selector: "td", css_prop: "width", attr: "width", strip_unit: true },
    Mapping { selector: "th", css_prop: "width", attr: "width", strip_unit: true },
    Mapping { selector: "img", css_prop: "width", attr: "width", strip_unit: true },
    Mapping { selector: "video", css_prop: "width", attr: "width", strip_unit: true },
    // Height
    Mapping { selector: "table", css_prop: "height", attr: "height", strip_unit: true },
    Mapping { selector: "td", css_prop: "height", attr: "height", strip_unit: true },
    Mapping { selector: "th", css_prop: "height", attr: "height", strip_unit: true },
    Mapping { selector: "img", css_prop: "height", attr: "height", strip_unit: true },
    Mapping { selector: "video", css_prop: "height", attr: "height", strip_unit: true },
    // Background color
    Mapping { selector: "table", css_prop: "background-color", attr: "bgcolor", strip_unit: false },
    Mapping { selector: "td", css_prop: "background-color", attr: "bgcolor", strip_unit: false },
    Mapping { selector: "th", css_prop: "background-color", attr: "bgcolor", strip_unit: false },
    Mapping { selector: "tr", css_prop: "background-color", attr: "bgcolor", strip_unit: false },
    Mapping { selector: "body", css_prop: "background-color", attr: "bgcolor", strip_unit: false },
    // Text alignment
    Mapping { selector: "td", css_prop: "text-align", attr: "align", strip_unit: false },
    Mapping { selector: "th", css_prop: "text-align", attr: "align", strip_unit: false },
    Mapping { selector: "p", css_prop: "text-align", attr: "align", strip_unit: false },
    Mapping { selector: "div", css_prop: "text-align", attr: "align", strip_unit: false },
    // Vertical alignment
    Mapping { selector: "td", css_prop: "vertical-align", attr: "valign", strip_unit: false },
    Mapping { selector: "th", css_prop: "vertical-align", attr: "valign", strip_unit: false },
];

/// Copy CSS properties from inline `style` attributes into matching HTML attributes.
///
/// Only adds the attribute if it doesn't already exist on the element.
/// Used for Outlook compatibility where HTML attributes are more reliable than CSS.
pub fn process(html: &str) -> Result<String, Error> {
    let mut handlers = Vec::new();

    for mapping in MAPPINGS {
        handlers.push(element!(mapping.selector, |el| {
            // Skip if attribute already exists
            if el.get_attribute(mapping.attr).is_some() {
                return Ok(());
            }

            let Some(style) = el.get_attribute("style") else {
                return Ok(());
            };

            if let Some(value) = extract_css_value(&style, mapping.css_prop) {
                let final_value = if mapping.strip_unit {
                    strip_px_unit(&value)
                } else {
                    value.to_string()
                };

                if !final_value.is_empty() {
                    el.set_attribute(mapping.attr, &final_value)
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

/// Extract a CSS property value from a `style` attribute string.
fn extract_css_value<'a>(style: &'a str, prop: &str) -> Option<&'a str> {
    for declaration in style.split(';') {
        let declaration = declaration.trim();
        if let Some((p, v)) = declaration.split_once(':') {
            if p.trim() == prop {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Strip the `px` suffix from a CSS value if present.
/// "600px" → "600", "100%" → "100%", "auto" → ""
fn strip_px_unit(value: &str) -> String {
    let value = value.trim();
    if let Some(num) = value.strip_suffix("px") {
        num.trim().to_string()
    } else if value.ends_with('%') {
        value.to_string()
    } else if value.chars().all(|c| c.is_ascii_digit() || c == '.') {
        value.to_string()
    } else {
        // Non-numeric values like "auto" can't be HTML attributes
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_css_value() {
        let style = "width: 600px; color: red; padding: 10px";
        assert_eq!(extract_css_value(style, "width"), Some("600px"));
        assert_eq!(extract_css_value(style, "color"), Some("red"));
        assert_eq!(extract_css_value(style, "missing"), None);
    }

    #[test]
    fn test_strip_px_unit() {
        assert_eq!(strip_px_unit("600px"), "600");
        assert_eq!(strip_px_unit("100%"), "100%");
        assert_eq!(strip_px_unit("auto"), "");
        assert_eq!(strip_px_unit("12.5px"), "12.5");
    }

    #[test]
    fn test_table_width() {
        let html = r#"<table style="width: 600px;"><tr><td>Hi</td></tr></table>"#;
        let result = process(html).unwrap();
        assert!(result.contains("width=\"600\""));
    }

    #[test]
    fn test_bgcolor() {
        let html = r#"<td style="background-color: #ff0000;">Hi</td>"#;
        let result = process(html).unwrap();
        assert!(result.contains("bgcolor=\"#ff0000\""));
    }

    #[test]
    fn test_existing_attr_preserved() {
        let html = r#"<table width="500" style="width: 600px;"><tr><td>Hi</td></tr></table>"#;
        let result = process(html).unwrap();
        // Should keep existing width="500", not overwrite with 600
        assert!(result.contains("width=\"500\""));
        assert!(!result.contains("width=\"600\""));
    }

    #[test]
    fn test_align() {
        let html = r#"<td style="text-align: center;">Hi</td>"#;
        let result = process(html).unwrap();
        assert!(result.contains("align=\"center\""));
    }

    #[test]
    fn test_skip_auto_value() {
        let html = r#"<table style="width: auto;"><tr><td>Hi</td></tr></table>"#;
        let result = process(html).unwrap();
        // "auto" can't be an HTML attribute, so it should not be added
        assert!(!result.contains("width=\"auto\""));
    }
}
