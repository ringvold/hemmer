use crate::error::Error;
use lol_html::{element, HtmlRewriter, Settings};

/// Generate Tailwind CSS from HTML class names and inject it into the document.
///
/// Scans the HTML for Tailwind utility classes, generates only the CSS needed,
/// and injects a `<style>` block into `<head>` (or before `</html>` as fallback).
pub fn process(html: &str, config: &encre_css::Config) -> Result<String, Error> {
    // 1. Generate CSS from the HTML content
    let css = encre_css::generate([html], config);

    if css.trim().is_empty() {
        return Ok(html.to_string());
    }

    // 2. Inject the generated CSS into the HTML
    inject_style(html, &css)
}

/// Inject a `<style>` block into the HTML `<head>`.
fn inject_style(html: &str, css: &str) -> Result<String, Error> {
    let style_tag = format!("<style>{css}</style>");
    let mut output = Vec::new();
    let mut injected = false;

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                // Prefer injecting at end of <head>
                element!("head", |el| {
                    if !injected {
                        el.append(&style_tag, lol_html::html_content::ContentType::Html);
                        injected = true;
                    }
                    Ok(())
                }),
            ],
            ..Settings::new()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );

    rewriter
        .write(html.as_bytes())
        .map_err(|e| Error::HtmlRewrite(e.to_string()))?;
    rewriter
        .end()
        .map_err(|e| Error::HtmlRewrite(e.to_string()))?;

    let mut result =
        String::from_utf8(output).map_err(|e| Error::HtmlRewrite(e.to_string()))?;

    // Fallback: if no <head> was found, prepend the style tag
    if !injected {
        result = format!("{style_tag}{result}");
    }

    Ok(result)
}

