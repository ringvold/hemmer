defmodule Hemmer do
  @moduledoc """
  Rust-powered email HTML transformation pipeline.

  Takes HTML — optionally with Tailwind utility classes — and runs it through
  a pipeline of transformations to produce email-client-ready output: CSS
  inlining, table attribute defaults, Outlook conditional comments, `rem` to
  `px` conversion, and more.

  ## Functions

  - `process_tailwind/1` — generates Tailwind CSS from classes, inlines it,
    applies all email transforms
  - `process_tailwind_minified/1` — same with HTML minification
  - `process/1` — for HTML that already has CSS (no Tailwind generation)
  - `inline_css/1` — only inline CSS, no other transforms

  ## Example

      html = \"\"\"
      <html><head></head><body>
        <table>
          <tr>
            <td class="p-6 bg-indigo-600 text-white">
              <h1 class="text-xl font-bold">Hello!</h1>
            </td>
          </tr>
        </table>
      </body></html>
      \"\"\"

      {:ok, result} = Hemmer.process_tailwind(html)

  """

  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :hemmer,
    crate: "hemmer_nif",
    base_url: "https://github.com/ringvold/hemmer/releases/download/v#{version}",
    force_build: System.get_env("HEMMER_BUILD") in ["1", "true"],
    version: version,
    targets:
      ~w(
        aarch64-apple-darwin
        aarch64-unknown-linux-gnu
        aarch64-unknown-linux-musl
        x86_64-apple-darwin
        x86_64-unknown-linux-gnu
        x86_64-unknown-linux-musl
      ),
    nif_versions: ["2.15", "2.16", "2.17"]

  @doc """
  Process HTML with Tailwind utility classes into email-ready HTML.

  Generates CSS from Tailwind classes, inlines it, and applies all
  email-client compatibility transformations.
  """
  def process_tailwind(_html), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Same as `process_tailwind/1` but with HTML minification.
  """
  def process_tailwind_minified(_html), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Process email HTML that already has CSS in `<style>` blocks.

  Applies all email-client compatibility transformations but does not
  generate Tailwind CSS.
  """
  def process(_html), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Only inline CSS from `<style>` blocks into element `style` attributes.

  No other transformations are applied.
  """
  def inline_css(_html), do: :erlang.nif_error(:nif_not_loaded)
end
