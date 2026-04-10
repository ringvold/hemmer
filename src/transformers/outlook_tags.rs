use crate::error::Error;

/// Convert `<outlook>` and `<not-outlook>` tags to MSO conditional comments.
///
/// Supports version names and comparison attributes:
/// - `<outlook>X</outlook>` → `<!--[if mso]>X<![endif]-->`
/// - `<outlook only="2013">X</outlook>` → `<!--[if mso 15]>X<![endif]-->`
/// - `<outlook only="2013,2016">X</outlook>` → `<!--[if (mso 15)|(mso 16)]>X<![endif]-->`
/// - `<outlook not="2013">X</outlook>` → `<!--[if !mso 15]>X<![endif]-->`
/// - `<outlook lt="2013">X</outlook>` → `<!--[if lt mso 15]>X<![endif]-->`
/// - `<outlook lte="2010">X</outlook>` → `<!--[if lte mso 14]>X<![endif]-->`
/// - `<outlook gt="2007">X</outlook>` → `<!--[if gt mso 12]>X<![endif]-->`
/// - `<outlook gte="2013">X</outlook>` → `<!--[if gte mso 15]>X<![endif]-->`
/// - `<not-outlook>X</not-outlook>` → `<!--[if !mso]><!-->X<!--<![endif]-->`
///
/// Outlook version → mso version mapping:
/// 2003 → 9, 2007 → 12, 2010 → 14, 2013 → 15, 2016 → 16, 2019 → 16
pub fn process(html: &str) -> Result<String, Error> {
    // Process <not-outlook> first to avoid conflicts with <outlook>
    let result = transform_tag(html, "not-outlook", |_| {
        ("<!--[if !mso]><!-->".to_string(), "<!--<![endif]-->".to_string())
    });

    let result = transform_tag(&result, "outlook", |attrs| {
        let condition = build_condition(attrs);
        (
            format!("<!--[if {condition}]>"),
            "<![endif]-->".to_string(),
        )
    });

    Ok(result)
}

/// Map an Outlook version name to its MSO version number.
/// Returns the input unchanged if it doesn't match a known version.
fn version_to_mso(version: &str) -> String {
    match version.trim() {
        "2003" => "9".to_string(),
        "2007" => "12".to_string(),
        "2010" => "14".to_string(),
        "2013" => "15".to_string(),
        "2016" | "2019" => "16".to_string(),
        other => other.to_string(),
    }
}

/// Build the conditional expression body (without the outer `[if ...]`).
///
/// Checks attributes in priority order: only, not, lt, lte, gt, gte.
/// Falls back to bare `mso` if no version attribute is present.
fn build_condition(attrs: &str) -> String {
    if let Some(value) = parse_attr(attrs, "only") {
        return build_only_condition(&value);
    }
    if let Some(value) = parse_attr(attrs, "not") {
        return build_not_condition(&value);
    }
    if let Some(value) = parse_attr(attrs, "lte") {
        return format!("lte mso {}", version_to_mso(&value));
    }
    if let Some(value) = parse_attr(attrs, "gte") {
        return format!("gte mso {}", version_to_mso(&value));
    }
    if let Some(value) = parse_attr(attrs, "lt") {
        return format!("lt mso {}", version_to_mso(&value));
    }
    if let Some(value) = parse_attr(attrs, "gt") {
        return format!("gt mso {}", version_to_mso(&value));
    }
    "mso".to_string()
}

/// Build condition for `only="..."`.
/// Single version → `mso N`, multiple → `(mso N)|(mso M)`.
/// Also supports raw mso syntax pass-through (e.g., "mso 9", "gte mso 9").
fn build_only_condition(value: &str) -> String {
    let trimmed = value.trim();

    // Pass-through for raw mso syntax (preserves backward compat)
    if trimmed.starts_with("mso") || trimmed.contains("mso") {
        return trimmed.to_string();
    }

    let versions: Vec<String> = trimmed
        .split(',')
        .map(|v| version_to_mso(v))
        .collect();

    if versions.len() == 1 {
        format!("mso {}", versions[0])
    } else {
        let parts: Vec<String> = versions.iter().map(|v| format!("(mso {v})")).collect();
        parts.join("|")
    }
}