/// Create a default encre-css config suitable for email.
///
/// - Preflight disabled (email clients have their own resets)
/// - All Tailwind colors overridden with hex values (oklch is not supported by email clients)
pub fn email_config() -> encre_css::Config {
    let mut config = encre_css::Config::default();

    config.preflight = encre_css::Preflight::None;

    // Override Tailwind v4 oklch colors with email-safe hex values.
    // These are the Tailwind v3 hex equivalents.
    let colors = &mut config.theme.colors;

    // Slate
    for (shade, hex) in [
        ("slate-50", "#f8fafc"), ("slate-100", "#f1f5f9"), ("slate-200", "#e2e8f0"),
        ("slate-300", "#cbd5e1"), ("slate-400", "#94a3b8"), ("slate-500", "#64748b"),
        ("slate-600", "#475569"), ("slate-700", "#334155"), ("slate-800", "#1e293b"),
        ("slate-900", "#0f172a"), ("slate-950", "#020617"),
    ] { colors.add(shade, hex); }

    // Gray
    for (shade, hex) in [
        ("gray-50", "#f9fafb"), ("gray-100", "#f3f4f6"), ("gray-200", "#e5e7eb"),
        ("gray-300", "#d1d5db"), ("gray-400", "#9ca3af"), ("gray-500", "#6b7280"),
        ("gray-600", "#4b5563"), ("gray-700", "#374151"), ("gray-800", "#1f2937"),
        ("gray-900", "#111827"), ("gray-950", "#030712"),
    ] { colors.add(shade, hex); }

    // Zinc
    for (shade, hex) in [
        ("zinc-50", "#fafafa"), ("zinc-100", "#f4f4f5"), ("zinc-200", "#e4e4e7"),
        ("zinc-300", "#d4d4d8"), ("zinc-400", "#a1a1aa"), ("zinc-500", "#71717a"),
        ("zinc-600", "#52525b"), ("zinc-700", "#3f3f46"), ("zinc-800", "#27272a"),
        ("zinc-900", "#18181b"), ("zinc-950", "#09090b"),
    ] { colors.add(shade, hex); }

    // Neutral
    for (shade, hex) in [
        ("neutral-50", "#fafafa"), ("neutral-100", "#f5f5f5"), ("neutral-200", "#e5e5e5"),
        ("neutral-300", "#d4d4d4"), ("neutral-400", "#a3a3a3"), ("neutral-500", "#737373"),
        ("neutral-600", "#525252"), ("neutral-700", "#404040"), ("neutral-800", "#262626"),
        ("neutral-900", "#171717"), ("neutral-950", "#0a0a0a"),
    ] { colors.add(shade, hex); }

    // Red
    for (shade, hex) in [
        ("red-50", "#fef2f2"), ("red-100", "#fee2e2"), ("red-200", "#fecaca"),
        ("red-300", "#fca5a5"), ("red-400", "#f87171"), ("red-500", "#ef4444"),
        ("red-600", "#dc2626"), ("red-700", "#b91c1c"), ("red-800", "#991b1b"),
        ("red-900", "#7f1d1d"), ("red-950", "#450a0a"),
    ] { colors.add(shade, hex); }

    // Orange
    for (shade, hex) in [
        ("orange-50", "#fff7ed"), ("orange-100", "#ffedd5"), ("orange-200", "#fed7aa"),
        ("orange-300", "#fdba74"), ("orange-400", "#fb923c"), ("orange-500", "#f97316"),
        ("orange-600", "#ea580c"), ("orange-700", "#c2410c"), ("orange-800", "#9a3412"),
        ("orange-900", "#7c2d12"), ("orange-950", "#431407"),
    ] { colors.add(shade, hex); }

    // Amber
    for (shade, hex) in [
        ("amber-50", "#fffbeb"), ("amber-100", "#fef3c7"), ("amber-200", "#fde68a"),
        ("amber-300", "#fcd34d"), ("amber-400", "#fbbf24"), ("amber-500", "#f59e0b"),
        ("amber-600", "#d97706"), ("amber-700", "#b45309"), ("amber-800", "#92400e"),
        ("amber-900", "#78350f"), ("amber-950", "#451a03"),
    ] { colors.add(shade, hex); }

    // Yellow
    for (shade, hex) in [
        ("yellow-50", "#fefce8"), ("yellow-100", "#fef9c3"), ("yellow-200", "#fef08a"),
        ("yellow-300", "#fde047"), ("yellow-400", "#facc15"), ("yellow-500", "#eab308"),
        ("yellow-600", "#ca8a04"), ("yellow-700", "#a16207"), ("yellow-800", "#854d0e"),
        ("yellow-900", "#713f12"), ("yellow-950", "#422006"),
    ] { colors.add(shade, hex); }

    // Green
    for (shade, hex) in [
        ("green-50", "#f0fdf4"), ("green-100", "#dcfce7"), ("green-200", "#bbf7d0"),
        ("green-300", "#86efac"), ("green-400", "#4ade80"), ("green-500", "#22c55e"),
        ("green-600", "#16a34a"), ("green-700", "#15803d"), ("green-800", "#166534"),
        ("green-900", "#14532d"), ("green-950", "#052e16"),
    ] { colors.add(shade, hex); }

    // Emerald
    for (shade, hex) in [
        ("emerald-50", "#ecfdf5"), ("emerald-100", "#d1fae5"), ("emerald-200", "#a7f3d0"),
        ("emerald-300", "#6ee7b7"), ("emerald-400", "#34d399"), ("emerald-500", "#10b981"),
        ("emerald-600", "#059669"), ("emerald-700", "#047857"), ("emerald-800", "#065f46"),
        ("emerald-900", "#064e3b"), ("emerald-950", "#022c22"),
    ] { colors.add(shade, hex); }

    // Teal
    for (shade, hex) in [
        ("teal-50", "#f0fdfa"), ("teal-100", "#ccfbf1"), ("teal-200", "#99f6e4"),
        ("teal-300", "#5eead4"), ("teal-400", "#2dd4bf"), ("teal-500", "#14b8a6"),
        ("teal-600", "#0d9488"), ("teal-700", "#0f766e"), ("teal-800", "#115e59"),
        ("teal-900", "#134e4a"), ("teal-950", "#042f2e"),
    ] { colors.add(shade, hex); }

    // Cyan
    for (shade, hex) in [
        ("cyan-50", "#ecfeff"), ("cyan-100", "#cffafe"), ("cyan-200", "#a5f3fc"),
        ("cyan-300", "#67e8f9"), ("cyan-400", "#22d3ee"), ("cyan-500", "#06b6d4"),
        ("cyan-600", "#0891b2"), ("cyan-700", "#0e7490"), ("cyan-800", "#155e75"),
        ("cyan-900", "#164e63"), ("cyan-950", "#083344"),
    ] { colors.add(shade, hex); }

    // Sky
    for (shade, hex) in [
        ("sky-50", "#f0f9ff"), ("sky-100", "#e0f2fe"), ("sky-200", "#bae6fd"),
        ("sky-300", "#7dd3fc"), ("sky-400", "#38bdf8"), ("sky-500", "#0ea5e9"),
        ("sky-600", "#0284c7"), ("sky-700", "#0369a1"), ("sky-800", "#075985"),
        ("sky-900", "#0c4a6e"), ("sky-950", "#082f49"),
    ] { colors.add(shade, hex); }

    // Blue
    for (shade, hex) in [
        ("blue-50", "#eff6ff"), ("blue-100", "#dbeafe"), ("blue-200", "#bfdbfe"),
        ("blue-300", "#93c5fd"), ("blue-400", "#60a5fa"), ("blue-500", "#3b82f6"),
        ("blue-600", "#2563eb"), ("blue-700", "#1d4ed8"), ("blue-800", "#1e40af"),
        ("blue-900", "#1e3a8a"), ("blue-950", "#172554"),
    ] { colors.add(shade, hex); }

    // Indigo
    for (shade, hex) in [
        ("indigo-50", "#eef2ff"), ("indigo-100", "#e0e7ff"), ("indigo-200", "#c7d2fe"),
        ("indigo-300", "#a5b4fc"), ("indigo-400", "#818cf8"), ("indigo-500", "#6366f1"),
        ("indigo-600", "#4f46e5"), ("indigo-700", "#4338ca"), ("indigo-800", "#3730a3"),
        ("indigo-900", "#312e81"), ("indigo-950", "#1e1b4e"),
    ] { colors.add(shade, hex); }

    // Violet
    for (shade, hex) in [
        ("violet-50", "#f5f3ff"), ("violet-100", "#ede9fe"), ("violet-200", "#ddd6fe"),
        ("violet-300", "#c4b5fd"), ("violet-400", "#a78bfa"), ("violet-500", "#8b5cf6"),
        ("violet-600", "#7c3aed"), ("violet-700", "#6d28d9"), ("violet-800", "#5b21b6"),
        ("violet-900", "#4c1d95"), ("violet-950", "#2e1065"),
    ] { colors.add(shade, hex); }

    // Purple
    for (shade, hex) in [
        ("purple-50", "#faf5ff"), ("purple-100", "#f3e8ff"), ("purple-200", "#e9d5ff"),
        ("purple-300", "#d8b4fe"), ("purple-400", "#c084fc"), ("purple-500", "#a855f7"),
        ("purple-600", "#9333ea"), ("purple-700", "#7e22ce"), ("purple-800", "#6b21a8"),
        ("purple-900", "#581c87"), ("purple-950", "#3b0764"),
    ] { colors.add(shade, hex); }

    // Fuchsia
    for (shade, hex) in [
        ("fuchsia-50", "#fdf4ff"), ("fuchsia-100", "#fae8ff"), ("fuchsia-200", "#f5d0fe"),
        ("fuchsia-300", "#f0abfc"), ("fuchsia-400", "#e879f9"), ("fuchsia-500", "#d946ef"),
        ("fuchsia-600", "#c026d3"), ("fuchsia-700", "#a21caf"), ("fuchsia-800", "#86198f"),
        ("fuchsia-900", "#701a75"), ("fuchsia-950", "#4a044e"),
    ] { colors.add(shade, hex); }

    // Pink
    for (shade, hex) in [
        ("pink-50", "#fdf2f8"), ("pink-100", "#fce7f3"), ("pink-200", "#fbcfe8"),
        ("pink-300", "#f9a8d4"), ("pink-400", "#f472b6"), ("pink-500", "#ec4899"),
        ("pink-600", "#db2777"), ("pink-700", "#be185d"), ("pink-800", "#9d174d"),
        ("pink-900", "#831843"), ("pink-950", "#500724"),
    ] { colors.add(shade, hex); }

    // Rose
    for (shade, hex) in [
        ("rose-50", "#fff1f2"), ("rose-100", "#ffe4e6"), ("rose-200", "#fecdd3"),
        ("rose-300", "#fda4af"), ("rose-400", "#fb7185"), ("rose-500", "#f43f5e"),
        ("rose-600", "#e11d48"), ("rose-700", "#be123c"), ("rose-800", "#9f1239"),
        ("rose-900", "#881337"), ("rose-950", "#4c0519"),
    ] { colors.add(shade, hex); }

    config
}

