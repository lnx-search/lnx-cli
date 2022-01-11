use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::Duration;

use crate::sampler::SamplerHandle;
use crate::shared::{Query, RequestClient, TargetUri};

#[derive(Debug, Deserialize)]
struct EnqueueResponseData {
    #[serde(rename = "uid")]
    update_id: usize,

    #[serde(flatten)]
    _other: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct CheckData {
    status: String,

    #[serde(rename = "startedAt")]
    started: Option<chrono::DateTime<Utc>>,

    #[serde(rename = "finishedAt")]
    finished: Option<chrono::DateTime<Utc>>,

    #[serde(flatten)]
    _other: HashMap<String, Value>,
}

pub(crate) async fn prep(address: &str, data: Value, index: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Clear the existing docs
    let _ = client
        .delete(format!("{}/indexes/{}/documents", address, index))
        .send()
        .await?;

    let data: EnqueueResponseData = client
        .post(format!("{}/indexes/{}/documents", address, index))
        .json(&data)
        .send()
        .await?
        .json()
        .await?;

    let status = data.update_id;
    let delta;

    loop {
        let data: CheckData = client
            .get(format!("{}/tasks/{}", address, status))
            .send()
            .await?
            .json()
            .await?;

        if data.status == "succeeded" {
            delta = data.finished.unwrap() - data.started.unwrap();
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    info!(
        "MeiliSearch took {}.{}s to process submitted documents",
        delta.num_seconds(),
        delta.num_milliseconds() / 100i64
    );

    info!("waiting 30 secs");
    tokio::time::sleep(Duration::from_secs(30)).await;


    Ok(())
}

pub(crate) async fn bench_standard(
    address: Arc<String>,
    sample: SamplerHandle,
    terms: Vec<String>,
    index: String,
) -> anyhow::Result<()> {
    crate::shared::start_standard(
        address,
        sample,
        terms,
        &index,
        move |client, uri, query| async { search(client, uri, query).await },
    )
    .await
}

pub(crate) async fn bench_typing(
    address: Arc<String>,
    sample: SamplerHandle,
    terms: Vec<String>,
    index: String,
) -> anyhow::Result<()> {
    crate::shared::start_typing(
        address,
        sample,
        terms,
        &index,
        move |client, uri, query| async { search(client, uri, query).await },
    )
    .await
}

#[derive(Serialize)]
struct QueryPayload {
    q: String,
}

async fn search(client: RequestClient, uri: TargetUri, query: Query) -> anyhow::Result<u16> {
    let r = client
        .post(uri.as_ref())
        .json(&QueryPayload { q: query })
        .send()
        .await?;

    Ok(r.status().as_u16())
}
