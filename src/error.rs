#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("CSS inlining failed: {0}")]
    CssInline(#[from] css_inline::InlineError),

    #[error("HTML rewriting failed: {0}")]
    HtmlRewrite(String),

    #[error("CSS parsing failed: {0}")]
    CssParse(String),

    #[error("HTML minification failed")]
    Minify,
}
