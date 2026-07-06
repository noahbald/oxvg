#!/usr/bin/env sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir=${1:-"$script_dir"}

mkdir -p "$output_dir"

sha256()
{
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  else
    shasum -a 256 "$1" | awk '{ print $1 }'
  fi
}

download()
{
  expected=$1
  url=$2
  filename=${url##*/}
  destination="$output_dir/$filename"
  temporary="$destination.part"

  curl \
    --fail \
    --location \
    --retry 3 \
    --show-error \
    --silent \
    --output "$temporary" \
    "$url"

  actual=$(sha256 "$temporary")
  if [ "$actual" != "$expected" ]; then
    rm -f "$temporary"
    echo "checksum mismatch for $filename: expected $expected, got $actual" >&2
    exit 1
  fi

  mv "$temporary" "$destination"
}

download \
  206f411823974c3e51b4164e0a70bf0899e352da89728cfe8228e532c72b9c46 \
  https://gitlab.gnome.org/GNOME/gnome-backgrounds/-/raw/main/backgrounds/blobs-d.svg
download \
  a7f37c9c87f0e0e37454b47ec1221b61416aa9e319cc46a1f0b5b70c4a1dcb6e \
  https://archlinux.org/static/logos/archlinux-logo-dark-scalable.518881f04ca9.svg
download \
  af16b78f83dcf9a92db1dfc6c4caaa875dbfe2e6f1c62a6c13ec0f81132e3722 \
  https://inkscape.org/gallery/item/39652/Inkscape_About_Screen_Isometric_madness_HdG4la4.svg
download \
  cd056fd2d3418c4b7a8d13405b1b1c55c5ae6203fa611c1b9abc85b3b6f522ee \
  https://github.com/tldr-pages/tldr/raw/refs/heads/main/images/banner.svg
download \
  90e01ef7af53a6b624fa24ee3a48c7d6dd9f55b0f999bc8c7303f8d289ecd798 \
  https://upload.wikimedia.org/wikipedia/en/8/80/Wikipedia-logo-v2.svg
download \
  46114360883b9da754174546b49103c7c1a2214ea0f90c26e4d4bfb7ae5addb9 \
  https://upload.wikimedia.org/wikipedia/commons/6/6c/Trajans-Column-lower-animated.svg
