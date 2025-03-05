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
    {:fast_thumbnail, "~> 0.1.0", github: "elchemista/fast_thumbnail"}
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

Below is a **LiveView** example that builds on your provided upload flow and demonstrates:

```elixir
defmodule MyAppWeb.UploadLive do
  use MyAppWeb, :live_view

  @impl Phoenix.LiveView
  def mount(_params, _session, socket) do
    socket =
      socket
      # Limit to .jpg, .jpeg for this example
      |> allow_upload(:avatar, accept: ~w(.jpg .jpeg), max_entries: 2)

    {:ok, socket}
  end

  @impl Phoenix.LiveView
  def handle_event("validate", _params, socket) do
    {:noreply, socket}
  end

  @impl Phoenix.LiveView
  def handle_event("cancel-upload", %{"ref" => ref}, socket) do
    {:noreply, cancel_upload(socket, :avatar, ref)}
  end

  @impl Phoenix.LiveView
  def handle_event("save", _params, socket) do
    uploaded_files =
      consume_uploaded_entries(socket, entry, fn %{path: tmp_path} ->
        save_result = save_and_process_upload(entry.client_name, tmp_path)
      end)

    # Make all check on uploaded_files {:ok, file} or {:ok, {:error, reason}}

    {:noreply, update(socket, :uploaded_files, &(&1 ++ uploaded_files))}
  end

  defp save_and_process_upload(name, tmp_path) do
    thumb_result = FastThumbnail.create(tmp_path, 200, :webp)

    case thumb_result do
      {:ok, thumb_path} ->

        s3_result = upload_to_s3(thumb_path, name <> ".webp")

        case s3_result do
          {:ok, s3_url} ->
              {:ok, s3_url}

          {:error, reason} ->
            {:ok, {:error, reason}}
            # otherwise you can postone
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp upload_to_s3(file_path, name_for_s3) do
    # Build your AWS client
    access_key = System.get_env("AWS_ACCESS_KEY")
    secret_key = System.get_env("AWS_SECRET_KEY")
    region     = System.get_env("AWS_REGION")
    bucket     = System.get_env("AWS_BUCKET_NAME") || "my-bucket"

    client     = AWS.Client.create(access_key, secret_key, region)

    # Read the local WebP thumbnail bytes
    file_body = File.read!(file_path)

    # We'll store it in S3 at some key (like "thumbnails/<filename>.webp")
    # This is just an example – adjust folder/paths as needed
    folder = "thumbnails"
    key = "#{folder}/#{name_for_s3}"

    s3_url = "https://s3.#{region}.amazonaws.com/#{bucket}/#{key}"

    # S3 `put_object` params
    put_params = %{
      "Body" => file_body,
      "ContentType" => "image/webp",
      "Metadata" => %{"OriginalName" => name_for_s3}
    }

    case AWS.S3.put_object(client, bucket, key, put_params) do
      {:ok, _, _} ->
        {:ok, s3_url}

      {:error, reason} ->
        {:error, reason}
    end
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
- **[rustler](https://github.com/rusterlium/rustler)** – used for building the native Rust code as an Elixir NIF.

## License

FastThumbnail is licensed under the [MIT License](LICENSE).  

*fast_image_resize* is distributed under its own license. Please refer to its repository for details.