# FastThumbnail

FastThumbnail is an Elixir library that uses a [Rust NIF](https://hexdocs.pm/rustler) under the hood to perform **fast, SIMD-optimized** image resizing. It leverages the excellent [fast_image_resize](https://github.com/Cykooz/fast_image_resize) crate to crop and resize images efficiently.

### Why?

- **Fast**: SIMD-accelerated resizing is blazingly fast.

I just wanted stupidly simple image resizing that do just one thing: crop and resize an image to a given width.
There is already good libraries for this [image](https://github.com/elixir-image/image), but they all seem to be too complex for my needs. Also I like small dependencies.

## Features

- **Center-crop** images to a square, then resize (all in a single pass).
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
    {:fast_thumbnail, "~> 0.1.5"}
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
# 1) Overwrite file in its original format (JPEG→JPEG, PNG→PNG, etc.)
iex> FastThumbnail.create("images/photo.jpg", 300, :overwrite)
{:ok, "images/photo.jpg"}

# 2) Create a new file in WebP format:
iex> FastThumbnail.create("images/photo.jpg", 300, :webp)
{:ok, "images/photo.jpg.webp"}

# 3) Return a base64-encoded WebP (no file writing):
iex> FastThumbnail.create("images/photo.jpg", 300, :base64)
{:ok, "data:image/webp;base64,UklGRrwAAABXRUJQVlA4T..."}

```

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
      consume_uploaded_entries(socket, entry, fn %{path: tmp_path} ->
          upload_to_s3(tmp_path, entry.client_name)

          # handle errors here
      end)

    {:noreply, update(socket, :uploaded_files, &(&1 ++ uploaded_files))}
  end

  defp upload_to_s3(file_path, name_for_s3) do
    with {:ok, thumb_base64} <- FastThumbnail.create(file_path, 200, :base64) do

    access_key = System.get_env("AWS_ACCESS_KEY")
    secret_key = System.get_env("AWS_SECRET_KEY")
    region     = System.get_env("AWS_REGION")
    bucket     = System.get_env("AWS_BUCKET_NAME") || "my-bucket"
    client     = AWS.Client.create(access_key, secret_key, region) 

    folder = "thumbnails"
    key = "#{folder}/#{name_for_s3}"

    # S3 `put_object` params
    put_params = %{
      "Body" => thumb_base64,
      "ContentType" => "image/webp",
      "ContentEncoding" => "base64",
      "Metadata" => %{"OriginalName" => name_for_s3}
    }

    AWS.S3.put_object(client, bucket, key, put_params)
  end
end
```

With this, you have a **LiveView** flow that:

1. Accepts user uploads,  
2. Creates a **resized** webp thumbnail,  
3. Uploads it to S3,  
4. Returns the final S3 URL or a local URL.

Under the hood, the Elixir function calls a Rust NIF which performs a **single-pass** crop+resize operation using [fast_image_resize](https://crates.io/crates/fast_image_resize). The resized bytes are either:

- Written back to disk in the same or different format, **or**
- Returned to Elixir as a base64 string.

## Credits

- **[fast_image_resize](https://github.com/Cykooz/fast_image_resize)** – the Rust crate that powers the SIMD-accelerated resizing. See also:
  - [Crates.io](https://crates.io/crates/fast_image_resize)
  - [Docs.rs](https://docs.rs/fast_image_resize)
- **[Thumbp](https://github.com/ryochin/thumbp)** - Another excellent Elixir library that provides a fast and efficient way to generate thumbnails from images.
- **[rustler](https://github.com/rusterlium/rustler)** – used for building the native Rust code as an Elixir NIF.

## License

FastThumbnail is licensed under the [Apache License, Version 2.0](LICENSE).

*fast_image_resize* is distributed under its own license. Please refer to its repository for details.
