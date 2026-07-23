use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::warn;

const REFRESH_INTERVAL: Duration = Duration::from_mins(20);

#[derive(Clone, Debug, Deserialize)]
pub struct LatestRelease {
    pub tag_name: String,
}

#[derive(Clone)]
pub struct ReleaseService {
    client: reqwest::Client,
    releases: Arc<RwLock<HashMap<&'static str, LatestRelease>>>,
}

impl ReleaseService {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            releases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn start_refreshing(&self, url: &'static str) {
        let service = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(error) = service.refresh(url).await {
                    warn!(%error, %url, "failed to refresh latest release");
                }
                tokio::time::sleep(REFRESH_INTERVAL).await;
            }
        });
    }

    pub async fn get(&self, url: &'static str) -> Option<LatestRelease> {
        self.releases.read().await.get(url).cloned()
    }

    async fn refresh(&self, url: &'static str) -> Result<()> {
        let release = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to request latest release from {url}"))?
            .error_for_status()
            .with_context(|| format!("latest release request failed for {url}"))?
            .json::<LatestRelease>()
            .await
            .with_context(|| format!("failed to decode latest release from {url}"))?;

        self.releases.write().await.insert(url, release);
        Ok(())
    }
}
