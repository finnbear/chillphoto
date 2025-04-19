# chillphoto
Static photo gallery website generator

## Installation
```sh
cargo install --git https://github.com/finnbear/chillphoto
```

## Usage

### Sub-commands
```sh
chillphoto init     # initialize top-level config
chillphoto serve    # preview the gallery
chillphoto build    # generate the gallery

ollama pull gemma3  # install dependency
chillphoto image-ai # AI-generate photo descriptions based on thumbnails
```

### Directory Structure

```sh
/gallery
  chillphoto.toml           # top-level config
  favicon.png               # favicon
  head.html                 # HTML to include in <head>
  home.{txt,md,html}        # gallery homepage caption
  About.{txt,md,html}       # page, linked on sidebar
  About.toml                # page config
  Category 2.{txt,md,html}  # category caption
  /Category 1               # category
    Photo1.{jpg,png}        # photo (w/ EXIF)
    Photo1.toml             # photo config
    Photo1.{txt,md,html}    # photo caption
  Category 1.toml           # category config
  /Category 2               # category
    Photo3.{JPG,PNG}        # photo (w/ EXIF)
```

### Category config

All fields are optional.
```toml
# displayed with gallery thumbnail, used in metadata
description = "..."
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
# photo in the category
thumbnail = "Photo1"
# correct AI hallucinations without needing to manually overwrite everything.
ai_description_hint = "all photos have dirt not sand"
```

### Photo config

All fields are optional.
```toml
# alt text
description = "..."
# to display in details, etc.
location = "..."
# override
author = "Full Name"
# override
license_url = "https://creativecommons.org/licenses/by-sa/4.0/deed.en"
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
# correct AI hallucinations without needing to manually overwrite everything.
ai_description_hint = "it's dirt not sand"
# stops of exposure to digitally add (or subtract).
exposure = 0.33
```

### Page config

All fields are optional.
```toml
# used in metadata
description = "..."
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
```

## Features
- [x] Instantly preview gallery via embedded server
- [x] Generate a completely static gallery website
- [x] Full, preview, and thumbnail sizes
- [x] Arbitrarily-nested photo categories
- [x] Arbitrary plain-text, Markdown, or HTML pages and captions
- [x] Input essential EXIF metadata
- [x] Output HTML, Sitemap, PWA, structured data, and Open Graph metadata
- [x] AI photo descriptions
- [x] Basic image adjustment (exposure)
- [ ] Pagination
- [ ] Hot-reloading
- [ ] Diagnostics and error handling
- [ ] Support for themes
- [ ] Pages within categories
- [ ] Archive page organized by date
- [ ] Optional comment support (via a 3rd party comment form)
- [ ] RSS feed
- [ ] Optional visual editor

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgements

The current default, and only, theme is heavily based on [ZenPage](https://github.com/zenphoto/zenphoto/tree/master/themes/zenpage) by [Malte Müller](https://maltem.de/).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.