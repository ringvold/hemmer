use crate::error::Error;
use lol_html::{element, rewrite_str, RewriteStrSettings};
use regex::Regex;

/// A rule for removing attributes from HTML elements.
#[derive(Debug, Clone)]
pub enum RemoveRule {
    /// Remove the attribute when its value is empty (after trim).
    Empty(String),
    /// Remove the attribute regardless of value.
    Always(String),
    /// Remove the attribute when its value exactly matches the given string.
    ExactValue { name: String, value: String },
    /// Remove the attribute when its value matches the given regex.
    /// The regex is compiled once when the rule is created.
    Regex { name: String, pattern: Regex },
}

impl RemoveRule {
    pub fn empty(name: impl Into<String>) -> Self {
        Self::Empty(name.into())
    }

    pub fn always(name: impl Into<String>) -> Self {
        Self::Always(name.into())
    }

    pub fn exact(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::ExactValue {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn regex(name: impl Into<String>, pattern: &str) -> Result<Self, Error> {
        let pattern = Regex::new(pattern)
            .map_err(|e| Error::HtmlRewrite(format!("invalid regex: {e}")))?;
        Ok(Self::Regex {
            name: name.into(),
            pattern,
        })
    }

    fn attr_name(&self) -> &str {
        match self {
            Self::Empty(n) | Self::Always(n) => n,
            Self::ExactValue { name, .. } | Self::Regex { name, .. } => name,
        }
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Empty(_) => value.trim().is_empty(),
            Self::Always(_) => true,
            Self::ExactValue { value: v, .. } => value == v,
            Self::Regex { pattern, .. } => pattern.is_match(value),
        }
    }
}

/// Default rules: remove empty `style` and `class` attributes.
pub fn default_rules() -> Vec<RemoveRule> {
    vec![RemoveRule::empty("style"), RemoveRule::empty("class")]
}

/// Remove attributes from HTML elements based on the supplied rules.
pub fn process(html: &str, rules: &[RemoveRule]) -> Result<String, Error> {
    if rules.is_empty() {
        return Ok(html.to_string());
    }

    let mut handlers = Vec::new();

    for rule in rules {
        let rule = rule.clone();
        let selector = format!("[{}]", rule.attr_name());

        handlers.push(element!(selector, move |el| {
            let name = rule.attr_name();
            if let Some(value) = el.get_attribute(name) {
                if rule.matches(&value) {
                    el.remove_attribute(name);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_removes_empty_style_class() {
        let html = r#"<div style="" class="">Hi</div>"#;
        let result = process(html, &default_rules()).unwrap();
        assert!(!result.contains("style="));
        assert!(!result.contains("class="));
    }

    #[test]
    fn test_default_keeps_non_empty_style() {
        let html = r#"<div style="color: red">Hi</div>"#;
        let result = process(html, &default_rules()).unwrap();
        assert!(result.contains("style=\"color: red\""));
    }

    #[test]
    fn test_always_removes_attribute() {
        let html = r#"<div data-temp="anything">Hi</div>"#;
        let rules = vec![RemoveRule::always("data-temp")];
        let result = process(html, &rules).unwrap();
        assert!(!result.contains("data-temp"));
    }

    #[test]
    fn test_exact_value_match() {
        let html = r#"<a role="none">Hi</a><a role="button">Hi</a>"#;
        let rules = vec![RemoveRule::exact("role", "none")];
        let result = process(html, &rules).unwrap();
        // role="none" removed, role="button" kept
        assert!(!result.contains("role=\"none\""));
        assert!(result.contains("role=\"button\""));
    }

    #[test]
    fn test_exact_value_mismatch_keeps() {
        let html = r#"<a role="button">Hi</a>"#;
        let rules = vec![RemoveRule::exact("role", "none")];
        let result = process(html, &rules).unwrap();
        assert!(result.contains("role=\"button\""));
    }

    #[test]
    fn test_regex_match() {
        let html = r#"<div data-x="temp_123">Hi</div><div data-x="keep">Hi</div>"#;
        let rules = vec![RemoveRule::regex("data-x", "^temp_").unwrap()];
        let result = process(html, &rules).unwrap();
        assert!(!result.contains("temp_123"));
        assert!(result.contains("data-x=\"keep\""));
    }

    #[test]
    fn test_regex_invalid_pattern_errors() {
        let result = RemoveRule::regex("data-x", "[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_rules_unchanged() {
        let html = r#"<div style="" class="x">Hi</div>"#;
        let result = process(html, &[]).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_multiple_rules() {
        let html = r#"<div style="" class="" data-temp="x">Hi</div>"#;
        let mut rules = default_rules();
        rules.push(RemoveRule::always("data-temp"));
        let result = process(html, &rules).unwrap();
        assert!(!result.contains("style="));
        assert!(!result.contains("class="));
        assert!(!result.contains("data-temp"));
    }

    #[test]
    fn test_empty_rule_keeps_whitespace_value() {
        // "Empty after trim" — whitespace-only is considered empty
        let html = r#"<div style="   ">Hi</div>"#;
        let result = process(html, &default_rules()).unwrap();
        assert!(!result.contains("style="));
    }
}
