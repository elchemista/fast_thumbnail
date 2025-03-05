defmodule FastThumbnailTest do
  use ExUnit.Case, async: true

  @tag :base64
  test "creating a base64 webp from a JPEG" do
    result = FastThumbnail.create("test/images/test_1.jpeg", 200, :base64)

    assert {:ok, data} = result
    assert String.starts_with?(data, "data:image/webp;base64,")
    assert byte_size(data) > 50
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
end
