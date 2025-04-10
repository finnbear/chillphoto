use std::io::Cursor;

use base64::Engine;
use image::{ImageFormat, RgbImage};
use ollama_rs::{
    generation::{
        completion::{request::GenerationRequest, GenerationResponse},
        images::Image,
        parameters::{KeepAlive, TimeUnit},
    },
    Ollama,
};
use tokio::runtime::Builder;

pub fn ai_summarize(category_name: &str, image: &RgbImage) -> String {
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

        let request = GenerationRequest::new(
            "llava:latest".to_string(),
            format!("The category is {category_name}. Describe the photo in 3-5 sentences."),
        )
        .add_image(image)
        .keep_alive(KeepAlive::Until { time: 5, unit: TimeUnit::Seconds })
        .system("You are a photo summarizer tasked with generating alt text for photos on an gallery website.");

        let response = match send_request(request).await {
            Ok(r) => r,
            Err(e) => {
                panic!("Failed to get response: {}", e);
            }
        };

        response.response
    })
}

async fn send_request(
    request: GenerationRequest<'_>,
) -> Result<GenerationResponse, Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    let response = ollama.generate(request).await?;
    Ok(response)
}
