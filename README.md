A simple PNG compression utility.

## How it works

  1. Re-compress PNG image with `zopflipng`
  2. Convert PNG image to WebP with `cwebp`
  3. Remove WebP image if WebP image is larger than PNG

## How to use

### Build
```sh
cargo build --release
```

### Run
```sh
compress_png -h
compress_png a.png
compress_png --no-webp a.png
compress_png --remove-larger-png a.png
```
