use crate::error::Error;

/// Convert `<outlook>` and `<not-outlook>` tags to MSO conditional comments.
///
/// Examples:
/// - `<outlook>X</outlook>` → `<!--[if mso]>X<![endif]-->`
/// - `<not-outlook>X</not-outlook>` → `<!--[if !mso]><!-->X<!--<![endif]-->`
/// - `<outlook only="mso 9">X</outlook>` → `<!--[if mso 9]>X<![endif]-->`
/// - `<outlook only="gte mso 9">X</outlook>` → `<!--[if gte mso 9]>X<![endif]-->`
pub fn process(html: &str) -> Result<String, Error> {
    // Process <not-outlook> first to avoid conflicts with <outlook>
    let result = transform_tag(html, "not-outlook", |_| {
        ("<!--[if !mso]><!-->".to_string(), "<!--<![endif]-->".to_string())
    });

    let result = transform_tag(&result, "outlook", |attrs| {
        let condition = parse_only_attr(attrs).unwrap_or_else(|| "mso".to_string());
        (
            format!("<!--[if {condition}]>"),
            "<![endif]-->".to_string(),
        )
    });

    Ok(result)
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

/// Parse `only="mso 9"` from the attributes string.
fn parse_only_attr(attrs: &str) -> Option<String> {
    let idx = attrs.find("only")?;
    let after = &attrs[idx + 4..];
    let after = after.trim_start();
    let after = after.strip_prefix('=')?;
    let after = after.trim_start();

    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let after = &after[1..];
    let end = after.find(quote)?;
    Some(after[..end].to_string())
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
}
