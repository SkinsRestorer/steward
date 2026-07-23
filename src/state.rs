use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use tokio::sync::Mutex;

use crate::{
    ai::AiService,
    bots,
    config::BotDefinition,
    events::{chatbot::ChatbotService, logging::LoggingService},
    ocr::OcrService,
    patterns::PatternService,
    releases::ReleaseService,
};

pub struct SharedServices {
    pub ai: AiService,
    pub chatbot: ChatbotService,
    pub http: reqwest::Client,
    pub logging: LoggingService,
    pub ocr: OcrService,
    pub patterns: PatternService,
    pub releases: ReleaseService,
    pub thread_updates: Mutex<()>,
}

impl SharedServices {
    pub async fn load() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("steward/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        let patterns = PatternService::compile(bots::ALL)?;
        let ai = AiService::new(http.clone())?;
        let ocr = OcrService::load().await?;
        let logging = LoggingService::start(bots::ALL).await?;
        let releases = ReleaseService::new(http.clone());

        Ok(Self {
            ai,
            chatbot: ChatbotService::new(),
            http,
            logging,
            ocr,
            patterns,
            releases,
            thread_updates: Mutex::new(()),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub bot: &'static BotDefinition,
    pub command_ids: Arc<OnceLock<HashMap<String, serenity::CommandId>>>,
    pub services: Arc<SharedServices>,
}

impl AppState {
    pub fn new(bot: &'static BotDefinition, services: Arc<SharedServices>) -> Self {
        Self {
            bot,
            command_ids: Arc::new(OnceLock::new()),
            services,
        }
    }
}

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, AppState, Error>;
