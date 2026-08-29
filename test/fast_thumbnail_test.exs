defmodule FastThumbnailTest do
  use ExUnit.Case, async: true

  import Bitwise

  @portrait_path "test/images/test_4.jpeg"

  @tag :base64
  test "creating a base64 webp from a JPEG" do
    result = FastThumbnail.create("test/images/test_1.jpeg", 200, :base64)

    assert {:ok, data} = result
    assert String.starts_with?(data, "data:image/webp;base64,")
    assert byte_size(data) > 50
    assert webp_dimensions(data) == {200, 200}
  end

  describe "proportional resizing" do
    test "calculates height from width" do
      assert {:ok, data} =
               FastThumbnail.create(@portrait_path, 100, :base64, fit: :width)

      assert webp_dimensions(data) == {100, 178}
    end

    test "calculates width from height" do
      assert {:ok, data} =
               FastThumbnail.create(@portrait_path, 100, :base64, fit: :height)

      assert webp_dimensions(data) == {56, 100}
    end

    test "can disable upscaling" do
      assert {:ok, data} =
               FastThumbnail.create(@portrait_path, 1_000, :base64,
                 fit: :width,
                 upscale: false
               )

      assert webp_dimensions(data) == {640, 1136}
    end
  end

  describe "destination box resizing" do
    test "cover returns the exact dimensions" do
      assert {:ok, data} = FastThumbnail.create(@portrait_path, 300, 200, :base64)
      assert webp_dimensions(data) == {300, 200}
    end

    test "contain preserves proportions without cropping" do
      assert {:ok, data} =
               FastThumbnail.create(@portrait_path, 300, 200, :base64, fit: :contain)

      assert webp_dimensions(data) == {113, 200}
    end

    test "stretch returns the exact dimensions" do
      assert {:ok, data} =
               FastThumbnail.create(@portrait_path, 300, 200, :base64, fit: :stretch)

      assert webp_dimensions(data) == {300, 200}
    end
  end

  describe "resize option validation" do
    test "rejects unknown resize options" do
      assert {:error, "Unknown resize options: [:quality]"} =
               FastThumbnail.create(@portrait_path, 100, :base64,
                 fit: :width,
                 quality: 80
               )
    end

    test "rejects unknown fit modes" do
      assert {:error, "Unsupported fit mode: :unknown"} =
               FastThumbnail.create(@portrait_path, 100, :base64, fit: :unknown)
    end

    test "rejects single-dimension fit modes for a destination box" do
      assert {:error, "Unsupported fit mode: :width"} =
               FastThumbnail.create(@portrait_path, 300, 200, :base64, fit: :width)

      assert {:error, "Unsupported fit mode: :height"} =
               FastThumbnail.create(@portrait_path, 300, 200, :base64, fit: :height)
    end

    test "rejects invalid upscale values" do
      assert {:error, "upscale must be a boolean, got: :sometimes"} =
               FastThumbnail.create(@portrait_path, 100, :base64,
                 fit: :width,
                 upscale: :sometimes
               )
    end
  end

  @tag :webp
  test "creating a new .webp file from JPEG" do
    source_path = "test/images/test_4.jpeg"
    webp_path = "#{source_path}.webp"

    File.rm(webp_path)

    result = FastThumbnail.create(source_path, 300, :webp)

    assert {:ok, ^webp_path} = result
    assert File.exists?(webp_path) == true

    File.rm(webp_path)
  end

  @tag :webp_unknown
  test "creating a new .webp file from unknown format" do
    source_path = "test/images/test_2"
    webp_path = "#{source_path}.webp"

    File.rm(webp_path)

    result = FastThumbnail.create(source_path, 300, :webp)

    assert {:ok, ^webp_path} = result
    assert File.exists?(webp_path) == true

    File.rm(webp_path)
  end

  @tag :overwrite
  test "overwriting a file in original format" do
    source_path = "test/images/test_1.jpeg"
    temp_path = "test/images/test_overwrite_temp.jpeg"

    File.cp!(source_path, temp_path)

    result = FastThumbnail.create(temp_path, 250, :overwrite)
    assert {:ok, ^temp_path} = result

    assert File.exists?(temp_path)

    File.rm(temp_path)
  end

  defp webp_dimensions("data:image/webp;base64," <> encoded) do
    encoded
    |> Base.decode64!()
    |> parse_webp_dimensions()
  end

  defp parse_webp_dimensions(<<"RIFF", _size::little-32, "WEBP", chunks::binary>>) do
    find_dimensions_chunk(chunks)
  end

  defp find_dimensions_chunk(
         <<fourcc::binary-size(4), size::little-32, chunk::binary-size(size), rest::binary>>
       ) do
    case dimensions_from_chunk(fourcc, chunk) do
      nil -> find_dimensions_chunk(skip_padding(rest, size))
      dimensions -> dimensions
    end
  end

  defp dimensions_from_chunk(
         "VP8 ",
         <<_frame_tag::binary-size(3), 0x9D, 0x01, 0x2A, width::little-16, height::little-16,
           _rest::binary>>
       ) do
    {band(width, 0x3FFF), band(height, 0x3FFF)}
  end

  defp dimensions_from_chunk(
         "VP8L",
         <<0x2F, b1, b2, b3, b4, _rest::binary>>
       ) do
    width = 1 + b1 + (band(b2, 0x3F) <<< 8)
    height = 1 + (b2 >>> 6) + (b3 <<< 2) + (band(b4, 0x0F) <<< 10)
    {width, height}
  end

  defp dimensions_from_chunk(
         "VP8X",
         <<_flags, _reserved::binary-size(3), width::little-24, height::little-24, _rest::binary>>
       ) do
    {width + 1, height + 1}
  end

  defp dimensions_from_chunk(_fourcc, _chunk), do: nil

  defp skip_padding(<<_padding, rest::binary>>, size) when rem(size, 2) == 1, do: rest
  defp skip_padding(rest, _size), do: rest
end
