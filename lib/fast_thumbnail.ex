defmodule FastThumbnail do
  use Rustler,
    otp_app: :fast_thumbnail,
    crate: "fast_thumbnail"

  @doc """
  Creates a thumbnail (width x width), cropping to center, and returning {:ok, path} or {:error, reason}.

  Modes:
  - `:base64`: return a base64-encoded WebP (no file I/O)
  - `:webp`:   write a new `"{path}.webp"` file
  - `:overwrite`:   overwrite in the *original format* (JPEG→JPEG, PNG→PNG, etc.)

  Example:
  ```elixir
  FastThumbnail.create("path/to/image.jpg", 200, :webp)
  # => {:ok, "path/to/image.webp"}

  FastThumbnail.create("path/to/image.jpg", 200, :overwrite)
  # => {:ok, "path/to/image.jpg"}

  FastThumbnail.create("path/to/image.jpg", 200, :base64)
  # => {:ok, "data:image/webp;base64,iVBORw0KGgoAAAANSUhEUgAAA..."}
  ```

  """
  @spec create(path :: String.t(), width :: integer(), mode :: atom()) ::
          {:ok, String.t()} | {:error, String.t()}

  def create(path, width, :base64)
      when is_bitstring(path) and is_integer(width) do
    nif_create(path, width, "base64")
    |> case do
      {:ok, b64} -> {:ok, "data:image/webp;base64,#{b64}"}
      {:error, reason} -> {:error, reason}
    end
  end

  def create(path, width, mode)
      when is_bitstring(path) and is_integer(width) and mode in [:webp, :overwrite] do
    nif_create(path, width, to_string(mode))
  end

  defp nif_create(_path, _width, _mode),
    do: :erlang.nif_error(:nif_not_loaded)
end
