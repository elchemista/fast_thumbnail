defmodule FastThumbnail.MixProject do
  use Mix.Project

  def project do
    [
      app: :fast_thumbnail,
      name: "Fast Thumbnail - Generate Thumbnails using Rust",
      version: "0.1.0",
      elixir: "~> 1.18",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      description: description(),
      package: package(),
      docs: [
        main: "Fast Thumbnail",
        extras: [
          "README.md",
          "LICENSE"
        ]
      ]
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp description() do
    "Generate Thumbnails using rust library for fast image resizing using of SIMD instructions."
  end

  defp package() do
    [
      licenses: ["MIT"],
      links: %{
        project: "https://github.com/elchemista/fast_thumbnail",
        developer_github: "https://github.com/elchemista"
      }
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler, "~> 0.36.1"},
      {:credo, "~> 1.6", only: [:dev, :test], runtime: false},
      # Documentation Provider
      {:ex_doc, "~> 0.28.3", only: [:dev, :test], optional: true, runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}
    ]
  end
end
