use crate::{
    gallery::CategoryPath, gallery::Gallery, gallery::GalleryConfig, gallery::Photo, util::checksum,
};
use async_openai::types::{
    ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestSystemMessageContent,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
    ChatCompletionRequestUserMessageContentPart, CreateChatCompletionRequestArgs, ImageDetail,
    ImageUrl,
};
use base64::Engine;
use image::ImageFormat;
use ollama_rs::{
    generation::{
        completion::{request::GenerationRequest, GenerationResponse},
        images::Image,
        parameters::{KeepAlive, TimeUnit},
    },
    models::ModelOptions,
    Ollama,
};
use std::fmt::Write as _;
use std::io::Cursor;
use tokio::runtime::Builder;
use toml_edit::DocumentMut;

pub fn init_image_ai(gallery: &Gallery, path: &CategoryPath, photo: &Photo, doc: &mut DocumentMut) {
    let mut prompt = String::new();
    if path.is_root() {
        writeln!(
            prompt,
            "The photo is in the gallery: {}.",
            gallery.config.title
        )
        .unwrap();
        if let Some(description) = &gallery.config.description {
            writeln!(prompt, "The gallery description is: {description}.",).unwrap();
        }
    } else {
        let category = gallery.category(path).unwrap();
        writeln!(prompt, "The photo is in the category: {}.", category.name).unwrap();
        if let Some(description) = &category.config.description {
            writeln!(prompt, "The category description is: {description}.",).unwrap();
        }
        if let Some(hint) = &category.config.ai_description_hint {
            writeln!(
                prompt,
                "A hint for the entire category been provided: {hint}."
            )
            .unwrap();
        }
    }
    if let Some(hint) = &gallery.config.ai_description_hint {
        writeln!(
            prompt,
            "A hint for the entire gallery been provided: {hint}."
        )
        .unwrap();
    }
    if let Some(hint) = &photo.config.ai_description_hint {
        writeln!(prompt, "A hint has been provided: {hint}.").unwrap();
    }

    writeln!(prompt, "Describe the photo.").unwrap();

    let prompt = ImageAiPrompt {
        prompt: &prompt,
        photo,
        config: &gallery.config,
    };

    if let Some(description) = photo.config.description.as_ref() {
        if photo.config.ai_description_output_checksum != Some(checksum(description.as_bytes())) {
            println!("keeping manual description for {}", photo.name);
            return;
        }
    }

    let input_checksum = prompt.checksum();
    if let Some(sum) = &photo.config.ai_description_input_checksum {
        if input_checksum == *sum {
            println!("keeping existing ai description for {}", photo.name);
            return;
        } else {
            println!("regenerating ai description for {}", photo.name);
        }
    }

    let summary = image_ai(
        prompt,
        &gallery.config.image_ai_api_base_url,
        gallery.config.image_ai_api_key.as_deref(),
    );

    doc["description"] = toml_edit::value(summary.clone());
    doc["ai_description_input_checksum"] = toml_edit::value(input_checksum);
    doc["ai_description_output_checksum"] = toml_edit::value(checksum(&summary.as_bytes()));

    println!("summarized {}: {summary}", photo.name);
}

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

pub fn image_ai(prompt: ImageAiPrompt, base_url: &str, api_key: Option<&str>) -> String {
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

    const TEMPERATURE: f32 = 0.25;

    rt.block_on(async {
        if let Some(api_key) = api_key {
            let client = async_openai::Client::with_config(
                async_openai::config::OpenAIConfig::new()
                    .with_api_base(base_url)
                    .with_api_key(api_key),
            );
            client
                .chat()
                .create(
                    CreateChatCompletionRequestArgs::default()
                        .model(&prompt.config.image_ai_model)
                        .temperature(TEMPERATURE)
                        .messages([
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(ChatCompletionRequestSystemMessageContent::Text(
                                    prompt.config.ai_description_system_prompt.clone(),
                                ))
                                .build()
                                .unwrap()
                                .into(),
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(ChatCompletionRequestUserMessageContent::Array(vec![
                                    ChatCompletionRequestUserMessageContentPart::Text(
                                        ChatCompletionRequestMessageContentPartText {
                                            text: prompt.prompt.to_owned(),
                                        },
                                    ),
                                    ChatCompletionRequestUserMessageContentPart::ImageUrl(
                                        ChatCompletionRequestMessageContentPartImage {
                                            image_url: ImageUrl {
                                                url: format!(
                                                    "data:image/jpeg;base64,{base64_image}"
                                                ),
                                                detail: Some(ImageDetail::High),
                                            },
                                        },
                                    ),
                                ]))
                                .build()
                                .unwrap()
                                .into(),
                        ])
                        .build()
                        .unwrap(),
                )
                .await
                .unwrap()
                .choices
                .remove(0)
                .message
                .content
                .unwrap()
                .trim()
                .to_owned()
        } else {
            let image = Image::from_base64(&base64_image);

            let request =
                GenerationRequest::new(prompt.config.image_ai_model.clone(), prompt.prompt)
                    .add_image(image)
                    .keep_alive(KeepAlive::Until {
                        time: 5,
                        unit: TimeUnit::Seconds,
                    })
                    .options(
                        ModelOptions::default()
                            .temperature(TEMPERATURE)
                            .top_k(6)
                            .top_p(0.4),
                    )
                    .system(&prompt.config.ai_description_system_prompt);

            let response = match send_request(request).await {
                Ok(r) => r,
                Err(e) => {
                    panic!("Failed to get response: {}", e);
                }
            };

            response.response.trim().to_owned()
        }
    })
}

async fn send_request(
    request: GenerationRequest<'_>,
) -> Result<GenerationResponse, Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    let response = ollama.generate(request).await?;
    Ok(response)
}
