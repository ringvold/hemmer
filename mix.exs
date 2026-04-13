defmodule Hemmer.MixProject do
  use Mix.Project

  @version "0.1.0"
  @source_url "https://github.com/ringvold/hemmer"

  def project do
    [
      app: :hemmer,
      version: @version,
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Rust-powered email HTML transformation pipeline",
      source_url: @source_url,
      docs: [main: "readme", extras: ["README.md"]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:rustler, ">= 0.0.0", optional: true},
      {:rustler_precompiled, "~> 0.8"},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false}
    ]
  end

  defp package do
    [
      files: [
        "lib",
        "native/hemmer_nif/src",
        "native/hemmer_nif/Cargo*",
        "src",
        "Cargo.toml",
        "Cargo.lock",
        "checksum-*.exs",
        "mix.exs",
        "README.md",
        "LICENSE"
      ],
      licenses: ["MIT"],
      links: %{"GitHub" => @source_url},
      maintainers: ["Harald Ringvold"]
    ]
  end
end
