use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use tokio::sync::Semaphore;

const EMBEDDED_DETECTION_MODEL: &[u8] = include_bytes!("../models/text-detection.rten");
const EMBEDDED_RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.rten");

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

    pub async fn recognize(&self, bytes: Vec<u8>) -> Result<String> {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .context("OCR semaphore closed")?;
        let engine = Arc::clone(&self.engine);

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let image = image::load_from_memory(&bytes)
                .context("failed to decode image for OCR")?
                .into_rgb8();
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

fn load_model(model_name: &str, embedded: &[u8]) -> Result<Model> {
    Model::load(embedded.to_vec())
        .map_err(|error| anyhow!("failed to load embedded {model_name}: {error}"))
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, anyhow};

    use super::{
        EMBEDDED_DETECTION_MODEL, EMBEDDED_RECOGNITION_MODEL, OcrEngine, OcrEngineParams,
        load_model,
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
}
