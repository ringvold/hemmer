use crate::error::Error;
use css_inline::CSSInliner;

/// Configuration for CSS inlining.
pub struct InlineCssConfig {
    /// Keep `<style>` tags after inlining (for media queries, etc.)
    pub keep_style_tags: bool,
    /// Convert CSS width/height to HTML attributes on img/video
    pub apply_width_attributes: bool,
    pub apply_height_attributes: bool,
    /// Extra CSS to inject before inlining
    pub extra_css: Option<String>,
    /// Keep `@media` rules in a `<style>` block
    pub keep_link_tags: bool,
}

impl Default for InlineCssConfig {
    fn default() -> Self {
        Self {
            keep_style_tags: true,
            apply_width_attributes: true,
            apply_height_attributes: true,
            extra_css: None,
            keep_link_tags: false,
        }
    }
}

pub fn process(html: &str, config: &InlineCssConfig) -> Result<String, Error> {
    let mut builder = CSSInliner::options()
        .keep_style_tags(config.keep_style_tags)
        .keep_link_tags(config.keep_link_tags);

    if let Some(ref extra) = config.extra_css {
        builder = builder.extra_css(Some(extra.as_str().into()));
    }

    let inliner = builder.build();
    Ok(inliner.inline(html)?)
}
