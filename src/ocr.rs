use std::{io::Cursor, sync::Arc};

use anyhow::{Context as _, Result, anyhow, ensure};
use image::ImageDecoder as _;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const EMBEDDED_DETECTION_MODEL: &[u8] = include_bytes!("../models/text-detection.rten");
const EMBEDDED_RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.rten");
const MAX_DECODE_ALLOCATION_BYTES: u64 = 128 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 4_096;
const MAX_IMAGE_PIXELS: u64 = 12 * 1024 * 1024;

#[derive(Clone)]
pub struct OcrService {
    engine: Arc<OcrEngine>,
    permits: Arc<Semaphore>,
}

impl OcrService {
    pub async fn load() -> Result<Self> {
        let engine = tokio::task::spawn_blocking(move || {
            let detection_model = load_model("text detection", EMBEDDED_DETECTION_MODEL)?;
            let recognition_model = load_model("text recognition", EMBEDDED_RECOGNITION_MODEL)?;

            OcrEngine::new(OcrEngineParams {
                detection_model: Some(detection_model),
                recognition_model: Some(recognition_model),
                ..Default::default()
            })
            .map_err(|error| anyhow!("failed to initialize OCR engine: {error}"))
        })
        .await
        .context("OCR initialization task failed")??;

        Ok(Self {
            engine: Arc::new(engine),
            permits: Arc::new(Semaphore::new(2)),
        })
    }

    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit> {
        Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .context("OCR semaphore closed")
    }

    pub async fn recognize(&self, bytes: Vec<u8>, permit: OwnedSemaphorePermit) -> Result<String> {
        let engine = Arc::clone(&self.engine);

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let image = decode_image(&bytes)?;
            let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
                .map_err(|error| anyhow!("failed to prepare OCR image source: {error}"))?;
            let input = engine
                .prepare_input(source)
                .map_err(|error| anyhow!("failed to prepare OCR input: {error}"))?;
            engine
                .get_text(&input)
                .map_err(|error| anyhow!("OCR recognition failed: {error}"))
        })
        .await
        .context("OCR worker task failed")?
    }
}

fn decode_image(bytes: &[u8]) -> Result<image::RgbImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION_BYTES);

    let mut reader = image::ImageReader::new(Cursor::new(bytes));
    reader.limits(limits);
    let reader = reader
        .with_guessed_format()
        .context("failed to determine OCR image format")?;
    let decoder = reader
        .into_decoder()
        .context("failed to create OCR image decoder")?;
    let (width, height) = decoder.dimensions();
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    ensure!(
        pixels <= MAX_IMAGE_PIXELS,
        "OCR image contains {pixels} pixels, exceeding the {MAX_IMAGE_PIXELS} pixel limit"
    );

    image::DynamicImage::from_decoder(decoder)
        .context("failed to decode image for OCR")
        .map(image::DynamicImage::into_rgb8)
}

fn load_model(model_name: &str, embedded: &'static [u8]) -> Result<Model> {
    Model::load_static_slice(embedded)
        .map_err(|error| anyhow!("failed to load embedded {model_name}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use anyhow::{Result, anyhow, ensure};

    use super::{
        EMBEDDED_DETECTION_MODEL, EMBEDDED_RECOGNITION_MODEL, MAX_IMAGE_DIMENSION, OcrEngine,
        OcrEngineParams, decode_image, load_model,
    };

    #[test]
    fn initializes_ocr_from_embedded_models() -> Result<()> {
        let detection_model = load_model("text detection", EMBEDDED_DETECTION_MODEL)?;
        let recognition_model = load_model("text recognition", EMBEDDED_RECOGNITION_MODEL)?;

        OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection_model),
            recognition_model: Some(recognition_model),
            ..Default::default()
        })
        .map_err(|error| anyhow!("failed to initialize OCR engine: {error}"))?;
        Ok(())
    }

    #[test]
    fn rejects_images_beyond_the_dimension_limit() -> Result<()> {
        let image = image::DynamicImage::new_rgb8(MAX_IMAGE_DIMENSION.saturating_add(1), 1);
        let mut encoded = Cursor::new(Vec::new());
        image.write_to(&mut encoded, image::ImageFormat::Png)?;

        ensure!(decode_image(&encoded.into_inner()).is_err());
        Ok(())
    }
}
