use crate::error::Error;
use lol_html::{element, rewrite_str, text, RewriteStrSettings};

/// Resolve simple `calc()` expressions to static values.
///
/// Handles arithmetic with same-unit values:
/// - `calc(10px + 5px)` → `15px`
/// - `calc(100% - 20%)` → `80%`
/// - `calc(2 * 10px)` → `20px`
/// - `calc(20px / 2)` → `10px`
///
/// Leaves mixed-unit expressions unchanged (e.g., `calc(100% - 20px)`)
/// since these can't be evaluated without runtime context.
///
/// Outlook desktop doesn't support calc(), so this is needed for compatibility.
pub fn process(html: &str) -> Result<String, Error> {
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("[style]", |el| {
                    if let Some(style) = el.get_attribute("style") {
                        let resolved = resolve_calcs(&style);
                        if resolved != style {
                            el.set_attribute("style", &resolved)
                                .map_err(|e| format!("{e}"))?;
                        }
                    }
                    Ok(())
                }),
                text!("style", |chunk| {
                    let css = chunk.as_str();
                    let resolved = resolve_calcs(css);
                    if resolved != css {
                        chunk.replace(&resolved, lol_html::html_content::ContentType::Text);
                    }
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| Error::HtmlRewrite(e.to_string()))
}

/// Find and resolve all calc() expressions in a CSS string.
fn resolve_calcs(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(idx) = remaining.find("calc(") {
        result.push_str(&remaining[..idx]);
        let after_open = &remaining[idx + 5..];

        // Find matching close paren (handle nesting)
        let Some(close_pos) = find_matching_paren(after_open) else {
            result.push_str("calc(");
            remaining = after_open;
            continue;
        };

        let inner = &after_open[..close_pos];
        let after_close = &after_open[close_pos + 1..];

        if let Some(resolved) = try_evaluate(inner) {
            result.push_str(&resolved);
        } else {
            // Couldn't resolve — leave as-is
            result.push_str("calc(");
            result.push_str(inner);
            result.push(')');
        }

        remaining = after_close;
    }

    result.push_str(remaining);
    result
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Try to evaluate a calc expression body. Returns None if it can't be resolved.
///
/// Supports:
/// - Single value: `10px` → `10px`
/// - Binary ops with same unit: `10px + 5px` → `15px`
/// - Multiplication/division by unitless: `2 * 10px`, `10px / 2`
/// - Nested calc: handled by outer pass
fn try_evaluate(expr: &str) -> Option<String> {
    let expr = expr.trim();

    // Tokenize
    let tokens = tokenize(expr)?;
    if tokens.is_empty() {
        return None;
    }

    // Parse and evaluate (left-to-right with operator precedence)
    let result = parse_expression(&tokens)?;
    Some(format_value(result))
}

#[derive(Debug, Clone)]
enum Token {
    Value(f64, String), // (number, unit)
    Op(char),
    Open,
    Close,
}

fn tokenize(s: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '+' || c == '-' || c == '*' || c == '/' {
            // Could be a sign, not an operator, if at start or after another op
            let is_sign = (c == '+' || c == '-')
                && matches!(tokens.last(), None | Some(Token::Op(_)) | Some(Token::Open));
            if !is_sign {
                tokens.push(Token::Op(c));
                chars.next();
                continue;
            }
        }

        if c == '(' {
            tokens.push(Token::Open);
            chars.next();
            continue;
        }
        if c == ')' {
            tokens.push(Token::Close);
            chars.next();
            continue;
        }

        // Parse a number (with optional sign and unit)
        let mut num_str = String::new();
        if let Some(&'+') | Some(&'-') = chars.peek() {
            num_str.push(chars.next().unwrap());
        }
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                num_str.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if num_str.is_empty() || num_str == "+" || num_str == "-" {
            return None;
        }
        let num: f64 = num_str.parse().ok()?;

        // Parse unit
        let mut unit = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() || c == '%' {
                unit.push(c);
                chars.next();
            } else {
                break;
            }
        }

        tokens.push(Token::Value(num, unit));
    }

    Some(tokens)
}

/// Parse and evaluate using a simple recursive descent parser with precedence.
fn parse_expression(tokens: &[Token]) -> Option<(f64, String)> {
    let (result, rest) = parse_add(tokens)?;
    if rest.is_empty() {
        Some(result)
    } else {
        None
    }
}

