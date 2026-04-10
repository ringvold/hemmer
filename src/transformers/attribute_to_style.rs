use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Convert HTML presentational attributes into inline CSS styles.
///
/// This is the opposite direction of `style_to_attr`. Use this when your
/// template uses HTML attributes (like `bgcolor`, `width`) and you want
/// CSS equivalents added to the `style` attribute as well.
///
/// Mappings:
/// - `bgcolor="#fff"` → `background-color: #fff`
/// - `background="img.png"` → `background-image: url('img.png')`
/// - `width="600"` → `width: 600px` (number) or `width: 100%` (with %)
/// - `height="..."` → `height: ...`
/// - `align="center"` on `<table>` → `margin-left: auto; margin-right: auto`
/// - `align="..."` on other elements → `text-align: ...`
/// - `valign="..."` → `vertical-align: ...`
///
/// Existing inline styles are preserved; new declarations are appended.
/// If a CSS property is already in the style attribute, the HTML attribute
/// is NOT used (existing style wins).
pub fn process(html: &str, attributes: &[&str]) -> Result<String, Error> {
    if attributes.is_empty() {
        return Ok(html.to_string());
    }

    let attrs: Vec<String> = attributes.iter().map(|s| s.to_string()).collect();

    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("*", |el| {
                let tag_name = el.tag_name();
                let mut new_decls: Vec<(String, String)> = Vec::new();

                for attr in &attrs {
                    let Some(value) = el.get_attribute(attr) else {
                        continue;
                    };
                    if value.trim().is_empty() {
                        continue;
                    }

                    match attr.as_str() {
                        "bgcolor" => {
                            new_decls.push(("background-color".into(), value));
                        }
                        "background" => {
                            new_decls.push((
                                "background-image".into(),
                                format!("url('{}')", value),
                            ));
                        }
                        "width" => {
                            new_decls.push(("width".into(), normalize_size(&value)));
                        }
                        "height" => {
                            new_decls.push(("height".into(), normalize_size(&value)));
                        }
                        "align" => {
                            if tag_name.eq_ignore_ascii_case("table") {
                                if value.eq_ignore_ascii_case("center") {
                                    new_decls.push(("margin-left".into(), "auto".into()));
                                    new_decls.push(("margin-right".into(), "auto".into()));
                                } else {
                                    new_decls.push(("float".into(), value));
                                }
                            } else {
                                new_decls.push(("text-align".into(), value));
                            }
                        }
                        "valign" => {
                            new_decls.push(("vertical-align".into(), value));
                        }
                        _ => {}
                    }
                }

                if new_decls.is_empty() {
                    return Ok(());
                }

                let existing = el.get_attribute("style").unwrap_or_default();
                let merged = merge_styles(&existing, &new_decls);
                el.set_attribute("style", &merged)
                    .map_err(|e| format!("{e}"))?;

                Ok(())
            })],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

/// Append `px` to numeric values; leave percentages and other units as-is.
fn normalize_size(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.ends_with('%') {
        return trimmed.to_string();
    }
    if trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') && !trimmed.is_empty() {
        return format!("{trimmed}px");
    }
    trimmed.to_string()
}

/// Merge new declarations into an existing style attribute string.
/// Existing declarations win — new ones are only added if the property
/// isn't already set.
fn merge_styles(existing: &str, new_decls: &[(String, String)]) -> String {
    let existing_props: Vec<String> = existing
        .split(';')
        .filter_map(|d| d.split_once(':').map(|(p, _)| p.trim().to_lowercase()))
        .collect();

    let mut declarations: Vec<String> = existing
        .split(';')
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .collect();

    for (prop, value) in new_decls {
        if !existing_props.contains(&prop.to_lowercase()) {
            declarations.push(format!("{prop}: {value}"));
        }
    }

    declarations.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgcolor_to_background_color() {
        let html = r##"<td bgcolor="#fff">Hi</td>"##;
        let result = process(html, &["bgcolor"]).unwrap();
        assert!(result.contains("style=\"background-color: #fff\""));
    }

    #[test]
    fn test_background_to_image() {
        let html = r#"<td background="bg.png">Hi</td>"#;
        let result = process(html, &["background"]).unwrap();
        assert!(result.contains("background-image: url('bg.png')"));
    }

    #[test]
    fn test_width_numeric_gets_px() {
        let html = r#"<table width="600"><tr></tr></table>"#;
        let result = process(html, &["width"]).unwrap();
        assert!(result.contains("style=\"width: 600px\""));
    }

    #[test]
    fn test_width_percent_unchanged() {
        let html = r#"<table width="100%"><tr></tr></table>"#;
        let result = process(html, &["width"]).unwrap();
        assert!(result.contains("width: 100%"));
        // Should NOT have "100%px"
        assert!(!result.contains("%px"));
    }

    #[test]
    fn test_align_center_on_table_uses_margin() {
        let html = r#"<table align="center"><tr></tr></table>"#;
        let result = process(html, &["align"]).unwrap();
        assert!(result.contains("margin-left: auto"));
        assert!(result.contains("margin-right: auto"));
    }

    #[test]
    fn test_align_on_td_uses_text_align() {
        let html = r#"<td align="center">Hi</td>"#;
        let result = process(html, &["align"]).unwrap();
        assert!(result.contains("text-align: center"));
    }

    #[test]
    fn test_valign_to_vertical_align() {
        let html = r#"<td valign="top">Hi</td>"#;
        let result = process(html, &["valign"]).unwrap();
        assert!(result.contains("vertical-align: top"));
    }

    #[test]
    fn test_existing_style_preserved() {
        let html = r##"<td bgcolor="#fff" style="color: red">Hi</td>"##;
        let result = process(html, &["bgcolor"]).unwrap();
        assert!(result.contains("color: red"));
        assert!(result.contains("background-color: #fff"));
    }

    #[test]
    fn test_existing_property_wins() {
        // If style already has background-color, attribute is NOT added
        let html = r##"<td bgcolor="#fff" style="background-color: red">Hi</td>"##;
        let result = process(html, &["bgcolor"]).unwrap();
        assert!(result.contains("background-color: red"));
        // The bgcolor #fff should NOT have been merged in
        assert!(!result.contains("background-color: #fff"));
    }

    #[test]
    fn test_no_attributes_unchanged() {
        let html = r#"<td>Hi</td>"#;
        let result = process(html, &["bgcolor"]).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_empty_attributes_list() {
        let html = r##"<td bgcolor="#fff">Hi</td>"##;
        let result = process(html, &[]).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_multiple_attributes() {
        let html = r##"<td bgcolor="#fff" width="200" valign="top">Hi</td>"##;
        let result = process(html, &["bgcolor", "width", "valign"]).unwrap();
        assert!(result.contains("background-color: #fff"));
        assert!(result.contains("width: 200px"));
        assert!(result.contains("vertical-align: top"));
    }
}
