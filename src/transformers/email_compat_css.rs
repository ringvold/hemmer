use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Base pixel size for rem conversion (matches browser default).
const REM_BASE: f64 = 16.0;

/// Convert CSS in inline `style` attributes to be email-client compatible.
///
/// Transforms:
/// - `rem` units → `px` (Outlook doesn't support rem)
/// - CSS logical properties → physical equivalents (no email client supports these)
/// - `gap` → removed (not supported; spacing should use padding/margin)
pub fn process(html: &str) -> Result<String, Error> {
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("[style]", |el| {
                if let Some(style) = el.get_attribute("style") {
                    let converted = convert_style(&style);
                    if converted != style {
                        el.set_attribute("style", &converted)
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

fn convert_style(style: &str) -> String {
    let mut result = Vec::new();

    for declaration in style.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }

        let Some((prop, value)) = declaration.split_once(':') else {
            result.push(declaration.to_string());
            continue;
        };

        let prop = prop.trim();
        let value = value.trim();

        // Convert rem values to px
        let value = convert_rem_to_px(value);

        // Expand logical properties to physical equivalents
        match prop {
            "padding-inline" => {
                result.push(format!("padding-left: {value}"));
                result.push(format!("padding-right: {value}"));
            }
            "padding-block" => {
                result.push(format!("padding-top: {value}"));
                result.push(format!("padding-bottom: {value}"));
            }
            "padding-inline-start" => {
                result.push(format!("padding-left: {value}"));
            }
            "padding-inline-end" => {
                result.push(format!("padding-right: {value}"));
            }
            "padding-block-start" => {
                result.push(format!("padding-top: {value}"));
            }
            "padding-block-end" => {
                result.push(format!("padding-bottom: {value}"));
            }
            "margin-inline" => {
                result.push(format!("margin-left: {value}"));
                result.push(format!("margin-right: {value}"));
            }
            "margin-block" => {
                result.push(format!("margin-top: {value}"));
                result.push(format!("margin-bottom: {value}"));
            }
            "margin-inline-start" => {
                result.push(format!("margin-left: {value}"));
            }
            "margin-inline-end" => {
                result.push(format!("margin-right: {value}"));
            }
            "margin-block-start" => {
                result.push(format!("margin-top: {value}"));
            }
            "margin-block-end" => {
                result.push(format!("margin-bottom: {value}"));
            }
            "inset-inline" => {
                result.push(format!("left: {value}"));
                result.push(format!("right: {value}"));
            }
            "inset-block" => {
                result.push(format!("top: {value}"));
                result.push(format!("bottom: {value}"));
            }
            "border-inline" => {
                result.push(format!("border-left: {value}"));
                result.push(format!("border-right: {value}"));
            }
            "border-block" => {
                result.push(format!("border-top: {value}"));
                result.push(format!("border-bottom: {value}"));
            }
            "inline-size" => {
                result.push(format!("width: {value}"));
            }
            "block-size" => {
                result.push(format!("height: {value}"));
            }
            "min-inline-size" => {
                result.push(format!("min-width: {value}"));
            }
            "max-inline-size" => {
                result.push(format!("max-width: {value}"));
            }
            "min-block-size" => {
                result.push(format!("min-height: {value}"));
            }
            "max-block-size" => {
                result.push(format!("max-height: {value}"));
            }
            // Drop unsupported properties that have no email equivalent
            "gap" | "row-gap" | "column-gap" => {}
            // Keep everything else
            _ => {
                result.push(format!("{prop}: {value}"));
            }
        }
    }

    if result.is_empty() {
        String::new()
    } else {
        format!("{};", result.join("; "))
    }
}

/// Convert rem values to px in a CSS value string.
/// E.g., "1.5rem" → "24px", "0.75rem" → "12px"
fn convert_rem_to_px(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.char_indices().peekable();

    while let Some((i, _)) = chars.peek() {
        let i = *i;

        // Try to find a number followed by "rem"
        if let Some((num_str, end)) = try_parse_number_rem(value, i) {
            if let Ok(num) = num_str.parse::<f64>() {
                let px = num * REM_BASE;
                // Format nicely: 24.0 → "24px", 12.5 → "12.5px"
                if px == px.floor() {
                    result.push_str(&format!("{}px", px as i64));
                } else {
                    result.push_str(&format!("{px}px"));
                }
                // Skip past the consumed characters
                for _ in 0..(end - i) {
                    chars.next();
                }
                continue;
            }
        }

        result.push(value.as_bytes()[i] as char);
        chars.next();
    }

    result
}

/// Try to parse a number followed by "rem" starting at position `start`.
/// Returns (number_string, end_position) if successful.
fn try_parse_number_rem(value: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &value[start..];

    // Find the extent of the number (digits, dots, minus)
    let num_end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());

    if num_end == 0 {
        return None;
    }

    let after_num = &rest[num_end..];
    if after_num.starts_with("rem") {
        // Make sure "rem" isn't part of a longer word
        let after_rem = &after_num[3..];
        if after_rem.is_empty()
            || !after_rem.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-')
        {
            return Some((&rest[..num_end], start + num_end + 3));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rem_to_px() {
        assert_eq!(convert_rem_to_px("1rem"), "16px");
        assert_eq!(convert_rem_to_px("1.5rem"), "24px");
        assert_eq!(convert_rem_to_px("0.75rem"), "12px");
        assert_eq!(convert_rem_to_px("0.875rem"), "14px");
        assert_eq!(convert_rem_to_px("2rem"), "32px");
    }

    #[test]
    fn test_rem_in_multi_value() {
        assert_eq!(
            convert_rem_to_px("1.5rem 2rem"),
            "24px 32px"
        );
    }

    #[test]
    fn test_px_unchanged() {
        assert_eq!(convert_rem_to_px("16px"), "16px");
        assert_eq!(convert_rem_to_px("100%"), "100%");
    }

    #[test]
    fn test_logical_to_physical() {
        assert_eq!(
            convert_style("padding-inline: 1rem"),
            "padding-left: 16px; padding-right: 16px;"
        );
        assert_eq!(
            convert_style("padding-block: 0.75rem"),
            "padding-top: 12px; padding-bottom: 12px;"
        );
        assert_eq!(
            convert_style("margin-inline: 2rem"),
            "margin-left: 32px; margin-right: 32px;"
        );
    }

    #[test]
    fn test_mixed_properties() {
        assert_eq!(
            convert_style("padding-inline: 1rem; color: red; margin-block: 0.5rem"),
            "padding-left: 16px; padding-right: 16px; color: red; margin-top: 8px; margin-bottom: 8px;"
        );
    }

    #[test]
    fn test_gap_removed() {
        assert_eq!(convert_style("gap: 1rem"), "");
        assert_eq!(
            convert_style("gap: 1rem; color: red"),
            "color: red;"
        );
    }

    #[test]
    fn test_size_properties() {
        assert_eq!(
            convert_style("inline-size: 100%"),
            "width: 100%;"
        );
        assert_eq!(
            convert_style("block-size: 2rem"),
            "height: 32px;"
        );
    }

    #[test]
    fn test_html_transform() {
        let html = r#"<div style="padding-inline: 1.5rem; font-size: 0.875rem;">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("padding-left: 24px"));
        assert!(result.contains("padding-right: 24px"));
        assert!(result.contains("font-size: 14px"));
        assert!(!result.contains("rem"));
    }
}
