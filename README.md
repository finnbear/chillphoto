# chillphoto
Static photo gallery website generator

## Installation
```sh
cargo install --git https://github.com/finnbear/chillphoto
```

## Usage
```sh
chillphoto init   # initialize top-level config
chillphoto build  # generate the gallery
chillphoto serve  # preview the gallery
```

## Current Features
- Instantly preview gallery via embedded server
- Generate a completely static gallery website site
- Full, preview, and thumbnail sizes
- Arbitrarily-nested photo categories
- Arbitrary plain-text, Markdown, or HTML pages and captions

## Planned Features
- Pagination
- Hot-reloading
- Diagnostics and error handling
- Support for themes
- Pages within categories
- Archive page organized by date
- Category descriptions and captions
- Optional comment support (via a 3rd party comment form)
- AI summarization of images
- Show subset of EXIF metadata
- RSS feed
- Optional visual editor

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