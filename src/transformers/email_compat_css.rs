use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Base pixel size for rem conversion (matches browser default).
const REM_BASE: f64 = 16.0;

/// Convert CSS to be email-client compatible.
///
/// Transforms in inline `style` attributes:
/// - `rem` units → `px` (Outlook doesn't support rem)
/// - CSS logical properties → physical equivalents (no email client supports these)
/// - `gap` → removed (not supported; spacing should use padding/margin)
///
/// Transforms across the whole document (so it picks up `<style>` blocks too):
/// - Modern `@media (width >= X)` range syntax → legacy `(min-width: X)`
/// - Modern `@media (width < X)` range syntax → legacy `(max-width: X-1px)`
/// - Same for `height` queries
/// - `rem` units in media query values → `px`
pub fn process(html: &str) -> Result<String, Error> {
    // 1. Convert media query syntax across the whole document.
    //    @media only appears inside <style> tags or @-rule contexts, never
    //    in HTML attributes, so a simple string scan is safe.
    let html = convert_media_queries(html);

    // 2. Walk inline style attributes for rem/logical-property conversion.
    rewrite_str(
        html.as_str(),
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

/// Convert modern CSS Media Queries Level 4 range syntax to legacy
/// `min-width:` / `max-width:` form for email-client compatibility.
///
/// Also converts `rem` values inside media queries to `px`.
fn convert_media_queries(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut remaining = css;

    while let Some(at_idx) = remaining.find("@media") {
        result.push_str(&remaining[..at_idx]);
        let after_at = &remaining[at_idx..];

        // Find the end of the @media query (the `{` that opens the block,
        // or end of string)
        let media_end = after_at.find('{').unwrap_or(after_at.len());
        let media_part = &after_at[..media_end];

        let converted = convert_media_query_conditions(media_part);
        result.push_str(&converted);

        remaining = &after_at[media_end..];
    }

    result.push_str(remaining);
    result
}

/// Convert range conditions inside a single `@media ...` prelude.
fn convert_media_query_conditions(media: &str) -> String {
    // Walk through and find `(<feature> <op> <value>)` patterns.
    // We need to handle: width, height, max-width, min-width (passthrough), etc.
    let mut result = String::with_capacity(media.len());
    let mut remaining = media;

    while let Some(open) = remaining.find('(') {
        result.push_str(&remaining[..open + 1]);
        let after_open = &remaining[open + 1..];

        let Some(close) = after_open.find(')') else {
            result.push_str(after_open);
            return result;
        };

        let inside = &after_open[..close];
        let converted_inside = convert_range_condition(inside);
        result.push_str(&converted_inside);
        result.push(')');

        remaining = &after_open[close + 1..];
    }

    result.push_str(remaining);
    result
}

/// Convert a single condition string (without parens) like "width >= 600px"
/// into "min-width: 600px". Pass through unrecognized syntax unchanged.
fn convert_range_condition(condition: &str) -> String {
    let trimmed = condition.trim();

    // Try each operator from longest to shortest to avoid `<` matching `<=`
    for op in [">=", "<=", ">", "<"] {
        if let Some((feature, value)) = trimmed.split_once(op) {
            let feature = feature.trim();
            let value = convert_rem_to_px(value.trim());

            // Only convert known features
            let prop = match feature {
                "width" | "height" => feature,
                _ => return condition.to_string(),
            };

            return match op {
                ">=" => format!("min-{prop}: {value}"),
                "<=" => format!("max-{prop}: {value}"),
                ">" => {
                    // Strictly greater → min-width: value + 1px
                    let bumped = bump_px(&value, 1);
                    format!("min-{prop}: {bumped}")
                }
                "<" => {
                    // Strictly less → max-width: value - 1px
                    let bumped = bump_px(&value, -1);
                    format!("max-{prop}: {bumped}")
                }
                _ => unreachable!(),
            };
        }
    }

    // No range operator — pass through (handles legacy `min-width: 600px` etc.)
    condition.to_string()
}

/// Add or subtract pixels from a CSS length value. Only works on `px` values;
/// other units are returned unchanged.
fn bump_px(value: &str, delta: i64) -> String {
    if let Some(num_str) = value.strip_suffix("px") {
        if let Ok(n) = num_str.trim().parse::<i64>() {
            return format!("{}px", n + delta);
        }
        if let Ok(n) = num_str.trim().parse::<f64>() {
            return format!("{}px", (n + delta as f64) as i64);
        }
    }
    value.to_string()
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

    // ─── Media query range syntax conversion ──────────────────

    #[test]
    fn test_width_gte_to_min_width() {
        assert_eq!(
            convert_media_queries("@media (width >= 600px)"),
            "@media (min-width: 600px)"
        );
    }

    #[test]
    fn test_width_lte_to_max_width() {
        assert_eq!(
            convert_media_queries("@media (width <= 600px)"),
            "@media (max-width: 600px)"
        );
    }

    #[test]
    fn test_width_lt_to_max_width_minus_one() {
        // (width < 600px) means strictly less than 600 → max-width: 599px
        assert_eq!(
            convert_media_queries("@media (width < 600px)"),
            "@media (max-width: 599px)"
        );
    }

    #[test]
    fn test_width_gt_to_min_width_plus_one() {
        // (width > 600px) means strictly more than 600 → min-width: 601px
        assert_eq!(
            convert_media_queries("@media (width > 600px)"),
            "@media (min-width: 601px)"
        );
    }

    #[test]
    fn test_height_range() {
        assert_eq!(
            convert_media_queries("@media (height >= 400px)"),
            "@media (min-height: 400px)"
        );
        assert_eq!(
            convert_media_queries("@media (height < 800px)"),
            "@media (max-height: 799px)"
        );
    }

    #[test]
    fn test_legacy_syntax_unchanged() {
        // Already-legacy syntax should pass through unchanged
        assert_eq!(
            convert_media_queries("@media (min-width: 600px)"),
            "@media (min-width: 600px)"
        );
        assert_eq!(
            convert_media_queries("@media (max-width: 599px)"),
            "@media (max-width: 599px)"
        );
    }

    #[test]
    fn test_full_style_block() {
        let html = r#"<style>
            @media (width >= 600px) {
                .desktop { display: block; }
            }
            @media (width < 600px) {
                .mobile { display: block; }
            }
        </style>"#;
        let result = process(html).unwrap();
        assert!(result.contains("(min-width: 600px)"));
        assert!(result.contains("(max-width: 599px)"));
        assert!(!result.contains("width >="));
        assert!(!result.contains("width <"));
    }

    #[test]
    fn test_multiple_media_queries_in_style() {
        let html = r#"<style>
            @media (width >= 40rem) { .a { color: red; } }
            @media (width >= 64rem) { .b { color: blue; } }
        </style>"#;
        let result = process(html).unwrap();
        // Should also convert rem in media queries
        assert!(result.contains("(min-width: 640px)"));
        assert!(result.contains("(min-width: 1024px)"));
    }
}
