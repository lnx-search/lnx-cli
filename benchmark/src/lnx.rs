use std::sync::Arc;
use std::time::Instant;

use serde_json::Value;

use crate::sampler::SamplerHandle;
use crate::shared::{Query, RequestClient, TargetUri};

pub(crate) async fn prep(address: &str, data: Value, index: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Clear the existing docs
    let _ = client
        .delete(format!("{}/indexes/{}/documents/clear", address, index))
        .send()
        .await?;

    let start = Instant::now();
    let _ = client
        .post(format!("{}/indexes/{}/documents", address, index))
        .json(&data)
        .send()
        .await?;

    let _ = client
        .post(format!("{}/indexes/{}/commit", address, index))
        .send()
        .await?;

    let delta = start.elapsed();
    info!("lnx took {:?} to process submitted documents", delta);

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

async fn search(client: RequestClient, uri: TargetUri, query: Query) -> anyhow::Result<u16> {
    let val = serde_json::json!({
        "query": {
            "fuzzy": {
                "ctx": query
            }
        },
    });

    let r = client
        .post(uri.as_ref())
        .json(&val)
        .send()
        .await?;

    Ok(r.status().as_u16())
}
