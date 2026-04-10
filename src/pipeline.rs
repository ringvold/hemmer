use crate::error::Error;
use crate::transformers;

/// Result of processing an email through the pipeline.
pub struct PipelineResult {
    pub html: String,
}

/// Configurable email HTML transformation pipeline.
///
/// Transformations run in a fixed, email-optimized order:
/// 0. Tailwind CSS generation (scan HTML classes → generate + inject CSS)
/// 1. Outlook tags (`<outlook>` → MSO conditional comments)
/// 2. Safe class names (rewrite `/`, `:`, etc.)
/// 3. CSS inlining (style blocks → inline styles)
/// 4. Resolve CSS variables (`var(--x)` → static value)
/// 5. Resolve calc() expressions
/// 6. Email CSS compatibility (rem→px, logical→physical properties)
/// 7. Style → attribute (copy CSS width/bgcolor/etc to HTML attrs for Outlook)
/// 8. Default attributes (table cellpadding, img alt, etc.)
/// 9. Six-digit hex colors
/// 10. Prevent widows (&nbsp; between last two words)
/// 11. Class cleanup (remove inlined classes not in @media rules)
/// 12. Base URL (resolve relative paths)
/// 13. URL parameters (UTM tracking)
/// 14. Meta tags (DOCTYPE, charset, viewport)
/// 15. Minify HTML
pub struct Pipeline {
    tailwind: Option<encre_css::Config>,
    outlook_tags: bool,
    safe_class_names: bool,
    attribute_to_style: Vec<String>,
    inline_css: Option<transformers::inline_css::InlineCssConfig>,
    resolve_props: bool,
    resolve_calc: bool,
    email_compat_css: bool,
    style_to_attr: bool,
    default_attributes: bool,
    six_digit_hex: bool,
    prevent_widows: bool,
    class_cleanup: bool,
    purge_css: bool,
    base_url: Option<String>,
    url_params: Vec<(String, String)>,
    meta_tags: bool,
    remove_attributes: Vec<transformers::remove_attributes::RemoveRule>,
    minify: bool,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            tailwind: None,
            outlook_tags: true,
            safe_class_names: true,
            attribute_to_style: Vec::new(),
            inline_css: Some(transformers::inline_css::InlineCssConfig::default()),
            resolve_props: true,
            resolve_calc: true,
            email_compat_css: true,
            style_to_attr: true,
            default_attributes: true,
            six_digit_hex: true,
            prevent_widows: true,
            class_cleanup: true,
            purge_css: false,
            base_url: None,
            url_params: Vec::new(),
            meta_tags: true,
            remove_attributes: transformers::remove_attributes::default_rules(),
            minify: false,
        }
    }
}

impl Pipeline {
    /// Create a pipeline with sensible email defaults (no Tailwind generation).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a pipeline with Tailwind CSS generation enabled.
    /// This scans HTML for Tailwind classes, generates CSS, injects it,
    /// then runs the full email transformation pipeline.
    pub fn with_tailwind() -> Self {
        Self {
            tailwind: Some(transformers::tailwind::email_config()),
            ..Self::default()
        }
    }

    /// Create a minimal pipeline with everything disabled.
    pub fn minimal() -> Self {
        Self {
            tailwind: None,
            outlook_tags: false,
            safe_class_names: false,
            attribute_to_style: Vec::new(),
            inline_css: None,
            resolve_props: false,
            resolve_calc: false,
            email_compat_css: false,
            style_to_attr: false,
            default_attributes: false,
            six_digit_hex: false,
            prevent_widows: false,
            class_cleanup: false,
            purge_css: false,
            base_url: None,
            url_params: Vec::new(),
            meta_tags: false,
            remove_attributes: Vec::new(),
            minify: false,
        }
    }

    pub fn tailwind(mut self, config: encre_css::Config) -> Self {
        self.tailwind = Some(config);
        self
    }

    pub fn no_tailwind(mut self) -> Self {
        self.tailwind = None;
        self
    }

    pub fn outlook_tags(mut self, enabled: bool) -> Self {
        self.outlook_tags = enabled;
        self
    }

    pub fn safe_class_names(mut self, enabled: bool) -> Self {
        self.safe_class_names = enabled;
        self
    }

    /// Enable attribute_to_style for the given HTML attributes.
    /// Each named attribute will be duplicated into the inline `style` attribute.
    /// Pass an empty list to disable.
    pub fn attribute_to_style(mut self, attributes: Vec<String>) -> Self {
        self.attribute_to_style = attributes;
        self
    }

    pub fn inline_css(mut self, config: transformers::inline_css::InlineCssConfig) -> Self {
        self.inline_css = Some(config);
        self
    }

    pub fn no_inline_css(mut self) -> Self {
        self.inline_css = None;
        self
    }

    pub fn resolve_props(mut self, enabled: bool) -> Self {
        self.resolve_props = enabled;
        self
    }

    pub fn resolve_calc(mut self, enabled: bool) -> Self {
        self.resolve_calc = enabled;
        self
    }

    pub fn email_compat_css(mut self, enabled: bool) -> Self {
        self.email_compat_css = enabled;
        self
    }

    pub fn style_to_attr(mut self, enabled: bool) -> Self {
        self.style_to_attr = enabled;
        self
    }

