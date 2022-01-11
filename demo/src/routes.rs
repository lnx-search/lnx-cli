use axum::extract;
use axum::response::{Html, IntoResponse, Json};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

pub(crate) static TARGET_URL: OnceCell<String> = OnceCell::new();

#[derive(Deserialize)]
pub(crate) struct SearchPayload {
    query: String,
    mode: String,
}


#[derive(Serialize, Deserialize)]
pub(crate) struct SearchResponse {
    data: serde_json::Value,
    status: u16,
}

pub(crate) async fn search(payload: extract::Json<SearchPayload>) -> impl IntoResponse {
    info!(
        "searching for query={:?} mode={:?}",
        &payload.query, &payload.mode
    );

    let target = TARGET_URL.get().unwrap();
    let client = reqwest::Client::new();

    let val = serde_json::json!({
        "query": {
            payload.mode.clone(): {
                "ctx": payload.query
            },
        },
    });

    let fut = client
        .post(target)
        .json(&val)
        .send();

    let r = match fut.await {
        Ok(r) => r,
        Err(e) => {
            return Json(serde_json::json!({
                "status": 500,
                "error": format!("{}", e.to_string()),
            }))
        },
    };

    let r: SearchResponse = match r.json().await {
        Ok(r) => r,
        Err(e) => {
            return Json(serde_json::json!({
                "status": 500,
                "error": format!("{}", e.to_string()),
            }))
        },
    };

    Json(serde_json::to_value(r).unwrap())
}

pub(crate) async fn index() -> impl IntoResponse {
    let html = include_str!("../static/index.html");

    Html(html)
}
