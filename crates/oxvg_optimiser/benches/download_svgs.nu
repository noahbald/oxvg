def main [output_dir?: path] {
  let destination = if $output_dir == null {
    $env.FILE_PWD
  } else {
    $output_dir
  }

  mkdir $destination

  let files = [
    {
      checksum: "206f411823974c3e51b4164e0a70bf0899e352da89728cfe8228e532c72b9c46"
      url: "https://gitlab.gnome.org/GNOME/gnome-backgrounds/-/raw/main/backgrounds/blobs-d.svg"
    }
    {
      checksum: "a7f37c9c87f0e0e37454b47ec1221b61416aa9e319cc46a1f0b5b70c4a1dcb6e"
      url: "https://archlinux.org/static/logos/archlinux-logo-dark-scalable.518881f04ca9.svg"
    }
    {
      checksum: "af16b78f83dcf9a92db1dfc6c4caaa875dbfe2e6f1c62a6c13ec0f81132e3722"
      url: "https://inkscape.org/gallery/item/39652/Inkscape_About_Screen_Isometric_madness_HdG4la4.svg"
    }
    {
      checksum: "cd056fd2d3418c4b7a8d13405b1b1c55c5ae6203fa611c1b9abc85b3b6f522ee"
      url: "https://github.com/tldr-pages/tldr/raw/refs/heads/main/images/banner.svg"
    }
    {
      checksum: "90e01ef7af53a6b624fa24ee3a48c7d6dd9f55b0f999bc8c7303f8d289ecd798"
      url: "https://upload.wikimedia.org/wikipedia/en/8/80/Wikipedia-logo-v2.svg"
    }
    {
      checksum: "46114360883b9da754174546b49103c7c1a2214ea0f90c26e4d4bfb7ae5addb9"
      url: "https://upload.wikimedia.org/wikipedia/commons/6/6c/Trajans-Column-lower-animated.svg"
    }
  ]

  $files | par-each { |entry|
    let filename = $entry.url | split row "/" | last
    let file = $destination | path join $filename

    http get $entry.url | save $file -f

    let actual = open --raw $file | hash sha256
    if $actual != $entry.checksum {
      rm $file
      error make {
        msg: $"checksum mismatch for ($filename): expected ($entry.checksum), got ($actual)"
      }
    }
  }
}
