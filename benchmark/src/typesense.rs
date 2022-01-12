use std::str::FromStr;
use std::sync::Arc;
use anyhow::anyhow;

use reqwest::header::HeaderValue;
use reqwest::{StatusCode, Url};
use serde::Serialize;
use serde_json::Value;
use tokio::time::Instant;

use crate::sampler::SamplerHandle;
use crate::shared::{Query, RequestClient, TargetUri};


pub(crate) async fn prep(address: &str, data: Value, index: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let start = Instant::now();
    if let Some(arr) = data.as_array() {
        for row in arr {
            let r = client
                .post(format!("{}/collections/{}/documents", address, index))
                .header("X-TYPESENSE-API-KEY", HeaderValue::from_static("bench-key"))
                .json(row)
                .send()
                .await?;

            if r.status() != StatusCode::CREATED {
                return Err(anyhow!("got unexpected response code {} data: {}", r.status(), r.text().await?))
            }
        }
    }

    info!(
        "TypeSense took {:?} to process submitted documents", start.elapsed()
    );

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
    query_by: &'static str,
}

async fn search(client: RequestClient, uri: TargetUri, query: Query) -> anyhow::Result<u16> {
    let uri = uri.replace("indexes", "collections")
        .replace("/search", "/documents/search");

    let ref_uri = Url::from_str(&uri)?;

    let r = client
        .get(ref_uri)
        .header("X-TYPESENSE-API-KEY", HeaderValue::from_static("bench-key"))
        .query(&QueryPayload { q: query, query_by: "title,overview" })
        .send()
        .await?;

    let status = r.status().clone();
    Ok(status.as_u16())
}
