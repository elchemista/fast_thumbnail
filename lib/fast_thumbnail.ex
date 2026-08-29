defmodule FastThumbnail do
  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :fast_thumbnail,
    crate: "fast_thumbnail",
    base_url: "https://github.com/elchemista/fast_thumbnail/releases/download/v#{version}",
    force_build: System.get_env("RUSTLER_PRECOMPILATION_EXAMPLE_BUILD") in ["1", "true"],
    nif_versions: ["2.15", "2.16"],
    version: version

  # use Rustler,
  #   otp_app: :fast_thumbnail,
  #   crate: "fast_thumbnail"

  @moduledoc "README.md"
             |> File.read!()

  @output_modes [:base64, :webp, :overwrite]
  @single_dimension_fits [:cover, :contain, :stretch, :width, :height]
  @box_fits [:cover, :contain, :stretch]
  @max_u32 4_294_967_295

  @type output_mode :: :base64 | :webp | :overwrite
  @type fit :: :cover | :contain | :stretch | :width | :height
  @type resize_option :: {:fit, fit()} | {:upscale, boolean()}
  @type result :: {:ok, String.t()} | {:error, String.t()}

  @doc """
  Creates a square thumbnail, preserving the legacy center-crop behavior.

  This is equivalent to:

      create(path, size, size, mode, fit: :cover)
  """
  @spec create(path :: String.t(), size :: pos_integer(), mode :: output_mode()) :: result()
  def create(path, size, mode)
      when is_binary(path) and is_integer(size) and size > 0 and size <= @max_u32 and
             mode in @output_modes do
    nif_create(path, size, Atom.to_string(mode))
    |> format_result(mode)
  end

  @doc """
  Creates a thumbnail using one primary dimension or a destination box.

  Use `fit: :width` to calculate the height automatically, or `fit: :height`
  to calculate the width automatically. The other fit modes use `size` as both
  dimensions of a square destination box.

  When called as `create(path, width, height, mode)`, the default fit mode is
  `:cover`: proportions are preserved and the source is center-cropped to
  produce the exact requested dimensions.

  ## Examples

      FastThumbnail.create("photo.jpg", 100, :webp, fit: :width)
      FastThumbnail.create("photo.jpg", 40, :webp, fit: :height)
      FastThumbnail.create("photo.jpg", 100, :webp, fit: :width, upscale: false)
  """
  @spec create(
          path :: String.t(),
          size :: pos_integer(),
          mode :: output_mode(),
          options :: [resize_option()]
        ) :: result()
  def create(path, size, mode, options)
      when is_binary(path) and is_integer(size) and size > 0 and size <= @max_u32 and
             mode in @output_modes and is_list(options) do
    create_with_options(path, size, size, mode, options, @single_dimension_fits)
  end

  @spec create(
          path :: String.t(),
          width :: pos_integer(),
          height :: pos_integer(),
          mode :: output_mode()
        ) :: result()
  def create(path, width, height, mode)
      when is_binary(path) and is_integer(width) and width > 0 and width <= @max_u32 and
             is_integer(height) and height > 0 and height <= @max_u32 and
             mode in @output_modes do
    create(path, width, height, mode, [])
  end

  @doc """
  Creates a thumbnail with a destination width, height and resize options.

  Supported fit modes:

    * `:cover` preserves proportions, center-crops and returns exactly
      `width x height`.
    * `:contain` preserves proportions and fits inside the given bounds without
      cropping. One output dimension can therefore be smaller than requested.
    * `:stretch` returns exactly `width x height`, allowing distortion.

  Set `upscale: false` to prevent the output from being enlarged beyond the
  source image. In that case, the output can be smaller than the requested box.
  """
  @spec create(
          path :: String.t(),
          width :: pos_integer(),
          height :: pos_integer(),
          mode :: output_mode(),
          options :: [resize_option()]
        ) :: result()
  def create(path, width, height, mode, options)
      when is_binary(path) and is_integer(width) and width > 0 and width <= @max_u32 and
             is_integer(height) and height > 0 and height <= @max_u32 and
             mode in @output_modes and is_list(options) do
    create_with_options(path, width, height, mode, options, @box_fits)
  end

  defp create_with_options(path, width, height, mode, options, allowed_fits) do
    with {:ok, options} <- validate_options(options, allowed_fits) do
      nif_create_with_options(
        path,
        width,
        height,
        Atom.to_string(mode),
        Atom.to_string(options[:fit]),
        options[:upscale]
      )
      |> format_result(mode)
    end
  end

  defp format_result({:ok, result}, :base64),
    do: {:ok, "data:image/webp;base64,#{result}"}

  defp format_result(result, _mode), do: result

  defp validate_options(options, allowed_fits) do
    with {:ok, options} <- validate_option_names(options),
         :ok <- validate_fit(options[:fit], allowed_fits),
         :ok <- validate_upscale(options[:upscale]) do
      {:ok, options}
    end
  end

  defp validate_option_names(options) do
    case Keyword.validate(options, fit: :cover, upscale: true) do
      {:ok, options} -> {:ok, options}
      {:error, unknown} -> {:error, "Unknown resize options: #{inspect(unknown)}"}
    end
  end

  defp validate_fit(fit, allowed_fits) do
    if fit in allowed_fits do
      :ok
    else
      {:error, "Unsupported fit mode: #{inspect(fit)}"}
    end
  end

  defp validate_upscale(upscale) when is_boolean(upscale), do: :ok

  defp validate_upscale(upscale),
    do: {:error, "upscale must be a boolean, got: #{inspect(upscale)}"}

  defp nif_create(_path, _width, _mode),
    do: :erlang.nif_error(:nif_not_loaded)

  defp nif_create_with_options(_path, _width, _height, _mode, _fit, _upscale),
    do: :erlang.nif_error(:nif_not_loaded)
end
