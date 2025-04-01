# chillphoto
Static photo gallery website generator

## Installation
```sh
cargo install --git https://github.com/finnbear/chillphoto
```

## Usage

### Sub-commands
```sh
chillphoto init   # initialize top-level config
chillphoto serve  # preview the gallery
chillphoto build  # generate the gallery
```

### Directory Structure

```sh
/gallery
  chillphoto.toml  # top-level config
  favicon.png      # favicon
  About.txt        # plain-text file
  Copyright.md     # Markdown page
  Equipment.html   # HTML page
  Category 1.toml  # category config
  /Category 1      # category
    Photo1.jpg     # photo (w/ EXIF)
    Photo1.toml    # photo config
    Photo1.txt     # photo caption
    Photo2.png     # photo (w/ EXIF)
    Photo2.md      # Markdown caption
  /Category 2      # category
    Photo3.JPG     # photo (w/ EXIF)
    Photo3.html    # HTML caption
```

### Category config

```toml
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
thumbnail = "Photo1"
```

### Photo config

```toml
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
# higher -> zoomed in more
# 1.0+
thumbnail_crop_factor = 1.0
# center of crop square
thumbnail_crop_center = {
    # 0.0 - 1.0
    x = 0.5,
    # 0.0 - 1.0
    y = 0.5
}
```

## Features
- [x] Instantly preview gallery via embedded server
- [x] Generate a completely static gallery website site
- [x] Full, preview, and thumbnail sizes
- [x] Arbitrarily-nested photo categories
- [x] Arbitrary plain-text, Markdown, or HTML pages and captions
- [x] Sitemap
- [ ] Pagination
- [ ] Hot-reloading
- [ ] Diagnostics and error handling
- [ ] Support for themes
- [ ] Progress bars
- [ ] Pages within categories
- [ ] Archive page organized by date
- [ ] Category descriptions and captions
- [ ] Optional comment support (via a 3rd party comment form)
- [ ] AI summarization of images
- [ ] Show subset of EXIF metadata
- [ ] RSS feed
- [ ] Optional visual editor

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.