/// Apply dark-mode auto-inversion protection to a config.
///
/// Some email clients (notably Gmail and Apple Mail) automatically invert
/// "pure" black and white when the user is in dark mode. This can break
/// carefully designed emails: a white background flips to black but the
/// rest of your colors don't, leaving the design inconsistent.
///
/// This helper overrides Tailwind's `black` and `white` color values with
/// near-pure equivalents that the auto-inversion algorithm doesn't
/// recognize as "pure" — visually identical to humans, but the dark mode
/// detector skips them and your design renders as you intended.
///
/// **Note:** This intentionally fights the user's dark mode preference.
/// Use it for transactional emails where design fidelity matters; consider
/// leaving it off for content emails where dark mode adaptation is fine.
///
/// ```
/// let mut config = hemmer::tailwind_email_config();
/// hemmer::transformers::tailwind::add_dark_mode_protection(&mut config);
/// ```
pub fn add_dark_mode_protection(config: &mut encre_css::Config) {
    config.theme.colors.add("black", "#000001");
    config.theme.colors.add("white", "#fffffe");
}

/// Add email-friendly screen breakpoints (`sm` and `xs`) to a config.
///
/// Adds:
/// - `sm` = 600px (the email industry standard mobile breakpoint)
/// - `xs` = 430px (smaller phones)
///
/// Use them with the `max-` prefix variant to get max-width semantics
/// (the desktop-first responsive approach typical for emails):
///
/// ```text
/// <div class="text-base max-sm:text-sm">Responsive text</div>
/// ```
///
/// **Note:** encre-css generates the modern CSS range query syntax
/// (`@media (width < 600px)`). For maximum email-client compatibility you
/// may want to post-process the output to use legacy `(max-width: 600px)`
/// syntax — see `email_compat_css` for the wider compatibility-fix layer.
///
/// ```
/// let mut config = hemmer::tailwind_email_config();
/// hemmer::transformers::tailwind::add_email_screens(&mut config);
/// ```
pub fn add_email_screens(config: &mut encre_css::Config) {
    config.theme.screens.add("sm", "600px");
    config.theme.screens.add("xs", "430px");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate(html: &str, config: &encre_css::Config) -> String {
        encre_css::generate([html], config)
    }

    // ─── Default email_config sanity checks ────────────────────

    #[test]
    fn test_email_config_uses_hex_colors() {
        let config = email_config();
        let css = generate(r#"<div class="bg-indigo-600">x</div>"#, &config);
        assert!(css.contains("#4f46e5"), "should use hex, got: {css}");
        assert!(!css.contains("oklch"), "should not contain oklch");
    }

    #[test]
    fn test_email_config_no_preflight() {
        let config = email_config();
        let css = generate(r#"<div class="text-sm">x</div>"#, &config);
        // Preflight would inject reset CSS at the top — we should not see it
        assert!(!css.contains("box-sizing: border-box"));
    }

    // ─── Dark mode protection (opt-in) ─────────────────────────

    #[test]
    fn test_dark_mode_protection_changes_black() {
        let mut config = email_config();
        add_dark_mode_protection(&mut config);
        let css = generate(r#"<div class="text-black">x</div>"#, &config);
        assert!(css.contains("#000001"), "expected #000001, got: {css}");
        // Should NOT contain pure #000 or #000000
        assert!(!css.contains("#000;") && !css.contains("#000000"));
    }

    #[test]
    fn test_dark_mode_protection_changes_white() {
        let mut config = email_config();
        add_dark_mode_protection(&mut config);
        let css = generate(r#"<div class="bg-white">x</div>"#, &config);
        assert!(css.contains("#fffffe"), "expected #fffffe, got: {css}");
        assert!(!css.contains("#fff;") && !css.contains("#ffffff;"));
    }

    #[test]
    fn test_dark_mode_protection_not_applied_by_default() {
        // Without the helper, encre-css uses its default black/white
        let config = email_config();
        let css = generate(r#"<div class="text-black bg-white">x</div>"#, &config);
        // The default should NOT contain our trick values
        assert!(!css.contains("#000001"));
        assert!(!css.contains("#fffffe"));
    }

    // ─── Email screens (opt-in) ────────────────────────────────

    #[test]
    fn test_email_screens_sm_value() {
        let mut config = email_config();
        add_email_screens(&mut config);
        // Use max-sm: which produces a max-width media query in encre-css
        let css = generate(r#"<div class="max-sm:text-sm">x</div>"#, &config);
        // The screen value 600px should appear in a media query
        assert!(css.contains("600px"), "expected 600px breakpoint, got: {css}");
        assert!(css.contains("@media"), "expected media query, got: {css}");
    }

    #[test]
    fn test_email_screens_xs_value() {
        let mut config = email_config();
        add_email_screens(&mut config);
        let css = generate(r#"<div class="max-xs:text-xs">x</div>"#, &config);
        assert!(css.contains("430px"), "expected 430px breakpoint, got: {css}");
    }

    #[test]
    fn test_email_screens_not_applied_by_default() {
        // Without the helper, encre-css uses Tailwind's default 40rem (640px) for sm
        let config = email_config();
        let css = generate(r#"<div class="max-sm:text-sm">x</div>"#, &config);
        // Default sm is 40rem, NOT 600px
        assert!(!css.contains("600px"));
    }

    #[test]
    fn test_helpers_can_be_combined() {
        let mut config = email_config();
        add_dark_mode_protection(&mut config);
        add_email_screens(&mut config);

        let css = generate(
            r#"<div class="text-black max-sm:bg-white">x</div>"#,
            &config,
        );
        // Both helpers should be active
        assert!(css.contains("#000001"));
        assert!(css.contains("#fffffe"));
        assert!(css.contains("600px"));
    }
}
