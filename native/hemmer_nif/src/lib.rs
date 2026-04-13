use hemmer::{InlineCssConfig, Pipeline};

/// Process email HTML with Tailwind CSS generation.
///
/// Takes raw HTML with Tailwind utility classes, generates only the needed CSS,
/// inlines it, and applies all email-specific transformations.
#[rustler::nif(schedule = "DirtyCpu")]
fn process_tailwind(html: &str) -> Result<String, String> {
    Pipeline::with_tailwind()
        .process(html)
        .map(|r| r.html)
        .map_err(|e| e.to_string())
}

/// Process email HTML with Tailwind CSS generation + minification.
#[rustler::nif(schedule = "DirtyCpu")]
fn process_tailwind_minified(html: &str) -> Result<String, String> {
    Pipeline::with_tailwind()
        .minify(true)
        .process(html)
        .map(|r| r.html)
        .map_err(|e| e.to_string())
}

/// Process email HTML that already has CSS (no Tailwind generation).
///
/// Applies: safe class names, CSS inlining, default table/img attributes,
/// 6-digit hex colors, empty attribute removal.
#[rustler::nif(schedule = "DirtyCpu")]
fn process(html: &str) -> Result<String, String> {
    Pipeline::new()
        .process(html)
        .map(|r| r.html)
        .map_err(|e| e.to_string())
}

/// Only inline CSS (no other transformations).
#[rustler::nif(schedule = "DirtyCpu")]
fn inline_css(html: &str) -> Result<String, String> {
    Pipeline::minimal()
        .inline_css(InlineCssConfig::default())
        .process(html)
        .map(|r| r.html)
        .map_err(|e| e.to_string())
}

rustler::init!("Elixir.Hemmer");
