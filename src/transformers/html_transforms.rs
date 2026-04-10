use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Run HTML-level email transformations in a single lol_html pass.
pub fn process(
    html: &str,
    default_attributes: bool,
    six_digit_hex: bool,
) -> Result<String, Error> {
    let mut handlers = Vec::new();

    if default_attributes {
        // Tables: cellpadding="0" cellspacing="0" role="none"
        handlers.push(element!("table", |el| {
            if el.get_attribute("cellpadding").is_none() {
                el.set_attribute("cellpadding", "0")
                    .map_err(|e| format!("{e}"))?;
            }
            if el.get_attribute("cellspacing").is_none() {
                el.set_attribute("cellspacing", "0")
                    .map_err(|e| format!("{e}"))?;
            }
            if el.get_attribute("role").is_none() {
                el.set_attribute("role", "none")
                    .map_err(|e| format!("{e}"))?;
            }
            Ok(())
        }));

        // Images: ensure alt attribute exists
        handlers.push(element!("img", |el| {
            if el.get_attribute("alt").is_none() {
                el.set_attribute("alt", "").map_err(|e| format!("{e}"))?;
            }
            Ok(())
        }));
    }

    if six_digit_hex {
        // Convert 3-digit hex to 6-digit in bgcolor and color attributes
        handlers.push(element!("[bgcolor]", |el| {
            if let Some(val) = el.get_attribute("bgcolor") {
                if let Some(expanded) = expand_short_hex(&val) {
                    el.set_attribute("bgcolor", &expanded)
                        .map_err(|e| format!("{e}"))?;
                }
            }
            Ok(())
        }));

        handlers.push(element!("[color]", |el| {
            if let Some(val) = el.get_attribute("color") {
                if let Some(expanded) = expand_short_hex(&val) {
                    el.set_attribute("color", &expanded)
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

/// Expand a 3-digit hex color to 6 digits.
/// `#abc` → `#aabbcc`, `#fff` → `#ffffff`
fn expand_short_hex(hex: &str) -> Option<String> {
    let trimmed = hex.trim();
    if trimmed.len() == 4 && trimmed.starts_with('#') {
        let chars: Vec<char> = trimmed[1..].chars().collect();
        if chars.iter().all(|c| c.is_ascii_hexdigit()) {
            return Some(format!(
                "#{}{}{}{}{}{}",
                chars[0], chars[0], chars[1], chars[1], chars[2], chars[2]
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_short_hex() {
        assert_eq!(expand_short_hex("#fff"), Some("#ffffff".to_string()));
        assert_eq!(expand_short_hex("#abc"), Some("#aabbcc".to_string()));
        assert_eq!(expand_short_hex("#FFFFFF"), None); // already 6 digits
        assert_eq!(expand_short_hex("red"), None); // not hex
    }

    #[test]
    fn test_default_table_attributes() {
        let html = "<table><tr><td>Hello</td></tr></table>";
        let result = process(html, true, false).unwrap();
        assert!(result.contains("cellpadding=\"0\""));
        assert!(result.contains("cellspacing=\"0\""));
        assert!(result.contains("role=\"none\""));
    }

    #[test]
    fn test_img_alt() {
        let html = r#"<img src="logo.png">"#;
        let result = process(html, true, false).unwrap();
        assert!(result.contains("alt=\"\""));
    }

    #[test]
    fn test_six_digit_hex() {
        let html = "<td bgcolor=\"#fff\">Hi</td>";
        let result = process(html, false, true).unwrap();
        assert!(result.contains("bgcolor=\"#ffffff\""));
    }
}
