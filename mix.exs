defmodule FastThumbnail.MixProject do
  use Mix.Project

  @version "0.1.6"

  def project do
    [
      app: :fast_thumbnail,
      name: "Fast Thumbnail - Generate Thumbnails using Rust",
      version: @version,
      elixir: "~> 1.18",
      build_embedded: Mix.env() == :prod,
      start_permanent: Mix.env() == :prod,
      test_coverage: [summary: [threshold: 90]],
      deps: deps(),
      description: description(),
      package: package(),
      rustler_precompiled: [
        provider: :github,
        owner: "elchemista",
        repo: "fast_thumbnail",
        tag: "v#{@version}"
      ],
      docs: [
        main: "readme",
        extras: [
          "README.md",
          "LICENSE"
        ]
      ],
      source_url: "https://github.com/elchemista/fast_thumbnail",
      homepage_url: "https://github.com/elchemista/fast_thumbnail"
    ]
  end

  def application do
    [
      extra_applications: []
    ]
  end

  defp description() do
    "Generate Thumbnails using rust library for fast image resizing using of SIMD instructions."
  end

  defp package() do
    [
      name: "fast_thumbnail",
      maintainers: ["Yuriy Zhar"],
      files: ~w(
        lib
        mix.exs
        README.md
        LICENSE
        checksum-*.exs
        native/fast_thumbnail/Cargo.toml
        native/fast_thumbnail/Cargo.lock
        native/fast_thumbnail/src
      ),
      licenses: ["Apache-2.0"],
      links: %{
        "GitHub" => "https://github.com/elchemista/fast_thumbnail"
      }
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler, "~> 0.38.0", optional: true},
      {:credo, "~> 1.7.19", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:rustler_precompiled, "~> 0.9.0"},
      # Documentation Provider
      {:ex_doc, "~> 0.40.3", only: [:dev, :test], optional: true, runtime: false}
    ]
  end
end
