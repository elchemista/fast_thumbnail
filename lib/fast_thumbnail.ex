defmodule FastThumbnail do
  use Rustler,
    otp_app: :fast_thumbnail,
    crate: "fast_thumbnail"

  @doc """
  Creates a thumbnail (width x width), cropping to center, overwriting the
  file in-place and returning {:ok, path} or {:error, reason}.
  """
  @spec create(path :: String.t(), width :: integer()) :: {:ok, String.t()} | {:error, String.t()}
  def create(path, width) when is_bitstring(path) and is_integer(width) do
    nif_create(path, width)
  end

  defp nif_create(_path, _width),
    do: :erlang.nif_error(:nif_not_loaded)
end