/// Build condition for `not="..."`.
/// Single → `!mso N`, multiple → `!((mso N)|(mso M))`.
fn build_not_condition(value: &str) -> String {
    let versions: Vec<String> = value
        .trim()
        .split(',')
        .map(|v| version_to_mso(v))
        .collect();

    if versions.len() == 1 {
        format!("!mso {}", versions[0])
    } else {
        let parts: Vec<String> = versions.iter().map(|v| format!("(mso {v})")).collect();
        format!("!({})", parts.join("|"))
    }
}

/// Transform all instances of `<tag ...>...</tag>` using a function that
/// returns the (open, close) replacement strings.
fn transform_tag(html: &str, tag: &str, build_replacement: impl Fn(&str) -> (String, String)) -> String {
    let open_prefix = format!("<{tag}");
    let close_tag = format!("</{tag}>");
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find(&open_prefix) {
        // Append everything before the match
        result.push_str(&remaining[..start]);
        let after_prefix = &remaining[start + open_prefix.len()..];

        // The next character must be space, '>' or '/' for it to be the right tag
        // (avoid matching <outlook-foo> when looking for <outlook>)
        let valid = after_prefix
            .chars()
            .next()
            .map(|c| c == '>' || c == ' ' || c == '/' || c == '\t' || c == '\n')
            .unwrap_or(false);

        if !valid {
            // Not our tag — append the prefix and continue scanning after it
            result.push_str(&open_prefix);
            remaining = after_prefix;
            continue;
        }

        // Find the end of the opening tag
        let Some(open_end) = after_prefix.find('>') else {
            // Malformed; append rest and bail
            result.push_str(&open_prefix);
            result.push_str(after_prefix);
            return result;
        };

        let attrs = &after_prefix[..open_end];
        let after_open = &after_prefix[open_end + 1..];

        // Find the matching close tag
        let Some(close_pos) = after_open.find(&close_tag) else {
            // No close tag; append everything and bail
            result.push_str(&open_prefix);
            result.push_str(after_prefix);
            return result;
        };

        let inner = &after_open[..close_pos];
        let after_close = &after_open[close_pos + close_tag.len()..];

        let (open_replacement, close_replacement) = build_replacement(attrs);
        result.push_str(&open_replacement);
        result.push_str(inner);
        result.push_str(&close_replacement);

        remaining = after_close;
    }

    result.push_str(remaining);
    result
}

/// Parse a named attribute value from the attributes string.
/// E.g., `parse_attr(r#" only="mso 9""#, "only")` → `Some("mso 9")`.
///
/// Uses word-boundary matching so `lt` won't match `lte`.
fn parse_attr(attrs: &str, name: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(rel_idx) = attrs[search_from..].find(name) {
        let idx = search_from + rel_idx;

        // Check word boundary before
        let prev_ok = idx == 0
            || attrs[..idx]
                .chars()
                .last()
                .map(|c| c.is_whitespace())
                .unwrap_or(true);

        let after_name = &attrs[idx + name.len()..];
        // Check word boundary after (next char must be `=` or whitespace)
        let next_ok = after_name
            .chars()
            .next()
            .map(|c| c == '=' || c.is_whitespace())
            .unwrap_or(false);

        if !prev_ok || !next_ok {
            search_from = idx + name.len();
            continue;
        }

        let after = after_name.trim_start();
        let Some(after) = after.strip_prefix('=') else {
            search_from = idx + name.len();
            continue;
        };
        let after = after.trim_start();

        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }

        let after = &after[1..];
        let end = after.find(quote)?;
        return Some(after[..end].to_string());
    }
    None
}