    pub fn default_attributes(mut self, enabled: bool) -> Self {
        self.default_attributes = enabled;
        self
    }

    pub fn six_digit_hex(mut self, enabled: bool) -> Self {
        self.six_digit_hex = enabled;
        self
    }

    pub fn prevent_widows(mut self, enabled: bool) -> Self {
        self.prevent_widows = enabled;
        self
    }

    pub fn class_cleanup(mut self, enabled: bool) -> Self {
        self.class_cleanup = enabled;
        self
    }

    /// Enable purging of unused CSS rules from `<style>` blocks.
    /// Off by default because the Tailwind generator already produces
    /// only the used CSS — enable when working with hand-written stylesheets.
    pub fn purge_css(mut self, enabled: bool) -> Self {
        self.purge_css = enabled;
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn url_params(mut self, params: Vec<(String, String)>) -> Self {
        self.url_params = params;
        self
    }

    pub fn meta_tags(mut self, enabled: bool) -> Self {
        self.meta_tags = enabled;
        self
    }

    /// Replace the attribute removal rules.
    /// Default rules remove empty `style` and `class` attributes.
    pub fn remove_attributes(
        mut self,
        rules: Vec<transformers::remove_attributes::RemoveRule>,
    ) -> Self {
        self.remove_attributes = rules;
        self
    }

    /// Add an additional attribute removal rule on top of the existing ones.
    pub fn add_remove_rule(
        mut self,
        rule: transformers::remove_attributes::RemoveRule,
    ) -> Self {
        self.remove_attributes.push(rule);
        self
    }

    pub fn minify(mut self, enabled: bool) -> Self {
        self.minify = enabled;
        self
    }

    /// Process HTML through the configured transformation pipeline.
    pub fn process(&self, html: &str) -> Result<PipelineResult, Error> {
        let mut output = html.to_string();

        // 0. Tailwind CSS generation — scan HTML, generate CSS, inject <style>
        if let Some(ref config) = self.tailwind {
            output = transformers::tailwind::process(&output, config)?;
        }

        // 1. Outlook tags — convert before any HTML parsing that might choke on them
        if self.outlook_tags {
            output = transformers::outlook_tags::process(&output)?;
        }

        // 2. Safe class names — before inlining so class selectors still match
        if self.safe_class_names {
            output = transformers::safe_class_names::process(&output)?;
        }

        // 2b. Attribute → style — must run before inlining so the CSS gets
        //     considered alongside other inline styles
        if !self.attribute_to_style.is_empty() {
            let attrs: Vec<&str> = self
                .attribute_to_style
                .iter()
                .map(|s| s.as_str())
                .collect();
            output = transformers::attribute_to_style::process(&output, &attrs)?;
        }

        // 3. CSS inlining — the core email transformation
        if let Some(ref config) = self.inline_css {
            output = transformers::inline_css::process(&output, config)?;
        }

        // 4. Resolve CSS variables — must run after inlining so vars in inline styles are caught
        if self.resolve_props {
            output = transformers::resolve_props::process(&output)?;
        }

        // 5. Resolve calc() — after var() resolution since calc might use vars
        if self.resolve_calc {
            output = transformers::resolve_calc::process(&output)?;
        }

        // 6. Email CSS compatibility — after calc resolution so we have static values
        if self.email_compat_css {
            output = transformers::email_compat_css::process(&output)?;
        }

        // 7. Style → attribute — Outlook fallback (must run after email_compat_css
        //    so width values are in px not rem)
        if self.style_to_attr {
            output = transformers::style_to_attr::process(&output)?;
        }

        // 8-9. HTML transformations via lol_html (single pass)
        if self.default_attributes || self.six_digit_hex {
            output = transformers::html_transforms::process(
                &output,
                self.default_attributes,
                self.six_digit_hex,
            )?;
        }

        // 10. Prevent widows
        if self.prevent_widows {
            output = transformers::widows::process(&output)?;
        }

        // 11. Class cleanup — remove classes that are no longer needed
        if self.class_cleanup {
            output = transformers::class_cleanup::process(&output)?;
        }

        // 11b. Purge CSS — strip rules from <style> blocks that don't match
        //      any element. Runs after class_cleanup so the usage scan
        //      reflects the final state of the HTML.
        if self.purge_css {
            output = transformers::purge_css::process(&output)?;
        }

        // 11c. Remove attributes — must run after class_cleanup so empty classes are caught
        if !self.remove_attributes.is_empty() {
            output = transformers::remove_attributes::process(&output, &self.remove_attributes)?;
        }

        // 12. Base URL — resolve relative paths
        if let Some(ref base) = self.base_url {
            output = transformers::base_url::process(&output, base)?;
        }

        // 13. URL parameters — append tracking params
        if !self.url_params.is_empty() {
            output = transformers::url_params::process(&output, &self.url_params)?;
        }

        // 14. Meta tags — inject DOCTYPE and standard meta tags
        if self.meta_tags {
            output = transformers::meta_tags::process(&output)?;
        }

        // 15. Minify — always last
        if self.minify {
            output = transformers::minify::process(&output)?;
        }

        Ok(PipelineResult { html: output })
    }
}