fn parse_add(tokens: &[Token]) -> Option<((f64, String), &[Token])> {
    let (mut left, mut rest) = parse_mul(tokens)?;
    while let Some(Token::Op(op)) = rest.first() {
        if *op != '+' && *op != '-' {
            break;
        }
        let op = *op;
        let (right, new_rest) = parse_mul(&rest[1..])?;
        left = combine_add_sub(left, right, op)?;
        rest = new_rest;
    }
    Some((left, rest))
}

fn parse_mul(tokens: &[Token]) -> Option<((f64, String), &[Token])> {
    let (mut left, mut rest) = parse_atom(tokens)?;
    while let Some(Token::Op(op)) = rest.first() {
        if *op != '*' && *op != '/' {
            break;
        }
        let op = *op;
        let (right, new_rest) = parse_atom(&rest[1..])?;
        left = combine_mul_div(left, right, op)?;
        rest = new_rest;
    }
    Some((left, rest))
}

fn parse_atom(tokens: &[Token]) -> Option<((f64, String), &[Token])> {
    match tokens.first()? {
        Token::Value(n, u) => Some(((*n, u.clone()), &tokens[1..])),
        Token::Open => {
            let (result, rest) = parse_add(&tokens[1..])?;
            if matches!(rest.first(), Some(Token::Close)) {
                Some((result, &rest[1..]))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn combine_add_sub(left: (f64, String), right: (f64, String), op: char) -> Option<(f64, String)> {
    // Both must have same unit (or one unitless)
    let unit = if left.1 == right.1 {
        left.1
    } else if left.1.is_empty() {
        right.1
    } else if right.1.is_empty() {
        left.1
    } else {
        return None; // Mixed units
    };

    let value = match op {
        '+' => left.0 + right.0,
        '-' => left.0 - right.0,
        _ => return None,
    };

    Some((value, unit))
}

fn combine_mul_div(left: (f64, String), right: (f64, String), op: char) -> Option<(f64, String)> {
    // For * and /, exactly one side must be unitless
    let (value, unit) = if left.1.is_empty() {
        (
            match op {
                '*' => left.0 * right.0,
                '/' => left.0 / right.0,
                _ => return None,
            },
            right.1,
        )
    } else if right.1.is_empty() {
        (
            match op {
                '*' => left.0 * right.0,
                '/' => left.0 / right.0,
                _ => return None,
            },
            left.1,
        )
    } else {
        return None; // Both have units
    };

    if !value.is_finite() {
        return None;
    }

    Some((value, unit))
}

fn format_value((value, unit): (f64, String)) -> String {
    if value == value.floor() && value.abs() < 1e15 {
        format!("{}{unit}", value as i64)
    } else {
        // Round to a reasonable precision
        let rounded = (value * 1000.0).round() / 1000.0;
        format!("{rounded}{unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_addition() {
        assert_eq!(resolve_calcs("calc(10px + 5px)"), "15px");
        assert_eq!(resolve_calcs("calc(100% + 20%)"), "120%");
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(resolve_calcs("calc(20px - 5px)"), "15px");
        assert_eq!(resolve_calcs("calc(100% - 25%)"), "75%");
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(resolve_calcs("calc(2 * 10px)"), "20px");
        assert_eq!(resolve_calcs("calc(10px * 3)"), "30px");
    }

    #[test]
    fn test_division() {
        assert_eq!(resolve_calcs("calc(20px / 2)"), "10px");
    }

    #[test]
    fn test_precedence() {
        assert_eq!(resolve_calcs("calc(10px + 2 * 5px)"), "20px");
    }

    #[test]
    fn test_mixed_units_unchanged() {
        // Mixed units can't be resolved
        let input = "calc(100% - 20px)";
        assert_eq!(resolve_calcs(input), input);
    }

    #[test]
    fn test_in_property() {
        assert_eq!(
            resolve_calcs("width: calc(50px + 10px); padding: calc(2 * 8px)"),
            "width: 60px; padding: 16px"
        );
    }

    #[test]
    fn test_no_calc_unchanged() {
        assert_eq!(resolve_calcs("color: red"), "color: red");
    }

    #[test]
    fn test_html_pipeline() {
        let html = r#"<div style="width: calc(100px + 50px);">Hi</div>"#;
        let result = process(html).unwrap();
        assert!(result.contains("width: 150px"));
        assert!(!result.contains("calc"));
    }
}