/// Backward-compat shim for the old API.
#[cfg(test)]
fn parse_only_attr(attrs: &str) -> Option<String> {
    parse_attr(attrs, "only")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_outlook() {
        let html = "<outlook>X</outlook>";
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso]>X<![endif]-->");
    }

    #[test]
    fn test_not_outlook() {
        let html = "<not-outlook>X</not-outlook>";
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if !mso]><!-->X<!--<![endif]-->");
    }

    #[test]
    fn test_outlook_with_only() {
        let html = r#"<outlook only="mso 9">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 9]>X<![endif]-->");
    }

    #[test]
    fn test_outlook_with_gte() {
        let html = r#"<outlook only="gte mso 9">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if gte mso 9]>X<![endif]-->");
    }

    #[test]
    fn test_nested_content() {
        let html = "<outlook><table><tr><td>Hi</td></tr></table></outlook>";
        let result = process(html).unwrap();
        assert_eq!(
            result,
            "<!--[if mso]><table><tr><td>Hi</td></tr></table><![endif]-->"
        );
    }

    #[test]
    fn test_multiple_outlook_tags() {
        let html = "<outlook>A</outlook> middle <outlook>B</outlook>";
        let result = process(html).unwrap();
        assert_eq!(
            result,
            "<!--[if mso]>A<![endif]--> middle <!--[if mso]>B<![endif]-->"
        );
    }

    #[test]
    fn test_no_outlook_tags() {
        let html = "<table><tr><td>Hi</td></tr></table>";
        let result = process(html).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_does_not_match_outlook_prefix() {
        // Make sure <outlook-foo> isn't matched
        let html = "<outlookcustom>X</outlookcustom>";
        let result = process(html).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_parse_only_attr() {
        assert_eq!(parse_only_attr(r#" only="mso 9""#), Some("mso 9".to_string()));
        assert_eq!(parse_only_attr(r#" only='gte mso 9'"#), Some("gte mso 9".to_string()));
        assert_eq!(parse_only_attr(""), None);
    }

    // ─── Version name mapping ──────────────────────────────────

    #[test]
    fn test_only_2003() {
        let html = r#"<outlook only="2003">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 9]>X<![endif]-->");
    }

    #[test]
    fn test_only_2007() {
        let html = r#"<outlook only="2007">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 12]>X<![endif]-->");
    }

    #[test]
    fn test_only_2010() {
        let html = r#"<outlook only="2010">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 14]>X<![endif]-->");
    }

    #[test]
    fn test_only_2013() {
        let html = r#"<outlook only="2013">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 15]>X<![endif]-->");
    }

    #[test]
    fn test_only_2016() {
        let html = r#"<outlook only="2016">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 16]>X<![endif]-->");
    }

    #[test]
    fn test_only_multiple_versions() {
        let html = r#"<outlook only="2013,2016">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if (mso 15)|(mso 16)]>X<![endif]-->");
    }

    // ─── Comparison attributes ─────────────────────────────────

    #[test]
    fn test_lt_attribute() {
        let html = r#"<outlook lt="2013">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if lt mso 15]>X<![endif]-->");
    }

    #[test]
    fn test_lte_attribute() {
        let html = r#"<outlook lte="2010">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if lte mso 14]>X<![endif]-->");
    }

    #[test]
    fn test_gt_attribute() {
        let html = r#"<outlook gt="2007">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if gt mso 12]>X<![endif]-->");
    }

    #[test]
    fn test_gte_attribute() {
        let html = r#"<outlook gte="2013">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if gte mso 15]>X<![endif]-->");
    }

    #[test]
    fn test_not_attribute() {
        let html = r#"<outlook not="2013">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if !mso 15]>X<![endif]-->");
    }

    #[test]
    fn test_not_multiple() {
        let html = r#"<outlook not="2013,2016">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if !((mso 15)|(mso 16))]>X<![endif]-->");
    }

    // ─── Backward compat: raw mso syntax still works ────────────

    #[test]
    fn test_raw_mso_syntax_still_works() {
        let html = r#"<outlook only="mso 9">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if mso 9]>X<![endif]-->");
    }

    #[test]
    fn test_raw_gte_mso_syntax_still_works() {
        let html = r#"<outlook only="gte mso 9">X</outlook>"#;
        let result = process(html).unwrap();
        assert_eq!(result, "<!--[if gte mso 9]>X<![endif]-->");
    }
}
