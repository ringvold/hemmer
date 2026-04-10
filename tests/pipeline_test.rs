use hemmer::Pipeline;

const EMAIL_HTML: &str = r##"<!DOCTYPE html>
<html>
<head>
    <style>
        .text-lg { font-size: 18px; }
        .text-gray-600 { color: #666666; }
        .p-4 { padding: 16px; }
        .bg-blue-500 { background-color: #3b82f6; }
        .w-1\/2 { width: 50%; }
        @media (max-width: 600px) {
            .sm\:text-base { font-size: 16px; }
        }
    </style>
</head>
<body>
    <table>
        <tr>
            <td class="p-4 bg-blue-500" bgcolor="#fff">
                <h1 class="text-lg">Welcome!</h1>
                <p class="text-gray-600">Hello world</p>
                <img src="logo.png">
            </td>
        </tr>
    </table>
</body>
</html>"##;

#[test]
fn test_full_pipeline() {
    let result = Pipeline::new().process(EMAIL_HTML).unwrap();
    let html = &result.html;

    // CSS should be inlined
    assert!(html.contains("font-size: 18px"), "text-lg should be inlined");
    assert!(html.contains("padding: 16px"), "p-4 should be inlined");

    // Default table attributes
    assert!(html.contains("cellpadding=\"0\""), "table should have cellpadding");
    assert!(html.contains("cellspacing=\"0\""), "table should have cellspacing");
    assert!(html.contains("role=\"none\""), "table should have role=none");

    // img should have alt attribute
    assert!(html.contains("alt=\"\""), "img should have empty alt");

    // 3-digit hex should be expanded
    assert!(html.contains("#ffffff"), "#fff should become #ffffff");
    assert!(!html.contains("bgcolor=\"#fff\""), "short hex should be gone");
}

#[test]
fn test_safe_class_names() {
    let html = r##"<html><head><style>
        .w-1\/2 { width: 50%; }
        .sm\:text-base { font-size: 16px; }
    </style></head>
    <body><div class="w-1\/2 sm\:text-base">Test</div></body></html>"##;

    let result = Pipeline::new().no_inline_css().process(html).unwrap();

    // Class names should be safe (no slashes or colons)
    assert!(result.html.contains("w-1-2"), "slash should be replaced with dash");
    assert!(result.html.contains("sm-text-base"), "colon should be replaced with dash");
    assert!(!result.html.contains("\\/"), "escaped slash should be gone");
    assert!(!result.html.contains("\\:"), "escaped colon should be gone");
}

#[test]
fn test_minimal_pipeline() {
    let html = "<table><tr><td><img src=\"test.png\"></td></tr></table>";

    let result = Pipeline::minimal().process(html).unwrap();

    // Nothing should be transformed
    assert!(!result.html.contains("cellpadding"), "minimal should not add attributes");
    assert!(!result.html.contains("alt="), "minimal should not add alt");
}

#[test]
fn test_minification() {
    let html = r#"<html>
    <head></head>
    <body>
        <p>Hello    world</p>
    </body>
</html>"#;

    let result = Pipeline::minimal().minify(true).process(html).unwrap();

    // Should be smaller
    assert!(result.html.len() < html.len(), "minified should be smaller");
    assert!(result.html.contains("Hello"), "content should be preserved");
}

#[test]
fn test_pipeline_builder() {
    let result = Pipeline::minimal()
        .default_attributes(true)
        .six_digit_hex(true)
        .process("<table bgcolor=\"#abc\"><tr><td>Hi</td></tr></table>")
        .unwrap();

    assert!(result.html.contains("cellpadding=\"0\""));
    assert!(result.html.contains("#aabbcc"));
}

#[test]
fn test_tailwind_pipeline() {
    // HTML with Tailwind classes but NO <style> block — encre-css generates the CSS
    let html = r##"<html>
<head></head>
<body>
    <table>
        <tr>
            <td class="p-4 bg-blue-500 text-white">
                <h1 class="text-lg font-bold">Hello from Tailwind!</h1>
                <p class="text-gray-600 mt-2">This email was styled with Tailwind utilities.</p>
            </td>
        </tr>
    </table>
</body>
</html>"##;

    let result = Pipeline::with_tailwind().process(html).unwrap();
    let out = &result.html;

    // encre-css should have generated and inlined the CSS
    assert!(out.contains("padding:"), "p-4 should produce padding");
    assert!(out.contains("font-weight:"), "font-bold should produce font-weight");
    assert!(out.contains("cellpadding=\"0\""), "table should have default attrs");

    // Should NOT contain a raw class without inline style
    // (CSS was generated, injected, then inlined)
    assert!(out.contains("style="), "styles should be inlined");
}

#[test]
fn test_tailwind_generates_only_used_css() {
    let html = r#"<html><head></head><body><div class="p-4">Test</div></body></html>"#;

    let result = Pipeline::with_tailwind().no_inline_css().process(html).unwrap();

    // Should have a <style> block with p-4 CSS
    assert!(result.html.contains("<style>"), "should inject style tag");
    assert!(result.html.contains("padding:"), "should contain padding for p-4");
    // Should NOT contain CSS for unused classes
    assert!(!result.html.contains("bg-blue"), "should not contain unused class CSS");
}
