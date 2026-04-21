# hemmer

A Rust pipeline for transforming HTML into email-client-ready output.

`hemmer` takes HTML — optionally with Tailwind utility classes — and runs it
through a configurable set of transformations that paper over the quirks of
email clients: CSS inlining, table attribute defaults, Outlook conditional
comments, `rem` → `px` conversion, CSS variable resolution, and a long list
of smaller fixes.

## Status

Early but functional. 151 tests passing. The API will probably keep shifting
until it stabilizes around a `0.x` release.

## Origin

`hemmer` was built to enable HTML + Tailwind email templates as a
replacement for [MJML](https://mjml.io) in an Elixir/Phoenix app. MJML
makes it hard to picture what you'll end up with until it's compiled, and
LLM-based assistants struggle with its custom markup — both humans and
models are far more comfortable in plain HTML and Tailwind.

[Maizzle](https://maizzle.com) was the original inspiration for the
transformer pipeline, but it brings a Node.js dependency and a templating
layer we didn't need (HEEx already handles that), and we wanted runtime
processing instead of a build step. Building this in Rust gave us all of
that and a clean Elixir NIF integration via
[Rustler](https://github.com/rusterlium/rustler).

The name *hemmer* is a sewing term — a machine attachment that folds the
edge of fabric to create a clean hem. Fitting for a tool that takes raw
HTML and gives it a clean edge for the limitations of email clients.

## What it does

`hemmer` runs a pipeline of independent transformers. Most are enabled by
default; a few are opt-in. Enabled transformers run in a fixed, email-optimized
order.

### CSS generation and inlining

- **`tailwind`** — runtime Tailwind CSS generation via
  [encre-css](https://gitlab.com/encre-org/encre-css), with email-safe hex
  color overrides for all 22 Tailwind v3 color scales (no `oklch()`)
- **`safe_class_names`** — rewrites Tailwind escaped characters
  (`\:`, `\/`, `[`, `]`, `%`, `#`, …) to email-safe equivalents in both
  `class` attributes and `<style>` tag contents
- **`inline_css`** — moves `<style>` rules into element `style` attributes
  via [css-inline](https://github.com/Stranger6667/css-inline)
- **`class_cleanup`** — removes inlined classes from elements while
  preserving classes referenced by `@media` queries
- **`purge_css`** — removes unused rules from `<style>` blocks (opt-in;
  the Tailwind generator already produces only used CSS)

### Email-client compatibility

- **`email_compat_css`** — converts `rem` → `px` and CSS logical properties
  (`padding-inline`, `margin-block`, …) to physical equivalents. Outlook,
  Gmail, and Yahoo support neither.
- **`resolve_props`** — replaces `var(--name)` references with the static
  values from `:root` declarations. Outlook desktop doesn't support custom
  properties.
- **`resolve_calc`** — evaluates `calc()` expressions with same-unit
  arithmetic to constants. Outlook desktop doesn't support `calc()`.
- **`style_to_attr`** — copies CSS `width` / `height` / `bgcolor` / `align`
  values into HTML attributes. Outlook desktop's Word renderer often honors
  HTML attributes when it ignores CSS.
- **`attribute_to_style`** — the opposite direction: copies HTML
  presentational attributes into inline CSS for modern clients (opt-in).

### HTML defaults and cleanup

- **`html_transforms`** — adds `cellpadding="0" cellspacing="0" role="none"`
  to tables, ensures `<img>` has an `alt` attribute, expands 3-digit hex
  colors in `bgcolor` and `color` attributes
- **`outlook_tags`** — `<outlook>` and `<not-outlook>` tags become MSO
  conditional comments. Supports version names (`only="2013"` →
  `[if mso 15]`) and the full `only` / `not` / `lt` / `lte` / `gt` / `gte`
  attribute set.
- **`widows`** — inserts `&nbsp;` between the last two words in elements
  marked with `prevent-widows` or `no-widows`
- **`base_url`** — resolves relative URLs in HTML attributes (`src`, `href`,
  `srcset`, …) and CSS `url()` values inside both inline styles and
  `<style>` tags
- **`url_params`** — appends UTM/tracking parameters to absolute URLs in
  `<a>` tags
- **`meta_tags`** — auto-injects DOCTYPE, charset, viewport, and
  format-detection meta tags if missing
- **`remove_attributes`** — configurable removal with empty / always-remove
  / exact-value / regex-match rules
- **`minify`** — final HTML minification (opt-in)

## What it doesn't do

`hemmer` is **not** a templating engine. Bring your own — HEEx, Tera,
MiniJinja, plain string formatting, anything. The pipeline runs *after*
templating, on a complete HTML string.

It also doesn't:

- generate plaintext versions of emails (do this in the layer above)
- send the email (use Swoosh, lettre, etc.)
- convert MJML markup (it's an alternative to MJML, not a converter)

## Usage

```rust
use hemmer::Pipeline;

let html = r#"
<html>
<head></head>
<body>
  <table>
    <tr>
      <td class="p-6 bg-indigo-600 text-white text-center">
        <h1 class="text-xl font-bold">Welcome!</h1>
      </td>
    </tr>
  </table>
</body>
</html>
"#;

let result = Pipeline::with_tailwind().process(html)?;
println!("{}", result.html);
```

This generates Tailwind CSS for the classes used in the document, inlines
it, applies all the email-compatibility transforms, and injects DOCTYPE and
meta tags.

For a minimal pipeline that only does what you opt into:

```rust
use hemmer::{Pipeline, InlineCssConfig};

let result = Pipeline::minimal()
    .inline_css(InlineCssConfig::default())
    .minify(true)
    .process(html)?;
```

The full builder API lets you toggle every transformer individually. See
[`src/pipeline.rs`](src/pipeline.rs) for the available methods.

## Using from Elixir

`hemmer` is available on Hex as [`:hemmer`](https://hex.pm/packages/hemmer):

```elixir
defp deps do
  [
    {:hemmer, "~> 0.1"}
  ]
end
```

Then use it directly from Elixir:

```elixir
html = """
<html><head></head><body>
  <table>
    <tr>
      <td class="p-6 bg-indigo-600 text-white text-center">
        <h1 class="text-xl font-bold">Welcome!</h1>
      </td>
    </tr>
  </table>
</body></html>
"""

{:ok, result} = Hemmer.process_tailwind(html)
# Or, for HTML that already includes CSS:
{:ok, result} = Hemmer.process(html)
```

## Inspired by

- **[Maizzle](https://maizzle.com)** — for the transformer-pipeline
  architecture and a long list of email-client gotchas to work around.
  Several transformers in `hemmer` mirror Maizzle's defaults closely.
- **[encre-css](https://gitlab.com/encre-org/encre-css)** — runtime Tailwind
  CSS generation in Rust, which makes the whole pipeline possible without a
  Node-based build step.
- **[css-inline](https://github.com/Stranger6667/css-inline)** — the actual
  CSS inlining engine, built on Servo's CSS parser.
- **[lol_html](https://github.com/cloudflare/lol-html)** — Cloudflare's
  streaming HTML rewriter, used everywhere `hemmer` needs to manipulate the
  DOM.

## License

MIT
