use crate::{config::GalleryConfig, photo::Photo, util::checksum};
use base64::Engine;
use image::ImageFormat;
use ollama_rs::{
    generation::{
        completion::{request::GenerationRequest, GenerationResponse},
        images::Image,
        parameters::{KeepAlive, TimeUnit},
    },
    Ollama,
};
use std::io::Cursor;
use tokio::runtime::Builder;

pub struct ImageAiPrompt<'a> {
    pub prompt: &'a str,
    pub photo: &'a Photo,
    pub config: &'a GalleryConfig,
}

impl<'a> ImageAiPrompt<'a> {
    pub fn checksum(&self) -> String {
        let mut to_hash = self.photo.input_image_data.clone();
        to_hash.extend_from_slice(self.config.image_ai_model.as_bytes());
        to_hash.extend_from_slice(self.prompt.as_bytes());
        to_hash.extend_from_slice(self.config.ai_description_system_prompt.as_bytes());
        to_hash.extend_from_slice(&self.photo.config.thumbnail_crop_factor.to_le_bytes());
        to_hash.extend_from_slice(&self.photo.config.thumbnail_crop_center.x.to_le_bytes());
        to_hash.extend_from_slice(&self.photo.config.thumbnail_crop_center.y.to_le_bytes());
        checksum(&to_hash)
    }
}

pub fn image_ai(prompt: ImageAiPrompt) -> String {
    let image = prompt.photo.thumbnail(&prompt.config);
    let jpeg = Vec::<u8>::new();
    let mut cursor = Cursor::new(jpeg);
    image.write_to(&mut cursor, ImageFormat::Jpeg).unwrap();
    let base64_image = base64::engine::general_purpose::STANDARD.encode(&cursor.into_inner());

    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let image = Image::from_base64(&base64_image);

        let request = GenerationRequest::new(prompt.config.image_ai_model.clone(), prompt.prompt)
            .add_image(image)
            .keep_alive(KeepAlive::Until {
                time: 5,
                unit: TimeUnit::Seconds,
            })
            .system(&prompt.config.ai_description_system_prompt);

        let response = match send_request(request).await {
            Ok(r) => r,
            Err(e) => {
                panic!("Failed to get response: {}", e);
            }
        };

        response.response.trim().to_owned()
    })
}

async fn send_request(
    request: GenerationRequest<'_>,
) -> Result<GenerationResponse, Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    let response = ollama.generate(request).await?;
    Ok(response)
}
