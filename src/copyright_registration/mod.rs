use crate::gallery::{Item, Photo};
use crate::output::write_image;
use crate::{gallery::Gallery, output::OutputFormat};
use chrono::Datelike;
use chrono::{NaiveDate, NaiveDateTime};
use genpdfi::fonts::{FontData, FontFamily};
use genpdfi::style::{Style, StyledString};
use genpdfi::{elements, Margins, SimplePageDecorator};
use std::fs;
use std::io::{Cursor, Write};
use zip::write::{SimpleFileOptions, ZipWriter};

mod genpdfi_cell_decorator;

pub use genpdfi_cell_decorator::*;

const ARCHIVE_SIZE_LIMIT: usize = 500 * 1000 * 1000;

impl Gallery {
    pub fn copyright(
        &mut self,
        year: u32,
        author: &str,
        case_number: &str,
        start: Option<&str>,
        limit: usize,
        resolution: u32,
    ) {
        // Copyright office doesn't support WebP.
        if self.config.photo_format == OutputFormat::WebP {
            self.config.photo_format = OutputFormat::Png;
        }

        struct PhotoCopyrightSubmission<'a> {
            filename: String,
            title: String,
            date_time: NaiveDateTime,
            photo: &'a Photo,
        }

        let mut archive = ZipWriter::new(Cursor::new(Vec::<u8>::new()));
        let mut manifest = Vec::<PhotoCopyrightSubmission>::new();
        let zip_options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let mut earliest_date = Option::<NaiveDate>::None;
        let mut latest_date = Option::<NaiveDate>::None;

        self.visit_items(|_, item| {
            let photo = if let Item::Photo(photo) = item {
                photo
            } else {
                return;
            };

            let date_time = if let Some(date) = photo.date_time() {
                date
            } else {
                return;
            };

            let date = date_time.date();
            if date.year_ce().1 != year
                || photo
                    .config
                    .author
                    .as_ref()
                    .or(self.config.author.as_ref())
                    .map(|a| a != author)
                    .unwrap_or(true)
            {
                return;
            }

            let filename = format!("{}.{}", photo.slug(), self.config.photo_format.extension());
            manifest.push(PhotoCopyrightSubmission {
                title: photo.output_name().to_owned(),
                filename,
                date_time,
                photo,
            });
        });

        manifest.sort_by_key(|m| (m.date_time, m.filename.clone()));

        if let Some(start) = start {
            let index = manifest
                .iter()
                .position(|submission| submission.title == *start || submission.filename == *start)
                .unwrap();
            manifest.drain(0..index);
        }
        if manifest.len() > limit {
            println!("WARNING: truncated to {limit} photos; next is \"{}\"", manifest[limit].title);
            manifest.truncate(limit);
        }

        let mut written = 0usize;

        for (i, submission) in manifest.iter().enumerate() {
            archive
                .start_file(&submission.filename, zip_options)
                .unwrap();
            let image_bytes = write_image(
                &submission.photo.custom_preview(&self.config, resolution),
                &submission.filename,
                Some((&self.config, submission.photo)),
            );
            archive.write_all(&image_bytes).unwrap();
            written += image_bytes.len();
            if written >= ARCHIVE_SIZE_LIMIT - 100000 {
                panic!("exceeded archive size limit after {} photos", i + 1);
            }
            let date = submission.date_time.date();
            if earliest_date.map(|d| date < d).unwrap_or(true) {
                earliest_date = Some(date);
            }
            if latest_date.map(|d| date > d).unwrap_or(true) {
                latest_date = Some(date);
            }
        }

        let group_name = format!(
            "{} {} photos by {author} in {year}{}",
            manifest.len(),
            self.config.title,
            if let Some(start) = start {
                format!(" starting with {start}")
            } else {
                String::new()
            }
        );
        let manifest_filename = format!("{group_name} Case Number {case_number}.pdf",);
        archive.start_file(&manifest_filename, zip_options).unwrap();

