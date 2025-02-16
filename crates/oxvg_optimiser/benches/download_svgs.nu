def main [] {
  # Files are picked from https://svgo.dev/docs/plugins/sortAttrs/ docs
  let files = [
    "https://gitlab.gnome.org/GNOME/gnome-backgrounds/-/raw/main/backgrounds/blobs-d.svg",
    "https://archlinux.org/static/logos/archlinux-logo-dark-scalable.518881f04ca9.svg",
    "https://inkscape.org/gallery/item/39652/Inkscape_About_Screen_Isometric_madness_HdG4la4.svg",
    "https://github.com/tldr-pages/tldr/raw/refs/heads/main/images/banner.svg",
    "https://upload.wikimedia.org/wikipedia/en/8/80/Wikipedia-logo-v2.svg",
    "https://upload.wikimedia.org/wikipedia/commons/6/6c/Trajans-Column-lower-animated.svg",
  ]
  $files | par-each { |e|
    http get $e |
    save $"($env.FILE_PWD)/($e | split row "/" | last)" -f
  }
}
