use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::RwLock, task::JoinHandle};

#[derive(Clone, Default)]
struct MockState {
    beststories: Endpoint,
    items: BTreeMap<u64, Endpoint>,
    item_delay_ms: BTreeMap<u64, u64>,
    newstories: Endpoint,
    recorded_cache_control: Vec<String>,
    list_delay_ms: u64,
    topstories: Endpoint,
}

#[derive(Clone)]
enum Endpoint {
    Json(Value),
    Status(StatusCode),
}

impl Default for Endpoint {
    fn default() -> Self {
        Self::Json(json!([]))
    }
}

pub struct MockHnServer {
    address: SocketAddr,
    handle: JoinHandle<()>,
    state: Arc<RwLock<MockState>>,
}

impl MockHnServer {
    pub async fn start() -> Self {
        let state = Arc::new(RwLock::new(MockState::default()));
        let app = Router::new()
            .route("/v0/topstories.json", get(discovery))
            .route("/v0/beststories.json", get(discovery))
            .route("/v0/newstories.json", get(discovery))
            .route("/v0/item/{*rest}", get(item))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock hn server");
        let address = listener.local_addr().expect("mock hn local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("run mock hn server");
        });
        Self {
            address,
            handle,
            state,
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub async fn set_lists(&self, top: Vec<u64>, best: Vec<u64>, new: Vec<u64>) {
        let mut state = self.state.write().await;
        state.topstories = Endpoint::Json(json!(top));
        state.beststories = Endpoint::Json(json!(best));
        state.newstories = Endpoint::Json(json!(new));
    }

    pub async fn fail_lists(&self, top: StatusCode, best: StatusCode, new: StatusCode) {
        let mut state = self.state.write().await;
        state.topstories = Endpoint::Status(top);
        state.beststories = Endpoint::Status(best);
        state.newstories = Endpoint::Status(new);
    }

    pub async fn set_item(&self, id: u64, body: Value) {
        self.state
            .write()
            .await
            .items
            .insert(id, Endpoint::Json(body));
    }

    pub async fn fail_item(&self, id: u64, status: StatusCode) {
        self.state
            .write()
            .await
            .items
            .insert(id, Endpoint::Status(status));
    }

    pub async fn recorded_cache_control_values(&self) -> Vec<String> {
        self.state.read().await.recorded_cache_control.clone()
    }

    pub async fn set_list_delay_ms(&self, delay_ms: u64) {
        self.state.write().await.list_delay_ms = delay_ms;
    }
}

impl Drop for MockHnServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn discovery(
    State(state): State<Arc<RwLock<MockState>>>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let (delay_ms, response) = {
        let state = state.read().await;
        let response = match uri.path() {
            "/v0/topstories.json" => state.topstories.clone(),
            "/v0/beststories.json" => state.beststories.clone(),
            _ => state.newstories.clone(),
        };
        (state.list_delay_ms, response)
    };
    record_cache_control(&state, &headers).await;
    if delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    };
    into_response(response)
}

async fn item(
    State(state): State<Arc<RwLock<MockState>>>,
    headers: axum::http::HeaderMap,
    Path(rest): Path<String>,
) -> impl IntoResponse {
    let id = rest
        .trim_end_matches(".json")
        .parse::<u64>()
        .unwrap_or_default();
    let (delay_ms, response) = {
        let state = state.read().await;
        let response = state
            .items
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Endpoint::Status(StatusCode::NOT_FOUND));
        let delay_ms = state.item_delay_ms.get(&id).copied().unwrap_or(0);
        (delay_ms, response)
    };
    record_cache_control(&state, &headers).await;
    if delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
    into_response(response)
}

async fn record_cache_control(state: &Arc<RwLock<MockState>>, headers: &axum::http::HeaderMap) {
    state.write().await.recorded_cache_control.push(
        headers
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string(),
    );
}

fn into_response(endpoint: Endpoint) -> impl IntoResponse {
    match endpoint {
        Endpoint::Json(body) => (StatusCode::OK, Json(body)).into_response(),
        Endpoint::Status(status) => status.into_response(),
    }
}

pub fn fixed_time(value: &str) -> hn_scored::time::Timestamp {
    hn_scored::time::Timestamp::parse(value).expect("valid fixed timestamp")
}

pub fn sample_item(id: u64, score: i64, descendants: u64) -> Value {
    json!({
        "id": id,
        "type": "story",
        "title": format!("Story {id}"),
        "url": format!("https://example.com/{id}"),
        "score": score,
        "descendants": descendants,
        "by": format!("user{id}"),
        "time": 1_700_000_000
    })
}
