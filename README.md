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
chillphoto init --image-ai # AI-generate photo descriptions based on thumbnails
```

### Directory Structure

```sh
/gallery
  chillphoto.toml           # top-level config
  favicon.png               # favicon
  head.html                 # HTML to include in <head>
  home.{txt,md,html}        # gallery homepage caption
  /static                   # custom static files
    button.svg              # decorative image for some page
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

### Top-level config

```
input = "src/**/*.{JPG,jpg,txt,png,md,html,toml}"
output = "./build"
title = "Photos!"
author = "Full Name"
author_url = "https://fullname.me"
root_url = "https://example.com"
license_url = "https://creativecommons.org/licenses/by-sa/4.0/deed.en"
acquire_license_url = "https://example.com/Copyright/"
description = "My favorite photos"
categories = ["photo"]
disallow_ai_training = false
photo_resolution = 3840
photo_format = "jpg"
preview_resolution = 1920
preview_format = "jpg"
thumbnail_resolution = 100
thumbnail_format = "jpg"
image_ai_api_base_url = "optional OpenAI-style API instead of ollama; defaults to OpenAI's API"
image_ai_api_key = "optional API key for image_ai_api_base_url"
image_ai_model = "gemma3"
# ai_description_system_prompt = "override system prompt"
ai_description_hint = "do not mention text in photos"
items_per_page = 30
date_format = "..." # see https://docs.rs/chrono/latest/chrono/format/strftime/index.html
```

### Category config

All fields are optional.
```toml
# displayed with gallery thumbnail, used in metadata
description = "..."
# override URL slug
slug = "..."
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
# photo in the category
thumbnail = "Photo1"
# correct AI hallucinations without needing to manually overwrite everything.
ai_description_hint = "all photos have dirt not sand"
# path or query string page number
pagination_flavor = "path"
# categories/photos per page
items_per_page = 30
```

### Photo config

All fields are optional.
```toml
# alt text
description = "..."
# override URL slug
slug = "..."
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
# specify or overeride the photo's date, using the gallery's date format
date = "..."
```

### Page config

All fields are optional.
```toml
# used in metadata
description = "..."
# override URL slug
slug = "..."
# higher -> first
# -2, -1, 0, 1, 2, etc.
order = 0
# to avoid it appearing in the sidebar
unlisted = true
```

## Features
- [x] Instantly preview gallery via embedded server
- [x] Generate a completely static gallery website
- [x] Full, preview, and thumbnail sizes
- [x] Arbitrarily-nested categories for photos and pages
- [x] Arbitrary plain-text, Markdown, or HTML pages and captions
- [x] Input essential EXIF metadata
- [x] Output HTML, Sitemap, PWA, structured data, XMP, and Open Graph metadata
- [x] AI photo descriptions
- [x] Basic image adjustment (exposure)
- [x] Pagination
- [x] Generate US Copyright Office group registration ZIP files
- [ ] 404 page
- [ ] Hot-reloading
- [ ] Diagnostics and error handling
- [ ] Support for themes
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