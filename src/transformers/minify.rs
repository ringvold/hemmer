use crate::error::Error;

pub fn process(html: &str) -> Result<String, Error> {
    let cfg = minify_html::Cfg {
        minify_css: true,
        keep_spaces_between_attributes: true,
        ..Default::default()
    };

    let minified = minify_html::minify(html.as_bytes(), &cfg);
    String::from_utf8(minified).map_err(|_| Error::Minify)
}
