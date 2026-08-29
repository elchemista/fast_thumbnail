# FastThumbnail

[![CI](https://github.com/elchemista/fast_thumbnail/actions/workflows/ci.yml/badge.svg)](https://github.com/elchemista/fast_thumbnail/actions/workflows/ci.yml)

FastThumbnail is an Elixir library that uses a [Rust NIF](https://hexdocs.pm/rustler) under the hood to perform **fast, SIMD-optimized** image resizing. It leverages the excellent [fast_image_resize](https://github.com/Cykooz/fast_image_resize) crate to crop and resize images efficiently.

### Why?

- **Fast**: SIMD-accelerated resizing is blazingly fast.

I just wanted stupidly simple image resizing that can crop or proportionally resize an image to the requested dimensions.
There is already good libraries for this [image](https://github.com/elixir-image/image), but they all seem to be too complex for my needs. Also I like small dependencies.

## Features

- **Flexible sizing**: square thumbnails, explicit width and height, or automatic proportional sizing.
- **Multiple fit modes**: center-crop (`:cover`), fit without cropping (`:contain`), exact dimensions (`:stretch`), width-driven and height-driven scaling.
- Optionally prevent small source images from being enlarged with `upscale: false`.
- **Multiple output modes**:  
  - Overwrite the original file with the same format (JPEG stays JPEG, PNG stays PNG, etc.).  
  - Save a **new** file in WebP format (`"path.webp"`).  
  - Return the resized image as a **base64-encoded** WebP string (no file writing).
- Backed by **SIMD** operations for maximum performance, thanks to the [fast_image_resize](https://github.com/Cykooz/fast_image_resize) library.

## Installation

Add `fast_thumbnail` to your dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:fast_thumbnail, "~> 0.1.6"}
  ]
end
```

Then run:

```bash
mix deps.get
mix compile
```

## Usage

```elixir
# 1) Legacy square thumbnail with center crop
iex> FastThumbnail.create("images/photo.jpg", 300, :overwrite)
{:ok, "images/photo.jpg"}

# 2) Resize proportionally from a target width
# A 1000x400 source becomes 100x40
iex> FastThumbnail.create("images/photo.jpg", 100, :webp, fit: :width)
{:ok, "images/photo.jpg.webp"}

# 3) Resize proportionally from a target height
iex> FastThumbnail.create("images/photo.jpg", 40, :webp, fit: :height)
{:ok, "images/photo.jpg.webp"}

# 4) Produce an exact 300x200 image, preserving proportions with a center crop
iex> FastThumbnail.create("images/photo.jpg", 300, 200, :webp, fit: :cover)
{:ok, "images/photo.jpg.webp"}

# 5) Fit inside a 300x200 box, preserving proportions without cropping
iex> FastThumbnail.create("images/photo.jpg", 300, 200, :webp, fit: :contain)
{:ok, "images/photo.jpg.webp"}

# 6) Produce an exact 300x200 image, allowing distortion
iex> FastThumbnail.create("images/photo.jpg", 300, 200, :webp, fit: :stretch)
{:ok, "images/photo.jpg.webp"}

# 7) Return a base64-encoded WebP (no file writing)
iex> FastThumbnail.create("images/photo.jpg", 300, :base64, fit: :width)
{:ok, "data:image/webp;base64,UklGRrwAAABXRUJQVlA4T..."}

# 8) Preserve proportions and never enlarge a smaller source image
iex> FastThumbnail.create("images/photo.jpg", 1200, :webp, fit: :width, upscale: false)
{:ok, "images/photo.jpg.webp"}
```

### Fit modes

| Fit | Dimensions | Aspect ratio | Crop |
| --- | --- | --- | --- |
| `:width` | Exact width, automatic height | Preserved | No |
| `:height` | Automatic width, exact height | Preserved | No |
| `:contain` | Fits inside the requested box | Preserved | No |
| `:cover` | Exact requested box | Preserved | Center crop |
| `:stretch` | Exact requested box | May change | No |

Calling `create(path, size, mode)` remains backward compatible and is equivalent to
`create(path, size, size, mode, fit: :cover)`.
When `upscale: false` is used, the output dimensions can be smaller than the
requested size so that source pixels are never enlarged.

## Example Liveview

Below is a **LiveView**:

```elixir
defmodule MyAppWeb.UploadLive do
  use MyAppWeb, :live_view

  @impl true
  def mount(_params, _session, socket) do
    socket =
      socket
      # Limit to .jpg, .jpeg for this example
      |> allow_upload(:avatar, accept: ~w(.jpg .jpeg), max_entries: 1)

    {:ok, socket}
  end

  @impl true
  def handle_event("validate", _params, socket) do
    {:noreply, socket}
  end

  def handle_event("cancel-upload", %{"ref" => ref}, socket) do
    {:noreply, cancel_upload(socket, :avatar, ref)}
  end

  def handle_event("save", _params, socket) do
    uploaded_files =
      consume_uploaded_entries(socket, :avatar, fn %{path: tmp_path}, entry ->
        case upload_to_s3(tmp_path, entry.client_name) do
          {:ok, result} -> {:ok, result}
          {:error, reason} -> {:postpone, reason}
        end
      end)

    {:noreply, update(socket, :uploaded_files, &(&1 ++ uploaded_files))}
  end

  defp upload_to_s3(file_path, name_for_s3) do
    with {:ok, "data:image/webp;base64," <> encoded} <-
           FastThumbnail.create(file_path, 200, :base64),
         {:ok, thumbnail} <- Base.decode64(encoded) do
      access_key = System.fetch_env!("AWS_ACCESS_KEY")
      secret_key = System.fetch_env!("AWS_SECRET_KEY")
      region = System.fetch_env!("AWS_REGION")
      bucket = System.get_env("AWS_BUCKET_NAME", "my-bucket")
      client = AWS.Client.create(access_key, secret_key, region)
      key = "thumbnails/#{name_for_s3}"

      put_params = %{
        "Body" => thumbnail,
        "ContentType" => "image/webp",
        "Metadata" => %{"OriginalName" => name_for_s3}
      }

      AWS.S3.put_object(client, bucket, key, put_params)
    end
  end
end
```

With this, you have a **LiveView** flow that:

1. Accepts user uploads,  
2. Creates a **resized** webp thumbnail,  
3. Uploads it to S3,  
4. Returns the final S3 URL or a local URL.

Under the hood, the Elixir function calls a Rust NIF which performs the requested resize, including crop when needed, using [fast_image_resize](https://crates.io/crates/fast_image_resize). The resized bytes are either:

- Written back to disk in the same or different format, **or**
- Returned to Elixir as a base64 string.

## Development

Run the full test suite with the locally compiled Rust NIF:

```bash
task test
```

The test task and CI both enforce a minimum line coverage of 90%.

## Credits

- **[fast_image_resize](https://github.com/Cykooz/fast_image_resize)** – the Rust crate that powers the SIMD-accelerated resizing. See also:
  - [Crates.io](https://crates.io/crates/fast_image_resize)
  - [Docs.rs](https://docs.rs/fast_image_resize)
- **[Thumbp](https://github.com/ryochin/thumbp)** - Another excellent Elixir library that provides a fast and efficient way to generate thumbnails from images.
- **[rustler](https://github.com/rusterlium/rustler)** – used for building the native Rust code as an Elixir NIF.

## License

FastThumbnail is licensed under the [Apache License, Version 2.0](LICENSE).

*fast_image_resize* is distributed under its own license. Please refer to its repository for details.
