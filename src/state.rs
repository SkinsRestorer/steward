use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;

use crate::{
    ai::AiService, config::BotDefinition, events::chatbot::ChatbotService, ocr::OcrService,
    releases::ReleaseService,
};

#[derive(Clone)]
pub struct SharedServices {
    pub ai: AiService,
    pub chatbot: ChatbotService,
    pub http: reqwest::Client,
    pub ocr: OcrService,
    pub releases: ReleaseService,
}

impl SharedServices {
    pub async fn load() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("steward/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        let ai = AiService::new(http.clone());
        let ocr = OcrService::load().await?;
        let releases = ReleaseService::new(http.clone());

        Ok(Self {
            ai,
            chatbot: ChatbotService::new(),
            http,
            ocr,
            releases,
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub bot: &'static BotDefinition,
    pub command_ids: Arc<RwLock<HashMap<String, serenity::CommandId>>>,
    pub services: Arc<SharedServices>,
}

impl AppState {
    pub fn new(bot: &'static BotDefinition, services: Arc<SharedServices>) -> Self {
        Self {
            bot,
            command_ids: Arc::new(RwLock::new(HashMap::new())),
            services,
        }
    }
}

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, AppState, Error>;