        let mut doc = genpdfi::Document::new(FontFamily {
            regular: FontData::new(
                include_bytes!("./fonts/LiberationMono-Regular.ttf").to_vec(),
                None,
            )
            .unwrap(),
            italic: FontData::new(
                include_bytes!("./fonts/LiberationMono-Italic.ttf").to_vec(),
                None,
            )
            .unwrap(),
            bold: FontData::new(
                include_bytes!("./fonts/LiberationMono-Bold.ttf").to_vec(),
                None,
            )
            .unwrap(),
            bold_italic: FontData::new(
                include_bytes!("./fonts/LiberationMono-BoldItalic.ttf").to_vec(),
                None,
            )
            .unwrap(),
        });

        let mut page_decorator = SimplePageDecorator::new();
        page_decorator.set_margins(Margins::all(5.0));
        doc.set_page_decorator(page_decorator);
        doc.set_title(manifest_filename);
        doc.set_font_size(8);
        doc.push(elements::Paragraph::new(StyledString::new(
            "Group Registration of Published Photographs",
            Style::new().bold().with_font_size(16),
            None,
        )));
        doc.push(elements::Break::new(1.0));
        doc.push(elements::Paragraph::new(StyledString::new(
            format!("This is the Complete List of Photographs for: {case_number}"),
            Style::new().bold(),
            None,
        )));
        doc.push(elements::Paragraph::new(StyledString::new(
            format!("The group is: {group_name}"),
            Style::new(),
            None,
        )));
        doc.push(elements::Paragraph::new(StyledString::new(
            format!("The author is: {author}"),
            Style::new(),
            None,
        )));
        doc.push(elements::Paragraph::new(StyledString::new(
            format!("The year of completion is: {year}"),
            Style::new(),
            None,
        )));
        if let Some((earliest_date, latest_date)) = earliest_date.zip(latest_date) {
            doc.push(elements::Paragraph::new(StyledString::new(
                format!(
                    "The earliest publication date is: {}",
                    earliest_date.format("%m/%d/%Y")
                ),
                Style::new(),
                None,
            )));
            doc.push(elements::Paragraph::new(StyledString::new(
                format!(
                    "The latest publication date is: {}",
                    latest_date.format("%m/%d/%Y")
                ),
                Style::new(),
                None,
            )));
        }
        doc.push(elements::Paragraph::new(StyledString::new(
            format!("Number of photos: {}", manifest.len()),
            Style::new(),
            None,
        )));

        doc.push(elements::Break::new(1.0));

        let mut table = elements::TableLayout::new(vec![1, 3, 3, 1]);

        table.set_cell_decorator(MarginFrameCellDecorator::new(true, true, false));
        table
            .push_row(vec![
                Box::new(elements::Text::new(StyledString::new(
                    "Photo No.",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(elements::Text::new(StyledString::new(
                    "Title of Photo",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(elements::Text::new(StyledString::new(
                    "Filename of Photo",
                    Style::new().bold(),
                    None,
                ))),
                Box::new(elements::Text::new(StyledString::new(
                    "Pub. Date",
                    Style::new().bold(),
                    None,
                ))),
            ])
            .unwrap();

        for (i, submission) in manifest.iter().enumerate() {
            table
                .push_row(vec![
                    Box::new(elements::Text::new((i + 1).to_string())),
                    Box::new(elements::Text::new(&submission.title)),
                    Box::new(elements::Text::new(&submission.filename)),
                    Box::new(elements::Text::new(format!(
                        "{}/{}",
                        submission.date_time.date().month(),
                        submission.date_time.date().year_ce().1
                    ))),
                ])
                .unwrap();
        }

        doc.push(table);

        doc.push(elements::Break::new(1.0));

        doc.push(elements::Paragraph::new(StyledString::new(
            format!(
                "This PDF was generated by chillphoto on {}.",
                chrono::Utc::now()
            ),
            Style::new(),
            None,
        )));

        doc.render(&mut archive).unwrap();

        let archive_contents = archive.finish().unwrap().into_inner();
        assert!(archive_contents.len() <= ARCHIVE_SIZE_LIMIT);
        fs::write(format!("{author}-{year}.zip"), archive_contents).unwrap();
    }
}
