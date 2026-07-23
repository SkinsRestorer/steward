use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use anyhow::{Context as _, Result};
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::warn;

use crate::download;

const REFRESH_INTERVAL: Duration = Duration::from_mins(20);
const MAX_RELEASE_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct LatestRelease {
    pub tag_name: String,
}

#[derive(Clone)]
pub struct ReleaseService {
    client: reqwest::Client,
    releases: Arc<RwLock<HashMap<&'static str, LatestRelease>>>,
    tasks: Arc<ReleaseTasks>,
}

#[derive(Default)]
struct ReleaseTasks {
    handles: StdMutex<HashMap<&'static str, tokio::task::JoinHandle<()>>>,
}

impl Drop for ReleaseTasks {
    fn drop(&mut self) {
        let handles = self
            .handles
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for handle in handles.values() {
            handle.abort();
        }
    }
}

impl ReleaseService {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            releases: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(ReleaseTasks::default()),
        }
    }

    pub fn start_refreshing(&self, url: &'static str) {
        let mut handles = self
            .tasks
            .handles
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if handles.contains_key(url) {
            return;
        }

        let client = self.client.clone();
        let releases = Arc::clone(&self.releases);
        let handle = tokio::spawn(async move {
            loop {
                if let Err(error) = refresh(&client, &releases, url).await {
                    warn!(%error, %url, "failed to refresh latest release");
                }
                tokio::time::sleep(REFRESH_INTERVAL).await;
            }
        });
        handles.insert(url, handle);
    }

    pub async fn get(&self, url: &'static str) -> Option<LatestRelease> {
        self.releases.read().await.get(url).cloned()
    }
}

async fn refresh(
    client: &reqwest::Client,
    releases: &RwLock<HashMap<&'static str, LatestRelease>>,
    url: &'static str,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to request latest release from {url}"))?
        .error_for_status()
        .with_context(|| format!("latest release request failed for {url}"))?;
    let body =
        download::read_limited_bytes(response, MAX_RELEASE_RESPONSE_BYTES, "release response")
            .await
            .with_context(|| format!("failed to read latest release from {url}"))?;
    let release = serde_json::from_slice::<LatestRelease>(&body)
        .with_context(|| format!("failed to decode latest release from {url}"))?;

    releases.write().await.insert(url, release);
    Ok(())
}
